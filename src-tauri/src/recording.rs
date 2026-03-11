use anyhow::{anyhow, Result};
use cpal::traits::StreamTrait;
use std::sync::mpsc;
use std::time::Instant;
use tauri::{AppHandle, Manager};

use crate::audio::{enumerate_sources, resolve_selected_device};
use crate::constants::*;
use crate::live_preview;
use crate::models::*;
use crate::overlay::{
    clear_overlay_session_state, default_overlay_levels, update_indicator_window,
};
use crate::permissions::{
    ensure_microphone_access, microphone_access_error_message, microphone_access_status_message,
    MicrophoneAccess,
};
use crate::state::*;
use crate::storage::*;
use crate::transcript::note_preview_diagnostic;
use crate::transcription;
use crate::{audio, parakeet, platform};

fn report_runtime_error(
    app: &AppHandle,
    shared: &SharedState,
    status_message: &str,
    error_message: String,
) {
    {
        let mut core = shared.lock();
        core.phase = AppPhase::Error;
        core.status_message = status_message.to_string();
        core.error_message = Some(error_message);
        core.recording_started_at = None;
        clear_overlay_session_state(&mut core);
    }

    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
}

fn ensure_recording_access(app: &AppHandle, shared: &SharedState) -> Result<()> {
    match ensure_microphone_access() {
        Ok(MicrophoneAccess::Authorized) => Ok(()),
        Ok(state) => {
            let error_message = microphone_access_error_message(state).to_string();
            report_runtime_error(
                app,
                shared,
                microphone_access_status_message(state),
                error_message.clone(),
            );
            Err(anyhow!(error_message))
        }
        Err(error) => {
            report_runtime_error(
                app,
                shared,
                "Microphone access check failed",
                error.to_string(),
            );
            Err(error)
        }
    }
}

pub(crate) fn begin_recording(
    app: &AppHandle,
    shared: &SharedState,
    mode: RecordingMode,
) -> Result<()> {
    let anchor = platform::detect_caret_anchor();
    let (selected_source_id, current_phase) = {
        let core = shared.lock();
        (core.settings.selected_source_id.clone(), core.phase.clone())
    };

    if !matches!(current_phase, AppPhase::Idle | AppPhase::Error) {
        return Ok(());
    }

    ensure_recording_access(app, shared)?;

    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    let preview_generation = preview_control.next_generation();
    let (response_tx, response_rx) = mpsc::channel();
    recorder
        .sender
        .send(RecorderRequest::Start {
            selected_source_id,
            mode: mode.clone(),
            anchor,
            response: response_tx,
        })
        .map_err(|_| anyhow!("Recording worker is unavailable"))?;

    let started = match response_rx
        .recv()
        .map_err(|_| anyhow!("Recording worker did not respond"))?
        .map_err(|error| anyhow!(error))
    {
        Ok(started) => started,
        Err(error) => {
            report_runtime_error(app, shared, "Recording couldn't start", error.to_string());
            return Err(error);
        }
    };

    let should_spawn_preview = {
        let mut core = shared.lock();
        core.phase = AppPhase::Recording;
        core.status_message = format!("Recording from {}", started.source_name);
        core.error_message = None;
        core.recording_started_at = Some(Instant::now());
        core.overlay.visible = true;
        core.overlay.title = "Listening".to_string();
        core.overlay.detail = String::new();
        core.overlay.levels = default_overlay_levels();
        core.overlay.elapsed_ms = 0;
        core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
        core.overlay.anchor = anchor;
        core.settings.selected_source_id = Some(started.source_id);
        core.sources = enumerate_sources();
        core.settings.show_live_transcription
    };

    let _ = save_persisted_state(app, shared);
    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
    if should_spawn_preview {
        note_preview_diagnostic(
            app,
            shared,
            "starting",
            "Preparing live preview",
            "Selecting preview backend",
        );
        live_preview::spawn_live_preview(
            app.clone(),
            shared.clone(),
            transcriber.inner().clone(),
            preview_control.inner().clone(),
            preview_generation,
            started.preview_buffer.clone(),
            started.preview_sample_rate,
        );
    }
    live_preview::spawn_live_meter(
        app.clone(),
        shared.clone(),
        preview_control.inner().clone(),
        preview_generation,
        started.preview_buffer,
        started.preview_sample_rate,
    );
    Ok(())
}

pub(crate) fn finalize_recording(session: RecordingSession) -> Result<Option<CompletedRecording>> {
    drop(session.stream);

    let samples = {
        let samples = session.buffer.lock().expect("recording buffer poisoned");
        samples.clone()
    };

    let duration_ms = session.started_at.elapsed().as_millis() as u64;
    if duration_ms < HOLD_MIN_DURATION_MS || samples.len() < session.channels * 256 {
        return Ok(None);
    }

    Ok(Some(CompletedRecording {
        samples: parakeet::resample_to_16khz(&samples, session.sample_rate),
        captured_samples: samples,
        captured_sample_rate: session.sample_rate,
        captured_channels: session.channels as u16,
        duration_ms,
        source_name: session.source_name,
        mode: session.mode,
        anchor: session.anchor,
    }))
}

pub(crate) fn stop_recording(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    let transcription_generation = preview_control.next_generation();
    let (response_tx, response_rx) = mpsc::channel();
    recorder
        .sender
        .send(RecorderRequest::Stop {
            response: response_tx,
        })
        .map_err(|_| anyhow!("Recording worker is unavailable"))?;

    let completed = response_rx
        .recv()
        .map_err(|_| anyhow!("Recording worker did not respond"))?
        .map_err(|error| anyhow!(error))?;
    let Some(completed) = completed else {
        {
            let mut core = shared.lock();
            core.phase = AppPhase::Idle;
            core.status_message = "Capture was too short".to_string();
            core.error_message = None;
            clear_overlay_session_state(&mut core);
        }
        update_indicator_window(app, shared);
        emit_snapshot(app, shared);
        return Ok(());
    };

    {
        let mut core = shared.lock();
        core.phase = AppPhase::Transcribing;
        core.status_message = "Running local Rust transcription".to_string();
        core.error_message = None;
        core.recording_started_at = None;
        core.overlay.visible = true;
        core.overlay.title = "Transcribing".to_string();
        core.overlay.detail.clear();
        core.overlay.elapsed_ms = completed.duration_ms;
        core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
        core.overlay.anchor = completed.anchor;
    }

    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
    transcription::complete_transcription(
        app.clone(),
        shared.clone(),
        transcriber.inner().clone(),
        preview_control.inner().clone(),
        transcription_generation,
        completed,
    );
    Ok(())
}

pub(crate) fn cancel_current_operation(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let preview_control = app.state::<PreviewControl>();
    let phase = {
        let core = shared.lock();
        core.phase.clone()
    };

    match phase {
        AppPhase::Recording => {
            preview_control.next_generation();
            let recorder = app.state::<RecorderHandle>();
            let (response_tx, response_rx) = mpsc::channel();
            recorder
                .sender
                .send(RecorderRequest::Stop {
                    response: response_tx,
                })
                .map_err(|_| anyhow!("Recording worker is unavailable"))?;
            let _ = response_rx
                .recv()
                .map_err(|_| anyhow!("Recording worker did not respond"))?;

            {
                let mut core = shared.lock();
                core.phase = AppPhase::Idle;
                core.status_message = "Recording cancelled".to_string();
                core.error_message = None;
                clear_overlay_session_state(&mut core);
            }

            update_indicator_window(app, shared);
            emit_snapshot(app, shared);
            Ok(())
        }
        AppPhase::Transcribing => {
            preview_control.next_generation();
            {
                let mut core = shared.lock();
                core.phase = AppPhase::Idle;
                core.status_message = "Transcription cancelled".to_string();
                core.error_message = None;
                clear_overlay_session_state(&mut core);
            }

            update_indicator_window(app, shared);
            emit_snapshot(app, shared);
            Ok(())
        }
        AppPhase::Idle | AppPhase::Error => Ok(()),
    }
}

pub(crate) fn refresh_sources(app: &AppHandle, shared: &SharedState) {
    {
        let mut core = shared.lock();
        core.sources = enumerate_sources();
        core.model_status = current_model_status(app, &core.settings);
        core.parakeet_model_status = built_in_parakeet_status(app);
    }
    emit_snapshot(app, shared);
}

pub(crate) fn refresh_sources_after_permission_check(
    app: &AppHandle,
    shared: &SharedState,
) -> Result<()> {
    ensure_recording_access(app, shared)?;
    refresh_sources(app, shared);
    Ok(())
}

pub(crate) fn spawn_recorder_thread() -> RecorderHandle {
    let (sender, receiver) = mpsc::channel::<RecorderRequest>();

    std::thread::spawn(move || {
        let mut current: Option<RecordingSession> = None;

        while let Ok(message) = receiver.recv() {
            match message {
                RecorderRequest::Start {
                    selected_source_id,
                    mode,
                    anchor,
                    response,
                } => {
                    let result = (|| -> Result<StartRecordingResponse> {
                        if current.is_some() {
                            return Err(anyhow!("Recording is already active"));
                        }

                        let (device, source, supported_config) =
                            resolve_selected_device(selected_source_id.as_deref())?;
                        let config = supported_config.config();
                        let channels = config.channels as usize;

                        let (stream, destination) =
                            audio::build_input_stream(&device, &supported_config)?;

                        stream.play()?;
                        let preview_buffer = destination.clone();
                        current = Some(RecordingSession {
                            stream,
                            buffer: destination,
                            sample_rate: config.sample_rate.0,
                            channels,
                            started_at: Instant::now(),
                            mode,
                            source_name: source.name.clone(),
                            anchor,
                        });

                        Ok(StartRecordingResponse {
                            source_id: source.id,
                            source_name: source.name,
                            preview_buffer,
                            preview_sample_rate: config.sample_rate.0,
                        })
                    })()
                    .map_err(|error| error.to_string());

                    let _ = response.send(result);
                }
                RecorderRequest::Stop { response } => {
                    let result = match current.take() {
                        Some(session) => {
                            finalize_recording(session).map_err(|error| error.to_string())
                        }
                        None => Ok(None),
                    };
                    let _ = response.send(result);
                }
            }
        }
    });

    RecorderHandle { sender }
}

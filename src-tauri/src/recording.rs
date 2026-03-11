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
use crate::state::*;
use crate::storage::*;
use crate::transcript::{note_capture_diagnostic, note_preview_diagnostic};
use crate::transcription;
use crate::{audio, parakeet, platform};

fn macos_capture_hint(message: &str) -> String {
    if cfg!(target_os = "macos") {
        format!(
            "{message}. If microphone capture never starts, check System Settings > Privacy & Security > Microphone for Warble."
        )
    } else {
        message.to_string()
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

    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    let preview_generation = preview_control.next_generation();
    let (response_tx, response_rx) = mpsc::channel();
    let started = match (|| -> Result<StartRecordingResponse> {
        recorder
            .sender
            .send(RecorderRequest::Start {
                selected_source_id,
                mode: mode.clone(),
                anchor,
                response: response_tx,
            })
            .map_err(|_| anyhow!("Recording worker is unavailable"))?;

        response_rx
            .recv()
            .map_err(|_| anyhow!("Recording worker did not respond"))?
            .map_err(|error| anyhow!(error))
    })() {
        Ok(started) => started,
        Err(error) => {
            let detail = macos_capture_hint(&error.to_string());
            note_capture_diagnostic(app, shared, "Capture start failed", detail.clone());
            return Err(anyhow!(detail));
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
        core.capture_diagnostics.source_name = started.source_name.clone();
        core.capture_diagnostics.sample_rate = started.preview_sample_rate;
        core.capture_diagnostics.channels = started.preview_channels;
        core.capture_diagnostics.last_buffered_samples = 0;
        core.settings.show_live_transcription
    };

    let _ = save_persisted_state(app, shared);
    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
    note_capture_diagnostic(
        app,
        shared,
        "Recording started",
        format!(
            "{} at {} Hz · {} ch",
            started.source_name, started.preview_sample_rate, started.preview_channels
        ),
    );
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

pub(crate) fn finalize_recording(session: RecordingSession) -> Result<CompletedRecording> {
    drop(session.stream);

    let samples = {
        let samples = session.buffer.lock().expect("recording buffer poisoned");
        samples.clone()
    };
    let stream_errors = {
        let errors = session
            .stream_errors
            .lock()
            .expect("stream error buffer poisoned");
        errors.clone()
    };

    let duration_ms = session.started_at.elapsed().as_millis() as u64;
    let captured_sample_count = samples.len();
    let should_transcribe =
        duration_ms >= HOLD_MIN_DURATION_MS && captured_sample_count >= session.channels * 256;

    Ok(CompletedRecording {
        samples: if should_transcribe {
            parakeet::resample_to_16khz(&samples, session.sample_rate)
        } else {
            Vec::new()
        },
        captured_samples: samples,
        captured_sample_rate: session.sample_rate,
        captured_channels: session.channels as u16,
        captured_sample_count,
        stream_errors,
        should_transcribe,
        duration_ms,
        source_name: session.source_name,
        mode: session.mode,
        anchor: session.anchor,
    })
}

pub(crate) fn stop_recording(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    let transcription_generation = preview_control.next_generation();
    let (response_tx, response_rx) = mpsc::channel();
    let completed = match (|| -> Result<CompletedRecording> {
        recorder
            .sender
            .send(RecorderRequest::Stop {
                response: response_tx,
            })
            .map_err(|_| anyhow!("Recording worker is unavailable"))?;

        response_rx
            .recv()
            .map_err(|_| anyhow!("Recording worker did not respond"))?
            .map_err(|error| anyhow!(error))
    })() {
        Ok(completed) => completed,
        Err(error) => {
            let detail = macos_capture_hint(&error.to_string());
            note_capture_diagnostic(app, shared, "Capture stop failed", detail.clone());
            return Err(anyhow!(detail));
        }
    };

    {
        let mut core = shared.lock();
        core.capture_diagnostics.source_name = completed.source_name.clone();
        core.capture_diagnostics.sample_rate = completed.captured_sample_rate;
        core.capture_diagnostics.channels = completed.captured_channels;
        core.capture_diagnostics.last_buffered_samples = completed.captured_sample_count;
    }

    for stream_error in &completed.stream_errors {
        note_capture_diagnostic(app, shared, "Microphone stream error", stream_error.clone());
    }

    if !completed.should_transcribe {
        {
            let mut core = shared.lock();
            core.phase = AppPhase::Idle;
            core.status_message = "Capture was too short".to_string();
            core.error_message = None;
            clear_overlay_session_state(&mut core);
        }
        note_capture_diagnostic(
            app,
            shared,
            "Capture too short",
            format!(
                "{} buffered samples over {} ms",
                completed.captured_sample_count, completed.duration_ms
            ),
        );
        update_indicator_window(app, shared);
        emit_snapshot(app, shared);
        return Ok(());
    }

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

    note_capture_diagnostic(
        app,
        shared,
        "Transcribing microphone capture",
        format!(
            "{} buffered samples over {} ms",
            completed.captured_sample_count, completed.duration_ms
        ),
    );
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

            note_capture_diagnostic(app, shared, "Recording cancelled", "Stopped before transcription");
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

            note_capture_diagnostic(app, shared, "Transcription cancelled", "Stopped current microphone job");
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

                        let (stream, destination, stream_errors) =
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
                            stream_errors,
                        });

                        Ok(StartRecordingResponse {
                            source_id: source.id,
                            source_name: source.name,
                            preview_buffer,
                            preview_sample_rate: config.sample_rate.0,
                            preview_channels: config.channels,
                        })
                    })()
                    .map_err(|error| error.to_string());

                    let _ = response.send(result);
                }
                RecorderRequest::Stop { response } => {
                    let result = match current.take() {
                        Some(session) => finalize_recording(session).map_err(|error| error.to_string()),
                        None => Err("Recording session was not active".to_string()),
                    };
                    let _ = response.send(result);
                }
            }
        }
    });

    RecorderHandle { sender }
}

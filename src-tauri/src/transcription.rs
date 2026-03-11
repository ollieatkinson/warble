use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::constants::*;
use crate::models::*;
use crate::overlay::{
    clear_overlay_session_state, default_overlay_levels, update_indicator_window,
};
use crate::state::*;
use crate::storage::*;
use crate::transcript::{cleanup_transcript_text, transcription_cancelled};
use crate::{media, parakeet, platform};

pub(crate) struct TranscriptionOutput {
    pub(crate) text: String,
    pub(crate) inference_provider: InferenceProvider,
    pub(crate) model_name: String,
}

pub(crate) fn transcribe_audio(
    app: &AppHandle,
    transcriber: &TranscriberHandle,
    settings: &Settings,
    audio: &[f32],
) -> Result<TranscriptionOutput> {
    let selected_key = selected_model_cache_key(settings);
    let mut guard = transcriber.lock();
    if guard.selected_key.as_ref() != Some(&selected_key) {
        let engine = match settings.selected_model_kind {
            TranscriptionModelKind::Parakeet => {
                let model = if let Some(path) = resolved_selected_model_path(settings) {
                    let path = PathBuf::from(path);
                    if parakeet::model_ready_in_dir(&path) {
                        parakeet::ParakeetTdt::load_from_dir(&path)?
                    } else {
                        parakeet::ParakeetTdt::load(&path)?
                    }
                } else {
                    let model_root = model_root_dir(app)?;
                    parakeet::ParakeetTdt::load(&model_root)?
                };
                TranscriberEngine::Parakeet(model)
            }
            TranscriptionModelKind::ParakeetCtc => {
                let model_path = resolved_selected_model_path(settings)
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow!("Parakeet CTC model path is not configured"))?;
                let model = if parakeet::ctc_model_ready_in_dir(&model_path) {
                    parakeet::ParakeetCtc::load_from_dir(&model_path)?
                } else {
                    parakeet::ParakeetCtc::load(&model_path)?
                };
                TranscriberEngine::ParakeetCtc(model)
            }
        };

        guard.engine = Some(engine);
        guard.selected_key = Some(selected_key);
    }

    let model_name = selected_model_display_name(settings);
    match guard.engine.as_mut().expect("transcriber initialized") {
        TranscriberEngine::Parakeet(model) => Ok(TranscriptionOutput {
            text: model.transcribe_audio(audio)?,
            inference_provider: model.provider(),
            model_name,
        }),
        TranscriberEngine::ParakeetCtc(model) => Ok(TranscriptionOutput {
            text: model.transcribe_audio(audio)?,
            inference_provider: model.provider(),
            model_name,
        }),
    }
}

pub(crate) fn transcribe_audio_segments(
    app: &AppHandle,
    shared: Option<&SharedState>,
    preview_control: Option<(&PreviewControl, u64)>,
    transcriber: &TranscriberHandle,
    settings: &Settings,
    audio: &[f32],
    progress_label: &str,
) -> Result<Option<TranscriptionOutput>> {
    if transcription_cancelled(shared, preview_control) {
        return Ok(None);
    }

    let limit_ms = selected_model_audio_limit_ms(settings);
    let preferred_chunk_ms = limit_ms
        .map(|value| value.saturating_sub(30_000).max(60_000))
        .unwrap_or(0);
    let chunk_samples = if preferred_chunk_ms > 0 {
        ((preferred_chunk_ms as f64 / 1_000.0) * parakeet::SAMPLE_RATE as f64) as usize
    } else {
        0
    };

    if chunk_samples == 0 || audio.len() <= chunk_samples {
        let output = transcribe_audio(app, transcriber, settings, audio)?;
        if transcription_cancelled(shared, preview_control) {
            return Ok(None);
        }
        return Ok(Some(output));
    }

    let chunks = audio.chunks(chunk_samples).collect::<Vec<_>>();
    let mut combined = String::new();
    let mut provider = InferenceProvider::Cpu;
    let mut model_name = selected_model_display_name(settings);

    for (index, chunk) in chunks.iter().enumerate() {
        if transcription_cancelled(shared, preview_control) {
            return Ok(None);
        }

        if let Some(shared) = shared {
            {
                let mut core = shared.lock();
                core.status_message = format!("{progress_label} ({}/{})", index + 1, chunks.len());
                core.overlay.visible = true;
                core.overlay.title = progress_label.to_string();
                core.overlay.detail = format!("Chunk {} of {}", index + 1, chunks.len());
            }
            update_indicator_window(app, shared);
            emit_snapshot(app, shared);
        }

        let output = transcribe_audio(app, transcriber, settings, chunk)?;
        if transcription_cancelled(shared, preview_control) {
            return Ok(None);
        }
        provider = output.inference_provider;
        model_name = output.model_name;

        let trimmed = output.text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str(trimmed);
    }

    Ok(Some(TranscriptionOutput {
        text: combined,
        inference_provider: provider,
        model_name,
    }))
}

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "unknown panic".to_string(),
        },
    }
}

pub(crate) fn complete_transcription(
    app: AppHandle,
    shared: SharedState,
    transcriber: TranscriberHandle,
    preview_control: PreviewControl,
    generation: u64,
    completed: CompletedRecording,
) {
    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if preview_control.current_generation() != generation {
                return;
            }

            let settings = {
                let core = shared.lock();
                core.settings.clone()
            };
            let result = transcribe_audio_segments(
                &app,
                Some(&shared),
                Some((&preview_control, generation)),
                &transcriber,
                &settings,
                &completed.samples,
                "Transcribing",
            );

            if preview_control.current_generation() != generation {
                return;
            }

            match result {
                Ok(Some(output)) => {
                    let text = output.text;
                    let text = cleanup_transcript_text(
                        text.trim(),
                        settings.cleanup_enabled,
                        &settings.cleanup_terms,
                    );
                    if text.is_empty() {
                        let mut core = shared.lock();
                        core.phase = AppPhase::Idle;
                        core.status_message = "Nothing intelligible was detected".to_string();
                        core.error_message = None;
                        core.model_status = current_model_status(&app, &core.settings);
                        core.parakeet_model_status = built_in_parakeet_status(&app);
                        clear_overlay_session_state(&mut core);
                        drop(core);
                        update_indicator_window(&app, &shared);
                        emit_snapshot(&app, &shared);
                        return;
                    }

                    let paste_result = if settings.auto_paste {
                        Some(platform::paste_text(&text))
                    } else {
                        None
                    };
                    let paste_outcome = paste_result
                        .as_ref()
                        .and_then(|result| result.as_ref().ok())
                        .copied();
                    let paste_error = paste_result
                        .as_ref()
                        .and_then(|result| result.as_ref().err())
                        .map(ToString::to_string);
                    let pasted = matches!(paste_outcome, Some(platform::PasteOutcome::ActiveApp));
                    let item_id = Uuid::new_v4().to_string();
                    let audio_path = save_history_audio(
                        &app,
                        &item_id,
                        &completed.captured_samples,
                        completed.captured_sample_rate,
                    )
                    .ok();

                    let mut core = shared.lock();
                    core.history.insert(
                        0,
                        HistoryItem {
                            id: item_id,
                            text: text.clone(),
                            created_at: Utc::now().to_rfc3339(),
                            source_name: completed.source_name.clone(),
                            mode: completed.mode.clone(),
                            duration_ms: completed.duration_ms,
                            pasted,
                            audio_path,
                            capture: HistoryCaptureDetails {
                                source_kind: CaptureSourceKind::Microphone,
                                model_id: settings.selected_model_id.clone(),
                                model_name: output.model_name,
                                inference_provider: output.inference_provider,
                                input_sample_rate: completed.captured_sample_rate,
                                input_channels: completed.captured_channels,
                                transcription_sample_rate: parakeet::SAMPLE_RATE,
                            },
                        },
                    );
                    let keep_len = HISTORY_LIMIT.min(core.history.len());
                    let removed_items = core.history.split_off(keep_len);
                    for item in &removed_items {
                        remove_history_audio_file(item);
                    }
                    core.phase = AppPhase::Idle;
                    core.status_message = match paste_outcome {
                        Some(platform::PasteOutcome::ActiveApp) => {
                            "Transcribed in Rust and pasted".to_string()
                        }
                        Some(platform::PasteOutcome::ClipboardOnly) => {
                            "Transcribed in Rust and copied to clipboard".to_string()
                        }
                        None => "Transcribed in Rust".to_string(),
                    };
                    core.error_message = paste_error;
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                    drop(core);

                    let _ = save_persisted_state(&app, &shared);
                }
                Ok(None) => {}
                Err(error) => {
                    let mut core = shared.lock();
                    core.phase = AppPhase::Error;
                    core.status_message = "Transcription failed".to_string();
                    core.error_message = Some(error.to_string());
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                }
            }

            update_indicator_window(&app, &shared);
            emit_snapshot(&app, &shared);
        }));

        if let Err(error) = outcome {
            let mut core = shared.lock();
            core.phase = AppPhase::Error;
            core.status_message = "Transcription worker failed".to_string();
            core.error_message = Some(format!(
                "The transcription worker crashed unexpectedly: {}",
                panic_payload_message(error)
            ));
            core.model_status = current_model_status(&app, &core.settings);
            core.parakeet_model_status = built_in_parakeet_status(&app);
            clear_overlay_session_state(&mut core);
            drop(core);
            update_indicator_window(&app, &shared);
            emit_snapshot(&app, &shared);
        }
    });
}

pub(crate) fn transcribe_media_file(
    app: AppHandle,
    shared: SharedState,
    transcriber: TranscriberHandle,
    path: String,
) -> Result<()> {
    let preview_control = app.state::<PreviewControl>();
    let transcription_generation = preview_control.next_generation();
    let preview_control = preview_control.inner().clone();
    {
        let core = shared.lock();
        if !matches!(core.phase, AppPhase::Idle | AppPhase::Error) {
            bail!("Finish the current transcription before importing a file");
        }
    }

    let file_path = media::canonical_media_path(&path)?;
    let file_label = media::file_path_label(&file_path);

    {
        let mut core = shared.lock();
        core.phase = AppPhase::Transcribing;
        core.status_message = format!("Decoding {file_label}");
        core.error_message = None;
        core.recording_started_at = None;
        core.overlay.visible = true;
        core.overlay.title = "Transcribing file".to_string();
        core.overlay.detail = file_label.clone();
        core.overlay.levels = default_overlay_levels();
        core.overlay.elapsed_ms = 0;
        core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
        core.overlay.anchor = None;
    }
    update_indicator_window(&app, &shared);
    emit_snapshot(&app, &shared);

    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let decoded = match media::decode_media_file(&file_path) {
                Ok(decoded) => decoded,
                Err(error) => {
                    let mut core = shared.lock();
                    core.phase = AppPhase::Error;
                    core.status_message = "File transcription failed".to_string();
                    core.error_message = Some(error.to_string());
                    clear_overlay_session_state(&mut core);
                    drop(core);
                    update_indicator_window(&app, &shared);
                    emit_snapshot(&app, &shared);
                    return;
                }
            };

            if preview_control.current_generation() != transcription_generation {
                return;
            }

            let input_sample_rate = decoded.sample_rate;
            let input_channels = decoded.channels;
            let samples_16khz = if decoded.sample_rate == parakeet::SAMPLE_RATE {
                decoded.samples.clone()
            } else {
                parakeet::resample_to_16khz(&decoded.samples, decoded.sample_rate)
            };

            {
                let mut core = shared.lock();
                core.status_message = format!("Transcribing {}", decoded.display_name);
                core.overlay.title = "Transcribing file".to_string();
                core.overlay.detail = decoded.display_name.clone();
                core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
            }
            update_indicator_window(&app, &shared);
            emit_snapshot(&app, &shared);

            let settings = {
                let core = shared.lock();
                core.settings.clone()
            };

            let result = transcribe_audio_segments(
                &app,
                Some(&shared),
                Some((&preview_control, transcription_generation)),
                &transcriber,
                &settings,
                &samples_16khz,
                "Transcribing file",
            );

            match result {
                Ok(Some(output)) => {
                    if preview_control.current_generation() != transcription_generation {
                        return;
                    }

                    let text = cleanup_transcript_text(
                        output.text.trim(),
                        settings.cleanup_enabled,
                        &settings.cleanup_terms,
                    );

                    if text.is_empty() {
                        let mut core = shared.lock();
                        core.phase = AppPhase::Idle;
                        core.status_message = "Nothing intelligible was detected".to_string();
                        core.error_message = None;
                        core.model_status = current_model_status(&app, &core.settings);
                        core.parakeet_model_status = built_in_parakeet_status(&app);
                        clear_overlay_session_state(&mut core);
                        drop(core);
                        update_indicator_window(&app, &shared);
                        emit_snapshot(&app, &shared);
                        return;
                    }

                    let duration_ms =
                        (samples_16khz.len() as f64 / parakeet::SAMPLE_RATE as f64 * 1000.0) as u64;
                    let item_id = Uuid::new_v4().to_string();
                    let mut core = shared.lock();
                    core.history.insert(
                        0,
                        HistoryItem {
                            id: item_id,
                            text,
                            created_at: Utc::now().to_rfc3339(),
                            source_name: decoded.display_name,
                            mode: RecordingMode::Toggle,
                            duration_ms,
                            pasted: false,
                            audio_path: None,
                            capture: HistoryCaptureDetails {
                                source_kind: CaptureSourceKind::File,
                                model_id: settings.selected_model_id.clone(),
                                model_name: output.model_name,
                                inference_provider: output.inference_provider,
                                input_sample_rate,
                                input_channels,
                                transcription_sample_rate: parakeet::SAMPLE_RATE,
                            },
                        },
                    );
                    let keep_len = HISTORY_LIMIT.min(core.history.len());
                    let removed_items = core.history.split_off(keep_len);
                    for item in &removed_items {
                        remove_history_audio_file(item);
                    }
                    core.phase = AppPhase::Idle;
                    core.status_message = "File transcribed locally".to_string();
                    core.error_message = None;
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                    drop(core);

                    let _ = save_persisted_state(&app, &shared);
                    update_indicator_window(&app, &shared);
                    emit_snapshot(&app, &shared);
                }
                Ok(None) => {}
                Err(error) => {
                    let mut core = shared.lock();
                    core.phase = AppPhase::Error;
                    core.status_message = "File transcription failed".to_string();
                    core.error_message = Some(error.to_string());
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                    drop(core);
                    update_indicator_window(&app, &shared);
                    emit_snapshot(&app, &shared);
                }
            }
        }));

        if let Err(error) = outcome {
            let mut core = shared.lock();
            core.phase = AppPhase::Error;
            core.status_message = "File transcription worker failed".to_string();
            core.error_message = Some(format!(
                "The file transcription worker crashed unexpectedly: {}",
                panic_payload_message(error)
            ));
            core.model_status = current_model_status(&app, &core.settings);
            core.parakeet_model_status = built_in_parakeet_status(&app);
            clear_overlay_session_state(&mut core);
            drop(core);
            update_indicator_window(&app, &shared);
            emit_snapshot(&app, &shared);
        }
    });

    Ok(())
}

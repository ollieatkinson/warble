use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use std::path::PathBuf;
use std::time::Instant;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::constants::*;
use crate::models::*;
use crate::overlay::{
    clear_overlay_session_state, default_overlay_levels, update_indicator_window,
};
use crate::state::*;
use crate::storage::*;
use crate::transcript::{
    cleanup_transcript_text, note_capture_diagnostic, transcription_cancelled,
};
use crate::{media, parakeet, platform};

pub(crate) struct TranscriptionOutput {
    pub(crate) text: String,
    pub(crate) inference_provider: InferenceProvider,
    pub(crate) model_name: String,
}

fn append_transcription_log(app: &AppHandle, stage: &str, detail: impl Into<String>) {
    let detail = detail.into();
    let message = if detail.is_empty() {
        format!("{} [transcription] {stage}", Utc::now().to_rfc3339())
    } else {
        format!(
            "{} [transcription] {stage}: {detail}",
            Utc::now().to_rfc3339()
        )
    };
    append_capture_log(app, &message);
}

fn duration_ms(sample_count: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }

    ((sample_count as f64 / sample_rate as f64) * 1000.0) as u64
}

pub(crate) fn transcribe_audio(
    app: &AppHandle,
    transcriber: &TranscriberHandle,
    settings: &Settings,
    audio: &[f32],
) -> Result<TranscriptionOutput> {
    let selected_key = selected_model_cache_key(settings);
    let mut guard = transcriber.lock();
    let model_name = selected_model_display_name(settings);
    if guard.selected_key.as_ref() != Some(&selected_key) {
        append_transcription_log(
            app,
            "Loading transcription model",
            format!(
                "model={} kind={:?} cache_key={selected_key}",
                model_name, settings.selected_model_kind
            ),
        );
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
        append_transcription_log(
            app,
            "Transcription model ready",
            format!(
                "model={} kind={:?}",
                model_name, settings.selected_model_kind
            ),
        );
    }

    append_transcription_log(
        app,
        "Transcription inference started",
        format!(
            "model={} samples={} duration_ms={} kind={:?}",
            model_name,
            audio.len(),
            duration_ms(audio.len(), parakeet::SAMPLE_RATE),
            settings.selected_model_kind
        ),
    );
    let started_at = Instant::now();
    match guard.engine.as_mut().expect("transcriber initialized") {
        TranscriberEngine::Parakeet(model) => {
            let output = TranscriptionOutput {
                text: model.transcribe_audio(audio)?,
                inference_provider: model.provider(),
                model_name,
            };
            append_transcription_log(
                app,
                "Transcription inference finished",
                format!(
                    "model={} chars={} provider={} elapsed_ms={}",
                    output.model_name,
                    output.text.chars().count(),
                    output.inference_provider,
                    started_at.elapsed().as_millis()
                ),
            );
            Ok(output)
        }
        TranscriberEngine::ParakeetCtc(model) => {
            let output = TranscriptionOutput {
                text: model.transcribe_audio(audio)?,
                inference_provider: model.provider(),
                model_name,
            };
            append_transcription_log(
                app,
                "Transcription inference finished",
                format!(
                    "model={} chars={} provider={} elapsed_ms={}",
                    output.model_name,
                    output.text.chars().count(),
                    output.inference_provider,
                    started_at.elapsed().as_millis()
                ),
            );
            Ok(output)
        }
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
        append_transcription_log(app, "Transcription cancelled", "before inference started");
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
        append_transcription_log(
            app,
            "Transcribing single chunk",
            format!(
                "label={} samples={} duration_ms={}",
                progress_label,
                audio.len(),
                duration_ms(audio.len(), parakeet::SAMPLE_RATE)
            ),
        );
        let output = transcribe_audio(app, transcriber, settings, audio)?;
        if transcription_cancelled(shared, preview_control) {
            append_transcription_log(
                app,
                "Transcription cancelled",
                "after single chunk inference",
            );
            return Ok(None);
        }
        return Ok(Some(output));
    }

    let chunks = audio.chunks(chunk_samples).collect::<Vec<_>>();
    let mut combined = String::new();
    let mut provider = InferenceProvider::Cpu;
    let mut model_name = selected_model_display_name(settings);
    append_transcription_log(
        app,
        "Transcribing chunked audio",
        format!(
            "label={} chunks={} chunk_samples={} total_samples={} duration_ms={}",
            progress_label,
            chunks.len(),
            chunk_samples,
            audio.len(),
            duration_ms(audio.len(), parakeet::SAMPLE_RATE)
        ),
    );

    for (index, chunk) in chunks.iter().enumerate() {
        if transcription_cancelled(shared, preview_control) {
            append_transcription_log(
                app,
                "Transcription cancelled",
                format!("before chunk {} of {}", index + 1, chunks.len()),
            );
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

        append_transcription_log(
            app,
            "Chunk transcription started",
            format!(
                "{}/{} samples={} duration_ms={}",
                index + 1,
                chunks.len(),
                chunk.len(),
                duration_ms(chunk.len(), parakeet::SAMPLE_RATE)
            ),
        );
        let output = transcribe_audio(app, transcriber, settings, chunk)?;
        if transcription_cancelled(shared, preview_control) {
            append_transcription_log(
                app,
                "Transcription cancelled",
                format!("after chunk {} of {}", index + 1, chunks.len()),
            );
            return Ok(None);
        }
        provider = output.inference_provider;
        model_name = output.model_name;
        append_transcription_log(
            app,
            "Chunk transcription finished",
            format!(
                "{}/{} chars={} provider={}",
                index + 1,
                chunks.len(),
                output.text.chars().count(),
                output.inference_provider
            ),
        );

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
                        note_capture_diagnostic(
                            &app,
                            &shared,
                            "No speech detected",
                            format!(
                                "{} buffered samples from {}",
                                completed.captured_sample_count, completed.source_name
                            ),
                        );
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
                        Some(platform::paste_text(&app, &text))
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
                    let model_name = output.model_name.clone();
                    let inference_provider = output.inference_provider;

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
                                model_name,
                                inference_provider,
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
                            "Warble transcribed and pasted".to_string()
                        }
                        Some(platform::PasteOutcome::ClipboardOnly) => {
                            "Warble transcribed and copied to clipboard".to_string()
                        }
                        None => "Warble transcribed locally".to_string(),
                    };
                    core.error_message = paste_error;
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                    drop(core);

                    note_capture_diagnostic(
                        &app,
                        &shared,
                        "Microphone transcription complete",
                        format!(
                            "{} chars with {} on {}",
                            text.chars().count(),
                            inference_provider,
                            output.model_name
                        ),
                    );
                    let _ = save_persisted_state(&app, &shared);
                }
                Ok(None) => {}
                Err(error) => {
                    note_capture_diagnostic(
                        &app,
                        &shared,
                        "Microphone transcription failed",
                        error.to_string(),
                    );
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
            let panic_message = panic_payload_message(error);
            note_capture_diagnostic(
                &app,
                &shared,
                "Transcription worker crashed",
                panic_message.clone(),
            );
            let mut core = shared.lock();
            core.phase = AppPhase::Error;
            core.status_message = "Transcription worker failed".to_string();
            core.error_message = Some(format!(
                "The transcription worker crashed unexpectedly: {}",
                panic_message
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
    let file_size_bytes = file_path.metadata().ok().map(|metadata| metadata.len());
    append_transcription_log(
        &app,
        "File transcription requested",
        format!(
            "file={} path={} size_bytes={}",
            file_label,
            file_path.display(),
            file_size_bytes
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
    );

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
    note_capture_diagnostic(
        &app,
        &shared,
        "File transcription started",
        file_label.clone(),
    );

    std::thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            append_transcription_log(
                &app,
                "File transcription worker started",
                format!(
                    "file={} generation={}",
                    file_label, transcription_generation
                ),
            );
            append_transcription_log(
                &app,
                "Media decode started",
                file_path.display().to_string(),
            );
            let decoded = match media::decode_media_file(&file_path) {
                Ok(decoded) => decoded,
                Err(error) => {
                    append_transcription_log(&app, "Media decode failed", error.to_string());
                    note_capture_diagnostic(&app, &shared, "File decode failed", error.to_string());
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
                append_transcription_log(
                    &app,
                    "File transcription aborted",
                    "decode completed after cancellation",
                );
                return;
            }

            let input_sample_rate = decoded.sample_rate;
            let input_channels = decoded.channels;
            append_transcription_log(
                &app,
                "Media decode finished",
                format!(
                    "file={} sample_rate={} channels={} samples={} duration_ms={}",
                    decoded.display_name,
                    decoded.sample_rate,
                    decoded.channels,
                    decoded.samples.len(),
                    duration_ms(decoded.samples.len(), decoded.sample_rate)
                ),
            );
            note_capture_diagnostic(
                &app,
                &shared,
                "File decoded",
                format!(
                    "{} Hz · {} ch · {} samples",
                    decoded.sample_rate,
                    decoded.channels,
                    decoded.samples.len()
                ),
            );
            let samples_16khz = if decoded.sample_rate == parakeet::SAMPLE_RATE {
                append_transcription_log(
                    &app,
                    "Resample skipped",
                    format!("already {} Hz", parakeet::SAMPLE_RATE),
                );
                decoded.samples.clone()
            } else {
                append_transcription_log(
                    &app,
                    "Resample started",
                    format!(
                        "from {} Hz to {} Hz with {} samples",
                        decoded.sample_rate,
                        parakeet::SAMPLE_RATE,
                        decoded.samples.len()
                    ),
                );
                let started_at = Instant::now();
                let resampled = parakeet::resample_to_16khz(&decoded.samples, decoded.sample_rate);
                append_transcription_log(
                    &app,
                    "Resample finished",
                    format!(
                        "samples={} duration_ms={} elapsed_ms={}",
                        resampled.len(),
                        duration_ms(resampled.len(), parakeet::SAMPLE_RATE),
                        started_at.elapsed().as_millis()
                    ),
                );
                resampled
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
            append_transcription_log(
                &app,
                "File transcription inference queued",
                format!(
                    "file={} model={} kind={:?} samples={} duration_ms={}",
                    decoded.display_name,
                    settings.selected_model_id,
                    settings.selected_model_kind,
                    samples_16khz.len(),
                    duration_ms(samples_16khz.len(), parakeet::SAMPLE_RATE)
                ),
            );

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
                        append_transcription_log(
                            &app,
                            "File transcription aborted",
                            "inference completed after cancellation",
                        );
                        return;
                    }

                    append_transcription_log(
                        &app,
                        "File transcription inference finished",
                        format!(
                            "chars={} provider={} model={}",
                            output.text.chars().count(),
                            output.inference_provider,
                            output.model_name
                        ),
                    );
                    let text = cleanup_transcript_text(
                        output.text.trim(),
                        settings.cleanup_enabled,
                        &settings.cleanup_terms,
                    );
                    append_transcription_log(
                        &app,
                        "Transcript cleanup finished",
                        format!(
                            "raw_chars={} cleaned_chars={} cleanup_enabled={}",
                            output.text.chars().count(),
                            text.chars().count(),
                            settings.cleanup_enabled
                        ),
                    );

                    if text.is_empty() {
                        append_transcription_log(
                            &app,
                            "File transcription completed with empty transcript",
                            decoded.display_name.clone(),
                        );
                        note_capture_diagnostic(
                            &app,
                            &shared,
                            "No speech detected in file",
                            decoded.display_name.clone(),
                        );
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
                    let history_id = item_id.clone();
                    let model_name = output.model_name.clone();
                    let inference_provider = output.inference_provider;
                    let cleaned_char_count = text.chars().count();
                    let source_name = decoded.display_name.clone();
                    let mut core = shared.lock();
                    core.history.insert(
                        0,
                        HistoryItem {
                            id: item_id,
                            text,
                            created_at: Utc::now().to_rfc3339(),
                            source_name: source_name.clone(),
                            mode: RecordingMode::Toggle,
                            duration_ms,
                            pasted: false,
                            audio_path: None,
                            capture: HistoryCaptureDetails {
                                source_kind: CaptureSourceKind::File,
                                model_id: settings.selected_model_id.clone(),
                                model_name,
                                inference_provider,
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
                    core.status_message = "Warble transcribed the file locally".to_string();
                    core.error_message = None;
                    core.model_status = current_model_status(&app, &core.settings);
                    core.parakeet_model_status = built_in_parakeet_status(&app);
                    clear_overlay_session_state(&mut core);
                    drop(core);

                    note_capture_diagnostic(
                        &app,
                        &shared,
                        "File transcription complete",
                        format!(
                            "{} chars from {} with {}",
                            cleaned_char_count, source_name, inference_provider
                        ),
                    );
                    append_transcription_log(
                        &app,
                        "File transcription completed",
                        format!(
                            "file={} chars={} duration_ms={} history_id={history_id}",
                            source_name, cleaned_char_count, duration_ms
                        ),
                    );
                    let _ = save_persisted_state(&app, &shared);
                    update_indicator_window(&app, &shared);
                    emit_snapshot(&app, &shared);
                }
                Ok(None) => {
                    append_transcription_log(
                        &app,
                        "File transcription cancelled",
                        decoded.display_name,
                    );
                }
                Err(error) => {
                    append_transcription_log(&app, "File transcription failed", error.to_string());
                    note_capture_diagnostic(
                        &app,
                        &shared,
                        "File transcription failed",
                        error.to_string(),
                    );
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
            let panic_message = panic_payload_message(error);
            append_transcription_log(
                &app,
                "File transcription worker crashed",
                panic_message.clone(),
            );
            note_capture_diagnostic(
                &app,
                &shared,
                "File transcription worker crashed",
                panic_message.clone(),
            );
            let mut core = shared.lock();
            core.phase = AppPhase::Error;
            core.status_message = "File transcription worker failed".to_string();
            core.error_message = Some(format!(
                "The file transcription worker crashed unexpectedly: {}",
                panic_message
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

use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;

use crate::constants::*;
use crate::overlay::{
    default_overlay_levels, measure_overlay_levels, update_indicator_window, update_overlay_elapsed,
};
use crate::state::*;
use crate::storage::emit_snapshot;
use crate::transcript::{live_preview_text, note_preview_diagnostic};
use crate::{parakeet, streaming_preview};

const METER_LEVEL_CHANGE_THRESHOLD: f32 = 0.025;

pub(crate) fn spawn_live_preview(
    app: AppHandle,
    shared: SharedState,
    transcriber: TranscriberHandle,
    preview_control: PreviewControl,
    generation: u64,
    preview_buffer: Arc<Mutex<Vec<f32>>>,
    preview_sample_rate: u32,
) {
    std::thread::spawn(move || {
        let streaming_config = {
            let core = shared.lock();
            streaming_preview::resolve_streaming_preview_config(
                &core.settings,
                &core.system_profile,
            )
        };

        if let Some(config) = streaming_config {
            note_preview_diagnostic(
                &app,
                &shared,
                config.diagnostic_backend(),
                "Trying streaming preview",
                config.trying_detail(),
            );
            match run_streaming_live_preview_loop(
                &app,
                &shared,
                &preview_control,
                generation,
                preview_buffer.clone(),
                preview_sample_rate,
                config,
            ) {
                Ok(()) => return,
                Err(error) => {
                    note_preview_diagnostic(
                        &app,
                        &shared,
                        "batch-tdt",
                        "Streaming fallback",
                        error.to_string(),
                    );
                }
            }
        } else {
            note_preview_diagnostic(
                &app,
                &shared,
                "batch-tdt",
                "Using batch preview",
                "No streaming add-on is ready",
            );
        }

        run_batch_live_preview_loop(
            &app,
            &shared,
            &transcriber,
            &preview_control,
            generation,
            preview_buffer,
            preview_sample_rate,
        );
    });
}

fn run_streaming_live_preview_loop(
    app: &AppHandle,
    shared: &SharedState,
    preview_control: &PreviewControl,
    generation: u64,
    preview_buffer: Arc<Mutex<Vec<f32>>>,
    preview_sample_rate: u32,
    config: streaming_preview::StreamingPreviewConfig,
) -> Result<()> {
    let mut engine = streaming_preview::StreamingPreviewEngine::load(&config)?;
    let mut last_sample_count = 0usize;
    let mut last_preview = String::new();
    let mut preview_emitted = false;
    let mut seen_audio_ms = 0u64;

    loop {
        std::thread::sleep(Duration::from_millis(LIVE_STREAM_PREVIEW_INTERVAL_MS));

        if preview_control.current_generation() != generation {
            break;
        }

        {
            let core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription
            {
                break;
            }
        }

        let new_samples = {
            let samples = preview_buffer.lock().expect("preview buffer poisoned");
            if samples.len() <= last_sample_count {
                None
            } else {
                let chunk = samples[last_sample_count..].to_vec();
                last_sample_count = samples.len();
                Some(chunk)
            }
        };

        let Some(new_samples) = new_samples else {
            continue;
        };

        let preview_audio = parakeet::resample_to_16khz(&new_samples, preview_sample_rate);
        seen_audio_ms = seen_audio_ms.saturating_add(
            ((preview_audio.len() as f64 / parakeet::SAMPLE_RATE as f64) * 1000.0) as u64,
        );
        let transcript = match engine.push_audio(&preview_audio) {
            Ok(Some(text)) => text,
            Ok(None) => {
                if !preview_emitted && seen_audio_ms >= LIVE_STREAM_PREVIEW_FALLBACK_MS {
                    return Err(anyhow!(
                        "streaming preview did not yield text quickly enough"
                    ));
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        let (cleanup_enabled, cleanup_terms) = {
            let core = shared.lock();
            (
                core.settings.cleanup_enabled,
                core.settings.cleanup_terms.clone(),
            )
        };
        let preview = live_preview_text(&transcript, cleanup_enabled, &cleanup_terms);

        if preview_control.current_generation() != generation {
            break;
        }

        if preview.is_empty() || preview == last_preview {
            continue;
        }
        if !preview_emitted {
            note_preview_diagnostic(
                app,
                shared,
                engine.diagnostic_backend(),
                "Streaming live preview active",
                engine.active_detail(),
            );
        }
        preview_emitted = true;
        last_preview = preview.clone();

        {
            let mut core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription
            {
                break;
            }
            core.overlay.visible = true;
            core.overlay.title = "Listening".to_string();
            core.overlay.detail = preview;
        }

        update_indicator_window(app, shared);
        emit_snapshot(app, shared);
    }

    Ok(())
}

fn run_batch_live_preview_loop(
    app: &AppHandle,
    shared: &SharedState,
    transcriber: &TranscriberHandle,
    preview_control: &PreviewControl,
    generation: u64,
    preview_buffer: Arc<Mutex<Vec<f32>>>,
    preview_sample_rate: u32,
) {
    let mut last_sample_count = 0usize;
    let mut last_preview = String::new();
    let mut stabilizer = PreviewStabilizer::default();
    let mut unstable_preview_passes = 0usize;
    let mut batch_emitted = false;
    let preview_window_samples = preview_sample_rate as usize * LIVE_PREVIEW_WINDOW_SECONDS;
    let preview_min_samples = (preview_sample_rate as u64 * LIVE_PREVIEW_MIN_MS / 1_000) as usize;

    loop {
        std::thread::sleep(Duration::from_millis(LIVE_PREVIEW_INTERVAL_MS));

        if preview_control.current_generation() != generation {
            break;
        }

        {
            let core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription
            {
                break;
            }
        }

        let chunk = {
            let samples = preview_buffer.lock().expect("preview buffer poisoned");
            if samples.len() <= last_sample_count || samples.len() < preview_min_samples {
                None
            } else {
                last_sample_count = samples.len();
                let start = samples.len().saturating_sub(preview_window_samples);
                Some(samples[start..].to_vec())
            }
        };

        let Some(chunk) = chunk else {
            continue;
        };

        let preview_audio = parakeet::resample_to_16khz(&chunk, preview_sample_rate);
        let (settings, cleanup_enabled, cleanup_terms) = {
            let core = shared.lock();
            (
                core.settings.clone(),
                core.settings.cleanup_enabled,
                core.settings.cleanup_terms.clone(),
            )
        };
        let preview = match crate::transcription::transcribe_audio(
            app,
            transcriber,
            &settings,
            &preview_audio,
        ) {
            Ok(output) => {
                let cleaned_preview =
                    live_preview_text(&output.text, cleanup_enabled, &cleanup_terms);

                if cleaned_preview.is_empty() {
                    unstable_preview_passes = 0;
                    None
                } else if let Some(stable_preview) = stabilizer.observe(&cleaned_preview) {
                    unstable_preview_passes = 0;
                    Some(stable_preview)
                } else {
                    unstable_preview_passes += 1;
                    let word_count = cleaned_preview.split_whitespace().count();
                    if unstable_preview_passes >= LIVE_PREVIEW_DRAFT_FALLBACK_PASSES
                        && word_count >= 3
                    {
                        Some(cleaned_preview)
                    } else {
                        None
                    }
                }
            }
            Err(_) => continue,
        };
        let Some(preview) = preview else {
            continue;
        };

        if preview_control.current_generation() != generation {
            break;
        }

        if preview.is_empty() || preview == last_preview {
            continue;
        }
        if !batch_emitted {
            note_preview_diagnostic(
                app,
                shared,
                "batch-tdt",
                "Batch live preview active",
                "Using rolling Parakeet TDT partials",
            );
            batch_emitted = true;
        }
        last_preview = preview.clone();

        {
            let mut core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription
            {
                break;
            }
            core.overlay.visible = true;
            core.overlay.title = "Listening".to_string();
            core.overlay.detail = preview;
        }

        update_indicator_window(app, shared);
        emit_snapshot(app, shared);
    }
}

pub(crate) fn spawn_live_meter(
    app: AppHandle,
    shared: SharedState,
    preview_control: PreviewControl,
    generation: u64,
    preview_buffer: Arc<Mutex<Vec<f32>>>,
    preview_sample_rate: u32,
) {
    std::thread::spawn(move || {
        let mut last_levels = default_overlay_levels();
        let mut last_elapsed_second = u64::MAX;
        let meter_window_samples =
            (preview_sample_rate as u64 * LIVE_METER_WINDOW_MS / 1_000) as usize;

        loop {
            std::thread::sleep(Duration::from_millis(LIVE_METER_INTERVAL_MS));

            if preview_control.current_generation() != generation {
                break;
            }

            {
                let core = shared.lock();
                if !matches!(core.phase, AppPhase::Recording) {
                    break;
                }
            }

            let levels = {
                let samples = preview_buffer.lock().expect("preview buffer poisoned");
                let start = samples.len().saturating_sub(meter_window_samples);
                measure_overlay_levels(&samples[start..], preview_sample_rate)
            };

            if preview_control.current_generation() != generation {
                break;
            }

            let (timer_enabled, elapsed_ms) = {
                let mut core = shared.lock();
                if !matches!(core.phase, AppPhase::Recording) {
                    break;
                }
                update_overlay_elapsed(&mut core);
                (core.settings.show_recording_timer, core.overlay.elapsed_ms)
            };
            let elapsed_second = elapsed_ms / 1_000;

            if levels
                .iter()
                .zip(last_levels.iter())
                .all(|(next, previous)| (next - previous).abs() < METER_LEVEL_CHANGE_THRESHOLD)
                && (!timer_enabled || elapsed_second == last_elapsed_second)
            {
                continue;
            }
            last_levels = levels.clone();
            last_elapsed_second = elapsed_second;

            {
                let mut core = shared.lock();
                if !matches!(core.phase, AppPhase::Recording) {
                    break;
                }
                core.overlay.levels = levels;
                core.overlay.elapsed_ms = elapsed_ms;
            }

            emit_snapshot(&app, &shared);
        }
    });
}

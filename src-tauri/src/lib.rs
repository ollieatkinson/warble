mod constants;
mod media;
mod models;
pub mod parakeet;
mod platform;
mod state;
mod storage;
mod streaming_preview;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig, SupportedStreamConfig};
use regex::{Regex, RegexBuilder};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use uuid::Uuid;
use constants::*;
use models::*;
use state::*;
use storage::*;

fn make_source_id(index: usize, name: &str) -> String {
    format!("{index}:{name}")
}

fn enumerate_sources() -> Vec<SourceInfo> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|device| device.name().ok());

    host.input_devices()
        .map(|devices| {
            devices
                .enumerate()
                .map(|(index, device)| {
                    let name = device
                        .name()
                        .unwrap_or_else(|_| format!("Input {}", index + 1));
                    let config = device.default_input_config().ok();
                    SourceInfo {
                        id: make_source_id(index, &name),
                        name: name.clone(),
                        sample_rate: config.as_ref().map(|cfg| cfg.sample_rate().0).unwrap_or(0),
                        channels: config.as_ref().map(|cfg| cfg.channels()).unwrap_or(0),
                        is_default: default_name.as_ref().map(|value| value == &name).unwrap_or(false),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_selected_device(
    selected_source_id: Option<&str>,
) -> Result<(cpal::Device, SourceInfo, SupportedStreamConfig)> {
    let host = cpal::default_host();
    let default_name = host.default_input_device().and_then(|device| device.name().ok());
    let devices = host.input_devices().context("failed to read input devices")?;

    let mut first_candidate: Option<(cpal::Device, SourceInfo, SupportedStreamConfig)> = None;

    for (index, device) in devices.enumerate() {
        let name = device
            .name()
            .unwrap_or_else(|_| format!("Input {}", index + 1));
        let Ok(config) = device.default_input_config() else {
            continue;
        };

        let info = SourceInfo {
            id: make_source_id(index, &name),
            name: name.clone(),
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
            is_default: default_name.as_ref().map(|value| value == &name).unwrap_or(false),
        };

        if first_candidate.is_none() {
            first_candidate = Some((device.clone(), info.clone(), config.clone()));
        }

        if selected_source_id.map(|value| value == info.id).unwrap_or(info.is_default) {
            return Ok((device, info, config));
        }
    }

    first_candidate.ok_or_else(|| anyhow!("No input devices were found"))
}

fn write_input_data<T>(input: &[T], channels: usize, destination: &Arc<Mutex<Vec<f32>>>)
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let mut samples = destination.lock().expect("recording buffer poisoned");
    for frame in input.chunks(channels) {
        let sum: f32 = frame
            .iter()
            .map(|sample| f32::from_sample(*sample))
            .sum();
        samples.push(sum / channels as f32);
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    destination: Arc<Mutex<Vec<f32>>>,
) -> Result<Stream>
where
    T: Sample + SizedSample + Send + 'static,
    f32: FromSample<T>,
{
    let error_callback = |error| eprintln!("audio stream error: {error}");
    let stream = device.build_input_stream(
        config,
        move |input: &[T], _| write_input_data(input, channels, &destination),
        error_callback,
        None,
    )?;
    Ok(stream)
}

fn begin_recording(app: &AppHandle, shared: &SharedState, mode: RecordingMode) -> Result<()> {
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
    recorder
        .sender
        .send(RecorderRequest::Start {
            selected_source_id,
            mode: mode.clone(),
            anchor,
            response: response_tx,
        })
        .map_err(|_| anyhow!("Recording worker is unavailable"))?;

    let started = response_rx
        .recv()
        .map_err(|_| anyhow!("Recording worker did not respond"))?
        .map_err(|error| anyhow!(error))?;

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
        spawn_live_preview(
            app.clone(),
            shared.clone(),
            transcriber.inner().clone(),
            preview_control.inner().clone(),
            preview_generation,
            started.preview_buffer.clone(),
            started.preview_sample_rate,
        );
    }
    spawn_live_meter(
        app.clone(),
        shared.clone(),
        preview_control.inner().clone(),
        preview_generation,
        started.preview_buffer,
        started.preview_sample_rate,
    );
    Ok(())
}

fn finalize_recording(session: RecordingSession) -> Result<Option<CompletedRecording>> {
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

struct TranscriptionOutput {
    text: String,
    inference_provider: InferenceProvider,
    model_name: String,
}

fn transcribe_audio_segments(
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
                core.status_message =
                    format!("{progress_label} ({}/{})", index + 1, chunks.len());
                core.overlay.visible = true;
                core.overlay.title = progress_label.to_string();
                core.overlay.detail =
                    format!("Chunk {} of {}", index + 1, chunks.len());
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

fn transcribe_audio(
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
            inference_provider: InferenceProvider::Cpu,
            model_name,
        }),
    }
}

fn default_cleanup_terms() -> Vec<String> {
    DEFAULT_CLEANUP_TERMS
        .iter()
        .map(|term| term.to_string())
        .collect()
}

fn normalize_cleanup_term(term: &str) -> Option<String> {
    let normalized = term
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase();

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_cleanup_terms(terms: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for term in terms {
        let Some(term) = normalize_cleanup_term(term) else {
            continue;
        };

        if normalized.iter().any(|existing| existing == &term) {
            continue;
        }

        normalized.push(term);
    }

    normalized
}

fn condense_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cleanup_patterns_from_terms(terms: &[String]) -> Vec<Regex> {
    let mut ordered = normalize_cleanup_terms(terms);
    ordered.sort_by(|left, right| right.len().cmp(&left.len()));

    ordered
        .into_iter()
        .filter_map(|term| {
            let escaped = regex::escape(&term).replace("\\ ", r"\s+");
            let pattern = format!(
                r#"(?i)(^|[\s\(\[\{{"'“”‘’,.;:!?]+){escaped}([\s\)\]\}}"'“”‘’,.;:!?]+|$)"#
            );
            RegexBuilder::new(&pattern).case_insensitive(true).build().ok()
        })
        .collect()
}

fn cleanup_transcript_text(text: &str, cleanup_enabled: bool, cleanup_terms: &[String]) -> String {
    let mut cleaned = condense_whitespace(text);
    if cleaned.is_empty() || !cleanup_enabled {
        return cleaned;
    }

    for pattern in cleanup_patterns_from_terms(cleanup_terms) {
        cleaned = pattern.replace_all(&cleaned, " ").into_owned();
    }

    let cleaned = condense_whitespace(&cleaned);
    let punctuation_spacing =
        Regex::new(r#"\s+([,.;:!?])"#).expect("punctuation spacing regex is valid");
    let cleaned = punctuation_spacing.replace_all(&cleaned, "$1").into_owned();
    let repeated_commas = Regex::new(r#"(,\s*){2,}"#).expect("repeated comma regex is valid");
    let cleaned = repeated_commas.replace_all(&cleaned, ", ").into_owned();

    cleaned
        .trim_matches(|character: char| character.is_whitespace() || [',', ';', ':'].contains(&character))
        .trim()
        .to_string()
}

fn live_preview_text(text: &str, cleanup_enabled: bool, cleanup_terms: &[String]) -> String {
    let mut lines = text
        .lines()
        .map(|line| cleanup_transcript_text(line, cleanup_enabled, cleanup_terms))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        let cleaned = cleanup_transcript_text(text, cleanup_enabled, cleanup_terms);
        if cleaned.is_empty() {
            return String::new();
        }

        return trim_preview_line(&cleaned, LIVE_PREVIEW_MAX_WORDS);
    }

    if lines.len() > 3 {
        lines = lines.split_off(lines.len().saturating_sub(3));
    }

    let mut total_words = lines
        .iter()
        .map(|line| line.split_whitespace().count())
        .sum::<usize>();

    while total_words > LIVE_PREVIEW_MAX_WORDS && !lines.is_empty() {
        let first_word_count = lines[0].split_whitespace().count();
        if lines.len() == 1 {
            lines[0] = trim_preview_line(&lines[0], LIVE_PREVIEW_MAX_WORDS);
            break;
        }
        total_words = total_words.saturating_sub(first_word_count);
        lines.remove(0);
    }

    if lines.is_empty() {
        return String::new();
    }

    if let Some(last_line) = lines.last_mut() {
        *last_line = trim_preview_line(last_line, LIVE_PREVIEW_MAX_WORDS);
    }

    lines.join("\n")
}

fn trim_preview_line(text: &str, max_words: usize) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() <= max_words {
        return text.to_string();
    }

    words[words.len().saturating_sub(max_words)..].join(" ")
}

fn transcription_cancelled(
    shared: Option<&SharedState>,
    preview_control: Option<(&PreviewControl, u64)>,
) -> bool {
    if let Some((preview_control, generation)) = preview_control {
        if preview_control.current_generation() != generation {
            return true;
        }
    }

    if let Some(shared) = shared {
        let core = shared.lock();
        matches!(core.phase, AppPhase::Idle | AppPhase::Error)
    } else {
        false
    }
}

fn preview_words(text: &str) -> Vec<String> {
    text.split_whitespace().map(ToString::to_string).collect()
}

fn normalize_preview_word(word: &str) -> String {
    word.trim_matches(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                ',' | '.' | ';' | ':' | '!' | '?' | '"' | '\'' | '(' | ')' | '[' | ']' | '{'
                    | '}'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
            )
    })
    .to_lowercase()
}

fn common_prefix_len(left: &[String], right: &[String]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(lhs, rhs)| {
            let lhs = normalize_preview_word(lhs);
            let rhs = normalize_preview_word(rhs);
            !lhs.is_empty() && lhs == rhs
        })
        .count()
}

fn render_preview_words(words: &[String]) -> String {
    if words.is_empty() {
        return String::new();
    }

    let start = words.len().saturating_sub(LIVE_PREVIEW_MAX_WORDS);
    words[start..].join(" ")
}

impl PreviewStabilizer {
    fn observe(&mut self, partial: &str) -> Option<String> {
        let current_words = preview_words(partial);
        if current_words.is_empty() {
            return None;
        }

        let previous_words = self.last_partial.clone();
        if let Some(previous_words) = previous_words.as_ref() {
            let stable_prefix_len = common_prefix_len(&self.stable_words, &current_words);
            if !self.stable_words.is_empty() && stable_prefix_len == 0 {
                self.divergence_count += 1;
                if self.divergence_count >= LIVE_PREVIEW_RESET_AFTER_DIVERGENCE {
                    self.stable_words.clear();
                    self.last_partial = Some(current_words);
                    self.divergence_count = 0;
                    return None;
                }
            } else {
                self.divergence_count = 0;
            }

            let shared_len = common_prefix_len(previous_words, &current_words);
            if shared_len > self.stable_words.len() {
                self.stable_words = current_words[..shared_len].to_vec();
            }
        }

        self.last_partial = Some(current_words);
        if self.stable_words.is_empty() {
            None
        } else {
            Some(render_preview_words(&self.stable_words))
        }
    }
}

fn default_overlay_levels() -> Vec<f32> {
    vec![0.0; LIVE_METER_BAR_COUNT]
}

fn hanning_window(index: usize, length: usize) -> f32 {
    if length <= 1 {
        return 1.0;
    }

    let phase = (2.0 * std::f32::consts::PI * index as f32) / (length - 1) as f32;
    0.5 - 0.5 * phase.cos()
}

fn goertzel_power(samples: &[f32], sample_rate: u32, target_frequency: f32) -> f32 {
    if samples.is_empty() || sample_rate == 0 {
        return 0.0;
    }

    let normalized_frequency = (target_frequency / sample_rate as f32).clamp(0.0, 0.5);
    if normalized_frequency <= 0.0 {
        return 0.0;
    }

    let omega = 2.0 * std::f32::consts::PI * normalized_frequency;
    let coefficient = 2.0 * omega.cos();
    let mut q1 = 0.0f32;
    let mut q2 = 0.0f32;

    for (index, sample) in samples.iter().enumerate() {
        let weighted = *sample * hanning_window(index, samples.len());
        let q0 = coefficient * q1 - q2 + weighted;
        q2 = q1;
        q1 = q0;
    }

    q1 * q1 + q2 * q2 - coefficient * q1 * q2
}

fn measure_overlay_levels(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() || sample_rate == 0 {
        return default_overlay_levels();
    }

    let analysis_len = samples.len().min(LIVE_METER_ANALYSIS_SAMPLES).max(256);
    let window = &samples[samples.len().saturating_sub(analysis_len)..];
    let (sum_squares, peak) = window.iter().fold((0.0f32, 0.0f32), |(sum, peak), sample| {
        let magnitude = sample.abs();
        (sum + sample * sample, peak.max(magnitude))
    });
    let rms = (sum_squares / window.len() as f32).sqrt();

    if rms < LIVE_METER_SILENCE_RMS_THRESHOLD && peak < LIVE_METER_SILENCE_PEAK_THRESHOLD {
        return default_overlay_levels();
    }

    let nyquist = sample_rate as f32 * 0.5;
    let min_frequency = 120.0f32;
    let max_frequency = (nyquist * 0.82).min(5_800.0).max(min_frequency * 1.5);
    let ratio = (max_frequency / min_frequency).powf(1.0 / (LIVE_METER_BAR_COUNT as f32 - 1.0));

    let powers = (0..LIVE_METER_BAR_COUNT)
        .map(|index| {
            let center_frequency = min_frequency * ratio.powf(index as f32);
            goertzel_power(window, sample_rate, center_frequency)
        })
        .collect::<Vec<_>>();

    let max_power = powers
        .iter()
        .copied()
        .fold(0.0f32, f32::max)
        .max(1e-9);
    let rms_drive = ((rms - LIVE_METER_SILENCE_RMS_THRESHOLD)
        / (LIVE_METER_FULL_RMS - LIVE_METER_SILENCE_RMS_THRESHOLD))
        .clamp(0.0, 1.0);
    let peak_drive = ((peak - LIVE_METER_SILENCE_PEAK_THRESHOLD)
        / (LIVE_METER_FULL_PEAK - LIVE_METER_SILENCE_PEAK_THRESHOLD))
        .clamp(0.0, 1.0);
    let activity = rms_drive.max(peak_drive).powf(0.85);

    if activity <= 0.01 {
        return default_overlay_levels();
    }

    let mut levels = default_overlay_levels();
    for (index, level) in levels.iter_mut().enumerate() {
        let normalized = (powers[index] / max_power).clamp(0.0, 1.0).sqrt();
        let gated = (normalized * activity).clamp(0.0, 1.0);
        *level = if gated < 0.025 { 0.0 } else { gated };
    }

    levels
}

fn indicator_window_size(settings: &Settings) -> (i32, i32) {
    let content_height = if settings.show_live_transcription {
        if settings.show_recording_timer { 102 } else { 94 }
    } else {
        58
    };
    let content_width = if settings.show_live_transcription {
        if settings.show_recording_timer {
            364
        } else {
            324
        }
    } else {
        match settings.overlay_animation_style {
            OverlayAnimationStyle::Radial => {
                if settings.show_recording_timer {
                    144
                } else {
                    86
                }
            }
            OverlayAnimationStyle::Spectrum => {
                if settings.show_recording_timer {
                    194
                } else {
                    136
                }
            }
            OverlayAnimationStyle::Waveform => {
                if settings.show_recording_timer {
                    202
                } else {
                    142
                }
            }
        }
    } as i32;

    (
        content_width + INDICATOR_WINDOW_PADDING * 2,
        content_height + INDICATOR_WINDOW_PADDING * 2,
    )
}

fn update_overlay_elapsed(core: &mut AppCore) {
    if let Some(started_at) = core.recording_started_at {
        core.overlay.elapsed_ms = started_at.elapsed().as_millis() as u64;
    }
}

fn clear_overlay_session_state(core: &mut AppCore) {
    core.overlay.visible = false;
    core.overlay.title.clear();
    core.overlay.detail.clear();
    core.overlay.levels = default_overlay_levels();
    core.overlay.elapsed_ms = 0;
    core.overlay.limit_ms = None;
    core.overlay.anchor = None;
    core.recording_started_at = None;
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        show_main_window(app);
    }
}

fn create_tray_icon(app: &AppHandle) -> Result<()> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let show_item = MenuItem::with_id(app, TRAY_SHOW_ID, "Open Transcribed", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, TRAY_HIDE_ID, "Hide Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .context("missing default window icon")?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Transcribed")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_HIDE_ID => hide_main_window(app),
            TRAY_QUIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn indicator_origin(
    app: &AppHandle,
    overlay: &OverlaySnapshot,
    position: OverlayPosition,
    indicator_width: i32,
    indicator_height: i32,
) -> Option<PhysicalPosition<i32>> {
    let window = app.get_webview_window("indicator")?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let work_area = monitor.work_area();
    let left = work_area.position.x;
    let top = work_area.position.y;
    let width = work_area.size.width as i32;
    let height = work_area.size.height as i32;

    if matches!(position, OverlayPosition::Caret) {
        if let Some(anchor) = overlay.anchor {
            let min_x = left + INDICATOR_MARGIN;
            let max_x = left + width - indicator_width - INDICATOR_MARGIN;
            let min_y = top + INDICATOR_MARGIN;
            let max_y = top + height - indicator_height - INDICATOR_MARGIN;

            return Some(PhysicalPosition::new(
                anchor.x.clamp(min_x, max_x.max(min_x)),
                anchor.y.clamp(min_y, max_y.max(min_y)),
            ));
        }
    }

    let x = match position {
        OverlayPosition::BottomLeft => left + INDICATOR_MARGIN,
        OverlayPosition::BottomRight => left + width - indicator_width - INDICATOR_MARGIN,
        OverlayPosition::BottomCenter | OverlayPosition::Caret => {
            left + (width - indicator_width) / 2
        }
    };
    let y = top + height - indicator_height - INDICATOR_MARGIN;

    Some(PhysicalPosition::new(x.max(left), y.max(top)))
}

fn register_shortcuts(app: &AppHandle, shared: &SharedState) -> Result<()> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let settings = {
            let core = shared.lock();
            core.settings.clone()
        };
        let hold_shortcut = normalize_shortcut(&settings.hold_shortcut);
        let toggle_shortcut = normalize_shortcut(&settings.toggle_shortcut);
        let cancel_shortcut = normalize_shortcut(CANCEL_SHORTCUT);

        if hold_shortcut == toggle_shortcut {
            return Err(anyhow!("Hold and toggle shortcuts must be different"));
        }
        if hold_shortcut == cancel_shortcut || toggle_shortcut == cancel_shortcut {
            return Err(anyhow!("Escape is reserved for cancel"));
        }

        app.global_shortcut().unregister_all()?;
        app.global_shortcut().register(settings.hold_shortcut.as_str())?;
        app.global_shortcut().register(settings.toggle_shortcut.as_str())?;
        let escape_registered = app.global_shortcut().register(CANCEL_SHORTCUT).is_ok();

        let mut core = shared.lock();
        core.shortcuts_active = true;
        core.shortcut_message = if escape_registered {
            format!(
                "Listening for {}, {}, and Esc",
                settings.hold_shortcut, settings.toggle_shortcut
            )
        } else {
            format!(
                "Listening for {} and {}",
                settings.hold_shortcut, settings.toggle_shortcut
            )
        };
        core.error_message = None;
    }

    Ok(())
}

fn update_indicator_window(app: &AppHandle, shared: &SharedState) {
    let (overlay, settings) = {
        let core = shared.lock();
        (core.overlay.clone(), core.settings.clone())
    };

    let Some(window) = app.get_webview_window("indicator") else {
        return;
    };

    let (indicator_width, indicator_height) = indicator_window_size(&settings);
    let _ = window.set_size(Size::Physical(PhysicalSize::new(
        indicator_width.max(1) as u32,
        indicator_height.max(1) as u32,
    )));

    if overlay.visible {
        if let Some(position) = indicator_origin(
            app,
            &overlay,
            settings.overlay_position,
            indicator_width,
            indicator_height,
        ) {
            let _ = window.set_position(Position::Physical(position));
        }
        let _ = window.show();
    } else {
        let _ = window.hide();
    }
}

fn spawn_live_preview(
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
            streaming_preview::resolve_streaming_preview_config(&core.settings, &core.system_profile)
        };

        if let Some(config) = streaming_config {
            if run_streaming_live_preview_loop(
                &app,
                &shared,
                &preview_control,
                generation,
                preview_buffer.clone(),
                preview_sample_rate,
                config,
            )
            .is_ok()
            {
                return;
            }
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
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription {
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
        preview_emitted = true;
        last_preview = preview.clone();

        {
            let mut core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription {
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
    let preview_window_samples = preview_sample_rate as usize * LIVE_PREVIEW_WINDOW_SECONDS;
    let preview_min_samples = (preview_sample_rate as u64 * LIVE_PREVIEW_MIN_MS / 1_000) as usize;

    loop {
        std::thread::sleep(Duration::from_millis(LIVE_PREVIEW_INTERVAL_MS));

        if preview_control.current_generation() != generation {
            break;
        }

        {
            let core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription {
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
        let preview = match transcribe_audio(app, transcriber, &settings, &preview_audio) {
            Ok(output) => stabilizer.observe(&live_preview_text(
                &output.text,
                cleanup_enabled,
                &cleanup_terms,
            )),
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
        last_preview = preview.clone();

        {
            let mut core = shared.lock();
            if !matches!(core.phase, AppPhase::Recording) || !core.settings.show_live_transcription {
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

fn spawn_live_meter(
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
                .all(|(next, previous)| (next - previous).abs() < 0.025)
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

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(message) => (*message).to_string(),
            Err(_) => "unknown panic".to_string(),
        },
    }
}

fn complete_transcription(
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

                    let pasted = {
                        if settings.auto_paste {
                            platform::paste_text(&text).is_ok()
                        } else {
                            false
                        }
                    };
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
                    core.status_message = if pasted {
                        "Transcribed in Rust and pasted".to_string()
                    } else {
                        "Transcribed in Rust".to_string()
                    };
                    core.error_message = None;
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

fn stop_recording(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    let transcription_generation = preview_control.next_generation();
    let (response_tx, response_rx) = mpsc::channel();
    recorder
        .sender
        .send(RecorderRequest::Stop { response: response_tx })
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
    complete_transcription(
        app.clone(),
        shared.clone(),
        transcriber.inner().clone(),
        preview_control.inner().clone(),
        transcription_generation,
        completed,
    );
    Ok(())
}

fn transcribe_media_file(
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
                        (samples_16khz.len() as f64 / parakeet::SAMPLE_RATE as f64 * 1000.0)
                            as u64;
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

fn cancel_current_operation(app: &AppHandle, shared: &SharedState) -> Result<()> {
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
                .send(RecorderRequest::Stop { response: response_tx })
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

fn create_indicator_window(app: &AppHandle) -> Result<()> {
    if app.get_webview_window("indicator").is_some() {
        return Ok(());
    }

    let (indicator_width, indicator_height) = indicator_window_size(&Settings::default());
    let window = WebviewWindowBuilder::new(
        app,
        "indicator",
        WebviewUrl::App("index.html?indicator=1".into()),
    )
    .title("Transcribed Indicator")
    .transparent(true)
    .decorations(false)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible(false)
    .focused(false)
    .inner_size(indicator_width as f64, indicator_height as f64)
    .build()?;

    let _ = window.set_focusable(true);
    let _ = window.set_ignore_cursor_events(false);
    let _ = window.set_shadow(false);
    Ok(())
}

fn refresh_sources(app: &AppHandle, shared: &SharedState) {
    {
        let mut core = shared.lock();
        core.sources = enumerate_sources();
        core.model_status = current_model_status(app, &core.settings);
        core.parakeet_model_status = built_in_parakeet_status(app);
    }
    emit_snapshot(app, shared);
}

fn spawn_recorder_thread() -> RecorderHandle {
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
                        let destination = Arc::new(Mutex::new(Vec::<f32>::new()));

                        let stream = match supported_config.sample_format() {
                            SampleFormat::F32 => build_input_stream::<f32>(
                                &device,
                                &config,
                                channels,
                                destination.clone(),
                            )?,
                            SampleFormat::I16 => build_input_stream::<i16>(
                                &device,
                                &config,
                                channels,
                                destination.clone(),
                            )?,
                            SampleFormat::U16 => build_input_stream::<u16>(
                                &device,
                                &config,
                                channels,
                                destination.clone(),
                            )?,
                            other => return Err(anyhow!("Unsupported sample format: {other:?}")),
                        };

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
                        Some(session) => finalize_recording(session).map_err(|error| error.to_string()),
                        None => Ok(None),
                    };
                    let _ = response.send(result);
                }
            }
        }
    });

    RecorderHandle { sender }
}

#[tauri::command]
fn get_snapshot(app: AppHandle, shared: tauri::State<'_, SharedState>) -> Result<Snapshot, String> {
    refresh_sources(&app, &shared);
    Ok(build_snapshot(&app, &shared))
}

#[tauri::command]
fn refresh_devices(app: AppHandle, shared: tauri::State<'_, SharedState>) {
    refresh_sources(&app, &shared);
}

#[tauri::command]
fn inspect_model_path(path: String) -> Result<ModelPathInspection, String> {
    inspect_model_candidate(&path)
}

#[tauri::command]
fn install_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
    model_kind: TranscriptionModelKind,
    path: String,
) -> Result<(), String> {
    let inspection = inspect_model_candidate(&path)?;
    if !inspection.ready || !inspection.compatible {
        return Err(match model_kind {
            TranscriptionModelKind::Parakeet => "That folder is not a usable Parakeet TDT model.".to_string(),
            TranscriptionModelKind::ParakeetCtc => "That folder is not a usable Parakeet CTC model.".to_string(),
        });
    }

    if inspection.model_kind != model_kind {
        return Err(match model_kind {
            TranscriptionModelKind::Parakeet => "Choose a compatible Parakeet TDT folder for this catalog entry.".to_string(),
            TranscriptionModelKind::ParakeetCtc => "Choose a compatible Parakeet CTC folder for this catalog entry.".to_string(),
        });
    }

    activate_catalog_model(&app, &shared, model_id, model_kind, inspection)
}

#[tauri::command]
fn download_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
) -> Result<(), String> {
    models::download_catalog_model(&app, &shared, model_id)
}

#[tauri::command]
fn remove_catalog_model(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    model_id: String,
) -> Result<(), String> {
    models::remove_catalog_model(&app, &shared, model_id)
}

#[tauri::command]
fn update_settings_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    update: SettingsUpdate,
) -> Result<(), String> {
    let previous_settings = {
        let core = shared.lock();
        core.settings.clone()
    };
    let hold_shortcut = update
        .hold_shortcut
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let toggle_shortcut = update
        .toggle_shortcut
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    {
        let mut core = shared.lock();
        if let Some(hold_shortcut) = hold_shortcut {
            core.settings.hold_shortcut = hold_shortcut;
        }
        if let Some(toggle_shortcut) = toggle_shortcut {
            core.settings.toggle_shortcut = toggle_shortcut;
        }
        if let Some(selected_source_id) = update.selected_source_id {
            core.settings.selected_source_id = Some(selected_source_id);
        }
        if let Some(auto_paste) = update.auto_paste {
            core.settings.auto_paste = auto_paste;
        }
        if let Some(selected_model_id) = update.selected_model_id {
            core.settings.selected_model_id = selected_model_id;
        }
        if let Some(selected_model_kind) = update.selected_model_kind {
            core.settings.selected_model_kind = selected_model_kind;
        }
        if let Some(selected_model_path) = update.selected_model_path {
            core.settings.selected_model_path = path_if_not_empty(selected_model_path);
        }
        if let Some(cleanup_enabled) = update.cleanup_enabled {
            core.settings.cleanup_enabled = cleanup_enabled;
        }
        if let Some(cleanup_terms) = update.cleanup_terms {
            core.settings.cleanup_terms = normalize_cleanup_terms(&cleanup_terms);
        }
        if let Some(audio_retention_policy) = update.audio_retention_policy {
            core.settings.audio_retention_policy = audio_retention_policy;
        }
        if let Some(overlay_position) = update.overlay_position {
            core.settings.overlay_position = overlay_position;
        }
        if let Some(overlay_animation_style) = update.overlay_animation_style {
            core.settings.overlay_animation_style = overlay_animation_style;
        }
        if let Some(show_recording_timer) = update.show_recording_timer {
            core.settings.show_recording_timer = show_recording_timer;
        }
        if let Some(show_live_transcription) = update.show_live_transcription {
            core.settings.show_live_transcription = show_live_transcription;
            if !show_live_transcription {
                core.overlay.detail.clear();
            }
        }
        if matches!(core.phase, AppPhase::Recording | AppPhase::Transcribing) {
            core.overlay.limit_ms = selected_model_audio_limit_ms(&core.settings);
        }
    }

    if let Err(error) = register_shortcuts(&app, &shared) {
        {
            let mut core = shared.lock();
            core.settings = previous_settings.clone();
            core.shortcuts_active = false;
            core.shortcut_message = "Global shortcuts are unavailable".to_string();
            core.status_message = "Shortcut update failed".to_string();
            core.error_message = Some(error.to_string());
        }
        let _ = register_shortcuts(&app, &shared);
        return Err(error.to_string());
    }
    persist_and_emit_settings_change(&app, &shared)?;
    Ok(())
}

#[tauri::command]
fn remove_history_item(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    id: String,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        let Some(index) = core.history.iter().position(|item| item.id == id) else {
            return Ok(());
        };

        let item = core.history.remove(index);
        remove_history_audio_file(&item);
        core.status_message = "History item removed".to_string();
        core.error_message = None;
    }

    save_persisted_state(&app, &shared).map_err(|error| error.to_string())?;
    emit_snapshot(&app, &shared);
    Ok(())
}

#[tauri::command]
fn add_cleanup_term(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    term: String,
) -> Result<(), String> {
    let normalized = normalize_cleanup_term(&term)
        .ok_or_else(|| "Enter a filler word or phrase to remove.".to_string())?;

    {
        let mut core = shared.lock();
        if core.settings.cleanup_terms.iter().any(|existing| existing == &normalized) {
            return Ok(());
        }
        core.settings.cleanup_terms.push(normalized);
        core.settings.cleanup_terms = normalize_cleanup_terms(&core.settings.cleanup_terms);
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
fn remove_cleanup_term(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    term: String,
) -> Result<(), String> {
    let Some(normalized) = normalize_cleanup_term(&term) else {
        return Ok(());
    };

    {
        let mut core = shared.lock();
        core.settings
            .cleanup_terms
            .retain(|existing| existing != &normalized);
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
fn restore_default_cleanup_terms(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        core.settings.cleanup_enabled = true;
        core.settings.cleanup_terms = default_cleanup_terms();
    }

    persist_and_emit_settings_change(&app, &shared)
}

#[tauri::command]
fn clear_error_message_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        core.error_message = None;
        if matches!(core.phase, AppPhase::Error) {
            core.phase = AppPhase::Idle;
            if core.status_message == "Transcription failed" {
                core.status_message = "Ready".to_string();
            }
        }
    }

    emit_snapshot(&app, &shared);
    Ok(())
}

#[tauri::command]
fn start_manual_recording(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    mode: RecordingMode,
) -> Result<(), String> {
    begin_recording(&app, &shared, mode).map_err(|error| error.to_string())
}

#[tauri::command]
fn stop_manual_recording(app: AppHandle, shared: tauri::State<'_, SharedState>) -> Result<(), String> {
    stop_recording(&app, &shared).map_err(|error| error.to_string())
}

#[tauri::command]
fn transcribe_media_file_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
    path: String,
) -> Result<(), String> {
    let transcriber = app.state::<TranscriberHandle>().inner().clone();
    transcribe_media_file(app, shared.inner().clone(), transcriber, path)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_current_operation_command(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<(), String> {
    cancel_current_operation(&app, &shared).map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recorder = spawn_recorder_thread();
    let transcriber = TranscriberHandle::default();
    let preview_control = PreviewControl::default();
    let shared = SharedState::new({
        let placeholder = PersistedState {
            settings: Settings::default(),
            history: Vec::new(),
        };
        AppCore::new(placeholder.settings, placeholder.history)
    });

    tauri::Builder::default()
        .manage(shared.clone())
        .manage(recorder)
        .manage(transcriber)
        .manage(preview_control)
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            let persisted = load_persisted_state(app.handle());
            {
                let mut core = shared.lock();
                core.settings = persisted.settings;
                core.history = persisted.history;
                if matches!(
                    core.settings.selected_model_kind,
                    TranscriptionModelKind::ParakeetCtc
                ) {
                    choose_fallback_model_selection(app.handle(), &mut core.settings);
                    core.status_message =
                        "Fell back to Parakeet TDT because CTC is not enabled in the stable runtime"
                            .to_string();
                }
                core.sources = enumerate_sources();
                core.model_status = current_model_status(app.handle(), &core.settings);
                core.parakeet_model_status = built_in_parakeet_status(app.handle());
            }
            if prune_history_audio(app.handle(), &shared) {
                let _ = save_persisted_state(app.handle(), &shared);
            }

            create_tray_icon(app.handle())?;
            create_indicator_window(app.handle())?;

            if let Err(error) = app.autolaunch().enable() {
                let mut core = shared.lock();
                core.error_message = Some(format!("Couldn't enable launch at login: {error}"));
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                let state_for_shortcuts = shared.clone();
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            let shortcut_text = shortcut.to_string();
                            let (hold_shortcut, toggle_shortcut) = {
                                let core = state_for_shortcuts.lock();
                                (
                                    normalize_shortcut(&core.settings.hold_shortcut),
                                    normalize_shortcut(&core.settings.toggle_shortcut),
                                )
                            };
                            let shortcut_text = normalize_shortcut(&shortcut_text);
                            let cancel_shortcut = normalize_shortcut(CANCEL_SHORTCUT);

                            if shortcut_text == cancel_shortcut && matches!(event.state, ShortcutState::Pressed) {
                                let _ = cancel_current_operation(app, &state_for_shortcuts);
                                return;
                            }

                            if shortcut_text == hold_shortcut {
                                match event.state {
                                    ShortcutState::Pressed => {
                                        let _ = begin_recording(app, &state_for_shortcuts, RecordingMode::Hold);
                                    }
                                    ShortcutState::Released => {
                                        let phase = {
                                            let core = state_for_shortcuts.lock();
                                            core.phase.clone()
                                        };
                                        if matches!(phase, AppPhase::Recording) {
                                            let _ = stop_recording(app, &state_for_shortcuts);
                                        }
                                    }
                                }
                                return;
                            }

                            if shortcut_text == toggle_shortcut && matches!(event.state, ShortcutState::Pressed) {
                                let phase = {
                                    let core = state_for_shortcuts.lock();
                                    core.phase.clone()
                                };

                                if matches!(phase, AppPhase::Recording) {
                                    let _ = stop_recording(app, &state_for_shortcuts);
                                } else {
                                    let _ = begin_recording(app, &state_for_shortcuts, RecordingMode::Toggle);
                                }
                            }
                        })
                        .build(),
                )?;

                if let Err(error) = register_shortcuts(app.handle(), &shared) {
                    {
                        let mut core = shared.lock();
                        core.settings = Settings::default();
                        core.shortcuts_active = false;
                        core.shortcut_message = "Falling back to default shortcuts".to_string();
                        core.status_message = format!("Shortcut defaults were restored: {error}");
                    }
                    if let Err(retry_error) = register_shortcuts(app.handle(), &shared) {
                        let mut core = shared.lock();
                        core.shortcuts_active = false;
                        core.shortcut_message = "Global shortcuts are unavailable".to_string();
                        core.status_message = "Running without global shortcuts".to_string();
                        core.error_message = Some(retry_error.to_string());
                    } else {
                        let _ = save_persisted_state(app.handle(), &shared);
                    }
                }
            }

            if launched_in_background() {
                hide_main_window(app.handle());
            } else {
                show_main_window(app.handle());
            }

            emit_snapshot(app.handle(), &shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            refresh_devices,
            inspect_model_path,
            install_catalog_model,
            download_catalog_model,
            remove_catalog_model,
            update_settings_command,
            add_cleanup_term,
            remove_cleanup_term,
            restore_default_cleanup_terms,
            clear_error_message_command,
            remove_history_item,
            start_manual_recording,
            stop_manual_recording,
            transcribe_media_file_command,
            cancel_current_operation_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        cleanup_transcript_text, common_prefix_len, default_cleanup_terms,
        live_preview_text, measure_overlay_levels, normalize_cleanup_term,
        normalize_cleanup_terms,
    };

    #[test]
    fn cleanup_transcript_removes_common_fillers() {
        let cleaned = cleanup_transcript_text(
            "Um, I think uh this should work.",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(cleaned, "I think this should work.");
    }

    #[test]
    fn cleanup_transcript_removes_multi_word_fillers() {
        let cleaned = cleanup_transcript_text(
            "You know I think this is fine.",
            true,
            &[String::from("you know")],
        );

        assert_eq!(cleaned, "I think this is fine.");
    }

    #[test]
    fn normalize_cleanup_terms_deduplicates_and_trims() {
        let normalized = normalize_cleanup_terms(&[
            String::from(" um "),
            String::from("UM"),
            String::from("you   know"),
        ]);

        assert_eq!(
            normalized,
            vec![String::from("um"), String::from("you know")]
        );
        assert_eq!(normalize_cleanup_term("   "), None);
    }

    #[test]
    fn measure_overlay_levels_stays_still_for_silence() {
        let silence = vec![0.0f32; 2_048];
        let levels = measure_overlay_levels(&silence, 16_000);

        assert!(levels.iter().all(|level| *level == 0.0));
    }

    #[test]
    fn measure_overlay_levels_reacts_to_spoken_energy() {
        let voiced = (0..2_048)
            .map(|index| {
                let phase = 2.0 * std::f32::consts::PI * 220.0 * index as f32 / 16_000.0;
                phase.sin() * 0.08
            })
            .collect::<Vec<_>>();
        let levels = measure_overlay_levels(&voiced, 16_000);

        assert!(levels.iter().any(|level| *level > 0.05));
    }

    #[test]
    fn live_preview_text_keeps_recent_lines() {
        let preview = live_preview_text(
            "first line of text\nsecond line with more words\nthird line stays visible\nfourth line is newest",
            true,
            &default_cleanup_terms(),
        );

        assert_eq!(
            preview,
            "second line with more words\nthird line stays visible\nfourth line is newest"
        );
    }

    #[test]
    fn common_prefix_len_ignores_case_and_terminal_punctuation() {
        let left = vec![
            String::from("Hello,"),
            String::from("world!"),
            String::from("Again"),
        ];
        let right = vec![
            String::from("hello"),
            String::from("world"),
            String::from("different"),
        ];

        assert_eq!(common_prefix_len(&left, &right), 2);
    }
}

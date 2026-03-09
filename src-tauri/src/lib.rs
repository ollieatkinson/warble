pub mod parakeet;
mod platform;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig, SupportedStreamConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use uuid::Uuid;

const EVENT_SNAPSHOT: &str = "transcribed://snapshot";
const PERSISTED_STATE_FILE: &str = "state.json";
const HOLD_MIN_DURATION_MS: u64 = 250;
const HISTORY_LIMIT: usize = 50;
const DEFAULT_HOLD_SHORTCUT: &str = "F8";
const DEFAULT_TOGGLE_SHORTCUT: &str = "F9";
const LEGACY_HOLD_SHORTCUT: &str = "Ctrl+Alt+Space";
const LEGACY_TOGGLE_SHORTCUT: &str = "Ctrl+Alt+Shift+Space";
const INDICATOR_WIDTH: i32 = 420;
const INDICATOR_HEIGHT: i32 = 108;
const INDICATOR_MARGIN: i32 = 24;
const LIVE_METER_INTERVAL_MS: u64 = 75;
const LIVE_METER_WINDOW_MS: u64 = 700;
const LIVE_METER_BAR_COUNT: usize = 12;
const BACKGROUND_ARG: &str = "--background";
const TRAY_ID: &str = "main-tray";
const TRAY_SHOW_ID: &str = "tray-show";
const TRAY_HIDE_ID: &str = "tray-hide";
const TRAY_QUIT_ID: &str = "tray-quit";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum RecordingMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
enum AppPhase {
    Idle,
    Recording,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModelStatus {
    Ready,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum OverlayPosition {
    BottomCenter,
    BottomLeft,
    BottomRight,
    Caret,
}

impl Default for OverlayPosition {
    fn default() -> Self {
        Self::BottomCenter
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum OverlayAnimationStyle {
    Spectrum,
    Waveform,
}

impl Default for OverlayAnimationStyle {
    fn default() -> Self {
        Self::Spectrum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    hold_shortcut: String,
    toggle_shortcut: String,
    selected_source_id: Option<String>,
    auto_paste: bool,
    overlay_position: OverlayPosition,
    overlay_animation_style: OverlayAnimationStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hold_shortcut: DEFAULT_HOLD_SHORTCUT.to_string(),
            toggle_shortcut: DEFAULT_TOGGLE_SHORTCUT.to_string(),
            selected_source_id: None,
            auto_paste: true,
            overlay_position: OverlayPosition::BottomCenter,
            overlay_animation_style: OverlayAnimationStyle::Spectrum,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryItem {
    id: String,
    text: String,
    created_at: String,
    source_name: String,
    mode: RecordingMode,
    duration_ms: u64,
    pasted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedState {
    settings: Settings,
    history: Vec<HistoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsUpdate {
    hold_shortcut: Option<String>,
    toggle_shortcut: Option<String>,
    selected_source_id: Option<String>,
    auto_paste: Option<bool>,
    overlay_position: Option<OverlayPosition>,
    overlay_animation_style: Option<OverlayAnimationStyle>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceInfo {
    id: String,
    name: String,
    sample_rate: u32,
    channels: u16,
    is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OverlaySnapshot {
    visible: bool,
    title: String,
    levels: Vec<f32>,
    anchor: Option<platform::CaretAnchor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    phase: AppPhase,
    settings: Settings,
    sources: Vec<SourceInfo>,
    history: Vec<HistoryItem>,
    model_status: ModelStatus,
    shortcuts_active: bool,
    shortcut_message: String,
    status_message: String,
    error_message: Option<String>,
    overlay: OverlaySnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelPathInspection {
    name: String,
    path: String,
    compatible: bool,
    ready: bool,
}

struct RecordingSession {
    stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: usize,
    started_at: Instant,
    mode: RecordingMode,
    source_name: String,
    anchor: Option<platform::CaretAnchor>,
}

#[derive(Debug)]
struct StartRecordingResponse {
    source_id: String,
    source_name: String,
    preview_buffer: Arc<Mutex<Vec<f32>>>,
    preview_sample_rate: u32,
}

enum RecorderRequest {
    Start {
        selected_source_id: Option<String>,
        mode: RecordingMode,
        anchor: Option<platform::CaretAnchor>,
        response: mpsc::Sender<Result<StartRecordingResponse, String>>,
    },
    Stop {
        response: mpsc::Sender<Result<Option<CompletedRecording>, String>>,
    },
}

#[derive(Debug)]
struct AppCore {
    settings: Settings,
    history: Vec<HistoryItem>,
    sources: Vec<SourceInfo>,
    phase: AppPhase,
    status_message: String,
    shortcuts_active: bool,
    shortcut_message: String,
    error_message: Option<String>,
    model_status: ModelStatus,
    overlay: OverlaySnapshot,
}

impl AppCore {
    fn new(settings: Settings, history: Vec<HistoryItem>) -> Self {
        Self {
            settings,
            history,
            sources: Vec::new(),
            phase: AppPhase::Idle,
            status_message: "Ready".to_string(),
            shortcuts_active: false,
            shortcut_message: "Checking global shortcuts".to_string(),
            error_message: None,
            model_status: ModelStatus::Missing,
            overlay: OverlaySnapshot {
                visible: false,
                title: String::new(),
                levels: default_overlay_levels(),
                anchor: None,
            },
        }
    }
}

#[derive(Clone)]
struct SharedState(Arc<Mutex<AppCore>>);

impl SharedState {
    fn new(core: AppCore) -> Self {
        Self(Arc::new(Mutex::new(core)))
    }

    fn lock(&self) -> MutexGuard<'_, AppCore> {
        self.0.lock().expect("shared state poisoned")
    }
}

#[derive(Clone)]
struct RecorderHandle {
    sender: mpsc::Sender<RecorderRequest>,
}

#[derive(Clone, Default)]
struct TranscriberHandle(Arc<Mutex<Option<parakeet::ParakeetTdt>>>);

impl TranscriberHandle {
    fn lock(&self) -> MutexGuard<'_, Option<parakeet::ParakeetTdt>> {
        self.0.lock().expect("transcriber state poisoned")
    }
}

#[derive(Clone, Default)]
struct PreviewControl(Arc<AtomicU64>);

impl PreviewControl {
    fn next_generation(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn current_generation(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
struct CompletedRecording {
    samples: Vec<f32>,
    duration_ms: u64,
    source_name: String,
    mode: RecordingMode,
    anchor: Option<platform::CaretAnchor>,
}

fn normalize_shortcut(shortcut: &str) -> String {
    shortcut.to_ascii_lowercase().replace(' ', "")
}

fn launched_in_background() -> bool {
    std::env::args().any(|arg| arg == BACKGROUND_ARG)
}

fn migrate_legacy_shortcuts(settings: &mut Settings) {
    if normalize_shortcut(&settings.hold_shortcut) == normalize_shortcut(LEGACY_HOLD_SHORTCUT)
        && normalize_shortcut(&settings.toggle_shortcut) == normalize_shortcut(LEGACY_TOGGLE_SHORTCUT)
    {
        settings.hold_shortcut = DEFAULT_HOLD_SHORTCUT.to_string();
        settings.toggle_shortcut = DEFAULT_TOGGLE_SHORTCUT.to_string();
    }
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory")?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn persisted_state_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app_data_dir(app)?.join(PERSISTED_STATE_FILE))
}

fn model_root_dir(app: &AppHandle) -> Result<PathBuf> {
    let dir = app_data_dir(app)?.join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn detect_model_status(app: &AppHandle) -> ModelStatus {
    model_root_dir(app)
        .ok()
        .filter(|root| parakeet::model_ready_at(root))
        .map(|_| ModelStatus::Ready)
        .unwrap_or(ModelStatus::Missing)
}

fn emit_snapshot(app: &AppHandle, shared: &SharedState) {
    let snapshot = {
        let core = shared.lock();
        Snapshot {
            phase: core.phase.clone(),
            settings: core.settings.clone(),
            sources: core.sources.clone(),
            history: core.history.clone(),
            model_status: core.model_status.clone(),
            shortcuts_active: core.shortcuts_active,
            shortcut_message: core.shortcut_message.clone(),
            status_message: core.status_message.clone(),
            error_message: core.error_message.clone(),
            overlay: core.overlay.clone(),
        }
    };

    let _ = app.emit(EVENT_SNAPSHOT, snapshot);
}

fn save_persisted_state(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let persisted = {
        let core = shared.lock();
        PersistedState {
            settings: core.settings.clone(),
            history: core.history.clone(),
        }
    };

    let path = persisted_state_path(app)?;
    fs::write(path, serde_json::to_vec_pretty(&persisted)?)?;
    Ok(())
}

fn load_persisted_state(app: &AppHandle) -> PersistedState {
    let Some(path) = persisted_state_path(app).ok() else {
        return PersistedState {
            settings: Settings::default(),
            history: Vec::new(),
        };
    };

    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<PersistedState>(&content) {
            let mut parsed = parsed;
            migrate_legacy_shortcuts(&mut parsed.settings);
            return parsed;
        }
    }

    PersistedState {
        settings: Settings::default(),
        history: Vec::new(),
    }
}

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

    {
        let mut core = shared.lock();
        core.phase = AppPhase::Recording;
        core.status_message = format!("Recording from {}", started.source_name);
        core.error_message = None;
        core.overlay.visible = true;
        core.overlay.title = "Listening".to_string();
        core.overlay.levels = default_overlay_levels();
        core.overlay.anchor = anchor;
        core.settings.selected_source_id = Some(started.source_id);
        core.sources = enumerate_sources();
    }

    let _ = save_persisted_state(app, shared);
    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
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
        duration_ms,
        source_name: session.source_name,
        mode: session.mode,
        anchor: session.anchor,
    }))
}

fn transcribe_audio(
    app: &AppHandle,
    transcriber: &TranscriberHandle,
    audio: &[f32],
) -> Result<String> {
    let mut guard = transcriber.lock();
    if guard.is_none() {
        let model_root = model_root_dir(app)?;
        *guard = Some(parakeet::ParakeetTdt::load(&model_root)?);
    }

    guard
        .as_mut()
        .expect("transcriber initialized")
        .transcribe_audio(audio)
}

fn default_overlay_levels() -> Vec<f32> {
    vec![0.14; LIVE_METER_BAR_COUNT]
}

fn measure_overlay_levels(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return default_overlay_levels();
    }

    let mut levels = default_overlay_levels();
    let chunk_len = samples.len().max(LIVE_METER_BAR_COUNT) / LIVE_METER_BAR_COUNT;

    for (index, level) in levels.iter_mut().enumerate() {
        let start = index * chunk_len;
        if start >= samples.len() {
            break;
        }

        let end = (start + chunk_len).min(samples.len());
        let slice = &samples[start..end];
        if slice.is_empty() {
            continue;
        }

        let rms = (slice.iter().map(|sample| sample * sample).sum::<f32>() / slice.len() as f32)
            .sqrt();
        let normalized = ((rms - 0.01).max(0.0) / 0.18).clamp(0.0, 1.0).sqrt();
        *level = 0.12 + normalized * 0.88;
    }

    levels
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

fn indicator_origin(app: &AppHandle, overlay: &OverlaySnapshot, position: OverlayPosition) -> Option<PhysicalPosition<i32>> {
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
            let max_x = left + width - INDICATOR_WIDTH - INDICATOR_MARGIN;
            let min_y = top + INDICATOR_MARGIN;
            let max_y = top + height - INDICATOR_HEIGHT - INDICATOR_MARGIN;

            return Some(PhysicalPosition::new(
                anchor.x.clamp(min_x, max_x.max(min_x)),
                anchor.y.clamp(min_y, max_y.max(min_y)),
            ));
        }
    }

    let x = match position {
        OverlayPosition::BottomLeft => left + INDICATOR_MARGIN,
        OverlayPosition::BottomRight => left + width - INDICATOR_WIDTH - INDICATOR_MARGIN,
        OverlayPosition::BottomCenter | OverlayPosition::Caret => {
            left + (width - INDICATOR_WIDTH) / 2
        }
    };
    let y = top + height - INDICATOR_HEIGHT - INDICATOR_MARGIN;

    Some(PhysicalPosition::new(x.max(left), y.max(top)))
}

fn register_shortcuts(app: &AppHandle, shared: &SharedState) -> Result<()> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let settings = {
            let core = shared.lock();
            core.settings.clone()
        };
        if normalize_shortcut(&settings.hold_shortcut) == normalize_shortcut(&settings.toggle_shortcut) {
            return Err(anyhow!("Hold and toggle shortcuts must be different"));
        }

        app.global_shortcut().unregister_all()?;
        app.global_shortcut().register(settings.hold_shortcut.as_str())?;
        app.global_shortcut().register(settings.toggle_shortcut.as_str())?;

        let mut core = shared.lock();
        core.shortcuts_active = true;
        core.shortcut_message = format!(
            "Listening for {} and {}",
            settings.hold_shortcut, settings.toggle_shortcut
        );
        core.error_message = None;
    }

    Ok(())
}

fn update_indicator_window(app: &AppHandle, shared: &SharedState) {
    let (overlay, overlay_position) = {
        let core = shared.lock();
        (core.overlay.clone(), core.settings.overlay_position)
    };

    let Some(window) = app.get_webview_window("indicator") else {
        return;
    };

    if overlay.visible {
        if let Some(position) = indicator_origin(app, &overlay, overlay_position) {
            let _ = window.set_position(Position::Physical(position));
        }
        let _ = window.show();
    } else {
        let _ = window.hide();
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
                measure_overlay_levels(&samples[start..])
            };

            if preview_control.current_generation() != generation {
                break;
            }

            if levels
                .iter()
                .zip(last_levels.iter())
                .all(|(next, previous)| (next - previous).abs() < 0.025)
            {
                continue;
            }
            last_levels = levels.clone();

            {
                let mut core = shared.lock();
                if !matches!(core.phase, AppPhase::Recording) {
                    break;
                }
                core.overlay.levels = levels;
            }

            emit_snapshot(&app, &shared);
        }
    });
}

fn complete_transcription(
    app: AppHandle,
    shared: SharedState,
    transcriber: TranscriberHandle,
    completed: CompletedRecording,
) {
    std::thread::spawn(move || {
        let result = transcribe_audio(&app, &transcriber, &completed.samples);

        match result {
            Ok(text) => {
                let text = text.trim().to_string();
                if text.is_empty() {
                    let mut core = shared.lock();
                    core.phase = AppPhase::Idle;
                    core.status_message = "Nothing intelligible was detected".to_string();
                    core.error_message = None;
                    core.model_status = detect_model_status(&app);
                    core.overlay.visible = false;
                    core.overlay.levels = default_overlay_levels();
                    drop(core);
                    update_indicator_window(&app, &shared);
                    emit_snapshot(&app, &shared);
                    return;
                }

                let pasted = {
                    let settings = {
                        let core = shared.lock();
                        core.settings.clone()
                    };

                    if settings.auto_paste {
                        platform::paste_text(&text).is_ok()
                    } else {
                        false
                    }
                };

                {
                    let mut core = shared.lock();
                    core.history.insert(
                        0,
                        HistoryItem {
                            id: Uuid::new_v4().to_string(),
                            text: text.clone(),
                            created_at: Utc::now().to_rfc3339(),
                            source_name: completed.source_name.clone(),
                            mode: completed.mode.clone(),
                            duration_ms: completed.duration_ms,
                            pasted,
                        },
                    );
                    core.history.truncate(HISTORY_LIMIT);
                    core.phase = AppPhase::Idle;
                    core.status_message = if pasted {
                        "Transcribed in Rust and pasted".to_string()
                    } else {
                        "Transcribed in Rust".to_string()
                    };
                    core.error_message = None;
                    core.model_status = detect_model_status(&app);
                    core.overlay.visible = false;
                    core.overlay.levels = default_overlay_levels();
                }

                let _ = save_persisted_state(&app, &shared);
            }
            Err(error) => {
                let mut core = shared.lock();
                core.phase = AppPhase::Error;
                core.status_message = "Transcription failed".to_string();
                core.error_message = Some(error.to_string());
                core.model_status = detect_model_status(&app);
                core.overlay.visible = false;
                core.overlay.levels = default_overlay_levels();
            }
        }

        update_indicator_window(&app, &shared);
        emit_snapshot(&app, &shared);
    });
}

fn stop_recording(app: &AppHandle, shared: &SharedState) -> Result<()> {
    let recorder = app.state::<RecorderHandle>();
    let transcriber = app.state::<TranscriberHandle>();
    let preview_control = app.state::<PreviewControl>();
    preview_control.next_generation();
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
            core.overlay.visible = false;
            core.overlay.levels = default_overlay_levels();
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
        core.overlay.visible = true;
        core.overlay.title = "Transcribing".to_string();
        core.overlay.anchor = completed.anchor;
    }

    update_indicator_window(app, shared);
    emit_snapshot(app, shared);
    complete_transcription(app.clone(), shared.clone(), transcriber.inner().clone(), completed);
    Ok(())
}

fn create_indicator_window(app: &AppHandle) -> Result<()> {
    if app.get_webview_window("indicator").is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        "indicator",
        WebviewUrl::App("index.html?indicator=1".into()),
    )
    .title("Transcribed Indicator")
    .transparent(true)
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(true)
    .visible(false)
    .focused(false)
    .inner_size(INDICATOR_WIDTH as f64, INDICATOR_HEIGHT as f64)
    .build()?;

    let _ = window.set_focusable(false);
    let _ = window.set_ignore_cursor_events(true);
    Ok(())
}

fn refresh_sources(app: &AppHandle, shared: &SharedState) {
    {
        let mut core = shared.lock();
        core.sources = enumerate_sources();
        core.model_status = detect_model_status(app);
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
    let core = shared.lock();
    Ok(Snapshot {
        phase: core.phase.clone(),
        settings: core.settings.clone(),
        sources: core.sources.clone(),
        history: core.history.clone(),
        model_status: core.model_status.clone(),
        shortcuts_active: core.shortcuts_active,
        shortcut_message: core.shortcut_message.clone(),
        status_message: core.status_message.clone(),
        error_message: core.error_message.clone(),
        overlay: core.overlay.clone(),
    })
}

#[tauri::command]
fn refresh_devices(app: AppHandle, shared: tauri::State<'_, SharedState>) {
    refresh_sources(&app, &shared);
}

#[tauri::command]
fn inspect_model_path(path: String) -> Result<ModelPathInspection, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Enter a local model folder path.".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| format!("Folder not found: {trimmed}"))?;

    if !canonical.is_dir() {
        return Err("That path is not a folder.".to_string());
    }

    let direct_ready = parakeet::REQUIRED_MODEL_FILES
        .iter()
        .all(|filename| canonical.join(filename).exists());
    let nested_dir = canonical.join(parakeet::MODEL_ID);
    let nested_ready = nested_dir.is_dir()
        && parakeet::REQUIRED_MODEL_FILES
            .iter()
            .all(|filename| nested_dir.join(filename).exists());
    let resolved_dir = if nested_ready { nested_dir } else { canonical.clone() };
    let name = resolved_dir
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Custom model")
        .to_string();
    let ready = direct_ready || nested_ready;

    Ok(ModelPathInspection {
        name,
        path: canonical.display().to_string(),
        compatible: ready,
        ready,
    })
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
        if let Some(overlay_position) = update.overlay_position {
            core.settings.overlay_position = overlay_position;
        }
        if let Some(overlay_animation_style) = update.overlay_animation_style {
            core.settings.overlay_animation_style = overlay_animation_style;
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
    save_persisted_state(&app, &shared).map_err(|error| error.to_string())?;
    update_indicator_window(&app, &shared);
    refresh_sources(&app, &shared);
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
                core.sources = enumerate_sources();
                core.model_status = detect_model_status(app.handle());
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

                            if shortcut_text == hold_shortcut {
                                match event.state {
                                    ShortcutState::Pressed => {
                                        let _ = begin_recording(app, &state_for_shortcuts, RecordingMode::Hold);
                                    }
                                    ShortcutState::Released => {
                                        let _ = stop_recording(app, &state_for_shortcuts);
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
            update_settings_command,
            start_manual_recording,
            stop_manual_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use cpal::Stream;

use crate::constants::{
    DEFAULT_HOLD_SHORTCUT, DEFAULT_PASTE_LAST_SHORTCUT, DEFAULT_TOGGLE_SHORTCUT,
};
use crate::overlay::default_overlay_levels;
use crate::parakeet;
use crate::platform;
use crate::storage::path_if_not_empty;
use crate::system::detect_system_profile;
use crate::transcript::{
    default_cleanup_terms, normalize_cleanup_terms, normalize_replacement_rules,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RecordingMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AppPhase {
    Idle,
    Recording,
    Transcribing,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ModelStatus {
    Ready,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum InferenceProvider {
    #[default]
    Cpu,
    Coreml,
    Directml,
    Webgpu,
}

impl InferenceProvider {
    pub(crate) fn is_accelerated(self) -> bool {
        !matches!(self, Self::Cpu)
    }
}

impl fmt::Display for InferenceProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => f.write_str("CPU"),
            Self::Coreml => f.write_str("CoreML"),
            Self::Directml => f.write_str("DirectML"),
            Self::Webgpu => f.write_str("WebGPU"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MacosModelRuntimePreference {
    #[default]
    Cpu,
    Coreml,
    Webgpu,
}

impl From<MacosModelRuntimePreference> for InferenceProvider {
    fn from(value: MacosModelRuntimePreference) -> Self {
        match value {
            MacosModelRuntimePreference::Cpu => Self::Cpu,
            MacosModelRuntimePreference::Coreml => Self::Coreml,
            MacosModelRuntimePreference::Webgpu => Self::Webgpu,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CaptureSourceKind {
    #[default]
    Microphone,
    File,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum TranscriptionModelKind {
    Parakeet,
    ParakeetCtc,
}

impl Default for TranscriptionModelKind {
    fn default() -> Self {
        Self::Parakeet
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OverlayPosition {
    TopCenter,
    TopLeft,
    TopRight,
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
pub(crate) enum OverlayAnimationStyle {
    Spectrum,
    Waveform,
    Radial,
}

impl Default for OverlayAnimationStyle {
    fn default() -> Self {
        Self::Spectrum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LivePreviewModelPreference {
    Auto,
    NemotronStreaming,
    ParakeetEou,
}

impl Default for LivePreviewModelPreference {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LiveTranscriptWidth {
    Compact,
    Balanced,
    Wide,
}

impl Default for LiveTranscriptWidth {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LiveTranscriptLines {
    One,
    Two,
    Three,
}

impl Default for LiveTranscriptLines {
    fn default() -> Self {
        Self::One
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ColorTheme {
    System,
    Light,
    Dark,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AudioRetentionPolicy {
    OneDay,
    SevenDays,
    ThirtyDays,
}

impl Default for AudioRetentionPolicy {
    fn default() -> Self {
        Self::OneDay
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Settings {
    pub(crate) hold_shortcut: String,
    pub(crate) toggle_shortcut: String,
    pub(crate) paste_last_shortcut: String,
    pub(crate) selected_source_id: Option<String>,
    pub(crate) auto_paste: bool,
    pub(crate) selected_model_id: String,
    pub(crate) selected_model_kind: TranscriptionModelKind,
    pub(crate) selected_model_path: Option<String>,
    pub(crate) installed_model_paths: BTreeMap<String, String>,
    pub(crate) macos_model_runtime_preferences: BTreeMap<String, MacosModelRuntimePreference>,
    pub(crate) cleanup_enabled: bool,
    pub(crate) cleanup_terms: Vec<String>,
    pub(crate) replacement_rules: Vec<ReplacementRule>,
    pub(crate) audio_retention_policy: AudioRetentionPolicy,
    pub(crate) overlay_position: OverlayPosition,
    pub(crate) overlay_animation_style: OverlayAnimationStyle,
    pub(crate) live_preview_model: LivePreviewModelPreference,
    pub(crate) live_transcript_width: LiveTranscriptWidth,
    pub(crate) live_transcript_lines: LiveTranscriptLines,
    pub(crate) show_recording_timer: bool,
    pub(crate) show_live_transcription: bool,
    pub(crate) color_theme: ColorTheme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hold_shortcut: DEFAULT_HOLD_SHORTCUT.to_string(),
            toggle_shortcut: DEFAULT_TOGGLE_SHORTCUT.to_string(),
            paste_last_shortcut: DEFAULT_PASTE_LAST_SHORTCUT.to_string(),
            selected_source_id: None,
            auto_paste: true,
            selected_model_id: "parakeet".to_string(),
            selected_model_kind: TranscriptionModelKind::Parakeet,
            selected_model_path: None,
            installed_model_paths: BTreeMap::new(),
            macos_model_runtime_preferences: BTreeMap::new(),
            cleanup_enabled: true,
            cleanup_terms: default_cleanup_terms(),
            replacement_rules: Vec::new(),
            audio_retention_policy: AudioRetentionPolicy::OneDay,
            overlay_position: OverlayPosition::BottomCenter,
            overlay_animation_style: OverlayAnimationStyle::Spectrum,
            live_preview_model: LivePreviewModelPreference::Auto,
            live_transcript_width: LiveTranscriptWidth::Balanced,
            live_transcript_lines: LiveTranscriptLines::One,
            show_recording_timer: false,
            show_live_transcription: false,
            color_theme: ColorTheme::System,
        }
    }
}

impl Settings {
    pub(crate) fn macos_runtime_preference_for_model(
        &self,
        model_id: &str,
    ) -> MacosModelRuntimePreference {
        self.macos_model_runtime_preferences
            .get(model_id)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn sanitize_overlay_position(&mut self) -> bool {
        false
    }

    pub(crate) fn apply_update(
        &mut self,
        update: &SettingsUpdate,
    ) -> bool {
        let hold_shortcut = update
            .hold_shortcut
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let toggle_shortcut = update
            .toggle_shortcut
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let paste_last_shortcut = update
            .paste_last_shortcut
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(hold_shortcut) = hold_shortcut {
            self.hold_shortcut = hold_shortcut;
        }
        if let Some(toggle_shortcut) = toggle_shortcut {
            self.toggle_shortcut = toggle_shortcut;
        }
        if let Some(paste_last_shortcut) = paste_last_shortcut {
            self.paste_last_shortcut = paste_last_shortcut;
        }
        if let Some(selected_source_id) = update.selected_source_id.as_ref() {
            self.selected_source_id = Some(selected_source_id.clone());
        }
        if let Some(auto_paste) = update.auto_paste {
            self.auto_paste = auto_paste;
        }
        if let Some(selected_model_id) = update.selected_model_id.as_ref() {
            self.selected_model_id = selected_model_id.clone();
        }
        if let Some(selected_model_kind) = update.selected_model_kind {
            self.selected_model_kind = selected_model_kind;
        }
        if let Some(selected_model_path) = update.selected_model_path.as_ref() {
            self.selected_model_path = path_if_not_empty(selected_model_path.clone());
        }
        if let Some(macos_model_runtime_preferences) =
            update.macos_model_runtime_preferences.as_ref()
        {
            self.macos_model_runtime_preferences = macos_model_runtime_preferences.clone();
        }
        if let Some(cleanup_enabled) = update.cleanup_enabled {
            self.cleanup_enabled = cleanup_enabled;
        }
        if let Some(cleanup_terms) = update.cleanup_terms.as_ref() {
            self.cleanup_terms = normalize_cleanup_terms(cleanup_terms);
        }
        if let Some(replacement_rules) = update.replacement_rules.as_ref() {
            self.replacement_rules = normalize_replacement_rules(replacement_rules);
        }
        if let Some(audio_retention_policy) = update.audio_retention_policy.as_ref() {
            self.audio_retention_policy = audio_retention_policy.clone();
        }
        if let Some(overlay_position) = update.overlay_position.as_ref() {
            self.overlay_position = overlay_position.clone();
        }
        if let Some(overlay_animation_style) = update.overlay_animation_style.as_ref() {
            self.overlay_animation_style = overlay_animation_style.clone();
        }
        if let Some(live_preview_model) = update.live_preview_model.as_ref() {
            self.live_preview_model = live_preview_model.clone();
        }
        if let Some(live_transcript_width) = update.live_transcript_width.as_ref() {
            self.live_transcript_width = live_transcript_width.clone();
        }
        if let Some(live_transcript_lines) = update.live_transcript_lines.as_ref() {
            self.live_transcript_lines = live_transcript_lines.clone();
        }
        if let Some(show_recording_timer) = update.show_recording_timer {
            self.show_recording_timer = show_recording_timer;
        }

        if let Some(color_theme) = update.color_theme {
            self.color_theme = color_theme;
        }

        let hides_live_transcription = matches!(update.show_live_transcription, Some(false));
        if let Some(show_live_transcription) = update.show_live_transcription {
            self.show_live_transcription = show_live_transcription;
        }

        hides_live_transcription
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReplacementRule {
    pub(crate) id: String,
    pub(crate) variants: Vec<String>,
    pub(crate) replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct HistoryCaptureDetails {
    pub(crate) source_kind: CaptureSourceKind,
    pub(crate) model_id: String,
    pub(crate) model_name: String,
    pub(crate) inference_provider: InferenceProvider,
    pub(crate) input_sample_rate: u32,
    pub(crate) input_channels: u16,
    pub(crate) transcription_sample_rate: u32,
}

impl Default for HistoryCaptureDetails {
    fn default() -> Self {
        Self {
            source_kind: CaptureSourceKind::Microphone,
            model_id: String::new(),
            model_name: String::new(),
            inference_provider: InferenceProvider::Cpu,
            input_sample_rate: 0,
            input_channels: 0,
            transcription_sample_rate: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryItem {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) created_at: String,
    pub(crate) source_name: String,
    pub(crate) mode: RecordingMode,
    pub(crate) duration_ms: u64,
    pub(crate) pasted: bool,
    pub(crate) audio_path: Option<String>,
    #[serde(default)]
    pub(crate) capture: HistoryCaptureDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersistedState {
    pub(crate) settings: Settings,
    pub(crate) history: Vec<HistoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsUpdate {
    pub(crate) hold_shortcut: Option<String>,
    pub(crate) toggle_shortcut: Option<String>,
    pub(crate) paste_last_shortcut: Option<String>,
    pub(crate) selected_source_id: Option<String>,
    pub(crate) auto_paste: Option<bool>,
    pub(crate) selected_model_id: Option<String>,
    pub(crate) selected_model_kind: Option<TranscriptionModelKind>,
    pub(crate) selected_model_path: Option<Option<String>>,
    pub(crate) macos_model_runtime_preferences:
        Option<BTreeMap<String, MacosModelRuntimePreference>>,
    pub(crate) cleanup_enabled: Option<bool>,
    pub(crate) cleanup_terms: Option<Vec<String>>,
    pub(crate) replacement_rules: Option<Vec<ReplacementRule>>,
    pub(crate) audio_retention_policy: Option<AudioRetentionPolicy>,
    pub(crate) overlay_position: Option<OverlayPosition>,
    pub(crate) overlay_animation_style: Option<OverlayAnimationStyle>,
    pub(crate) live_preview_model: Option<LivePreviewModelPreference>,
    pub(crate) live_transcript_width: Option<LiveTranscriptWidth>,
    pub(crate) live_transcript_lines: Option<LiveTranscriptLines>,
    pub(crate) show_recording_timer: Option<bool>,
    pub(crate) show_live_transcription: Option<bool>,
    pub(crate) color_theme: Option<ColorTheme>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlaySnapshot {
    pub(crate) visible: bool,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) levels: Vec<f32>,
    pub(crate) elapsed_ms: u64,
    pub(crate) limit_ms: Option<u64>,
    pub(crate) anchor: Option<platform::CaretAnchor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewDiagnostics {
    pub(crate) backend: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) recent_events: Vec<String>,
    pub(crate) log_path: Option<String>,
}

impl Default for PreviewDiagnostics {
    fn default() -> Self {
        Self {
            backend: String::new(),
            status: "Idle".to_string(),
            detail: String::new(),
            recent_events: Vec::new(),
            log_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CaptureDiagnostics {
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) recent_events: Vec<String>,
    pub(crate) log_path: Option<String>,
    pub(crate) source_name: String,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) last_buffered_samples: usize,
}

impl Default for CaptureDiagnostics {
    fn default() -> Self {
        Self {
            status: "Idle".to_string(),
            detail: String::new(),
            recent_events: Vec::new(),
            log_path: None,
            source_name: String::new(),
            sample_rate: 0,
            channels: 0,
            last_buffered_samples: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelDownloadProgress {
    pub(crate) display_name: String,
    pub(crate) file_name: String,
    pub(crate) downloaded_bytes: u64,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) bytes_per_second: Option<u64>,
    pub(crate) seconds_remaining: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    pub(crate) phase: AppPhase,
    pub(crate) platform: platform::PlatformKind,
    pub(crate) auto_paste_support: platform::AutoPasteSupport,
    pub(crate) settings: Settings,
    pub(crate) last_transcript_available: bool,
    pub(crate) sources: Vec<SourceInfo>,
    pub(crate) history: Vec<HistoryItem>,
    pub(crate) model_status: ModelStatus,
    pub(crate) parakeet_model_status: ModelStatus,
    pub(crate) installed_model_sizes: BTreeMap<String, u64>,
    pub(crate) model_downloads: BTreeMap<String, ModelDownloadProgress>,
    pub(crate) system_profile: SystemProfile,
    pub(crate) shortcuts_active: bool,
    pub(crate) shortcut_message: String,
    pub(crate) status_message: String,
    pub(crate) error_message: Option<String>,
    pub(crate) preview_diagnostics: PreviewDiagnostics,
    pub(crate) capture_diagnostics: CaptureDiagnostics,
    pub(crate) overlay: OverlaySnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugLogs {
    pub(crate) capture: String,
    pub(crate) live_preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProfile {
    pub(crate) logical_cores: usize,
    pub(crate) total_memory_bytes: u64,
    pub(crate) gpu_name: Option<String>,
    pub(crate) gpu_memory_bytes: u64,
    pub(crate) supported_acceleration_providers: Vec<InferenceProvider>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelPathInspection {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) model_kind: TranscriptionModelKind,
    pub(crate) compatible: bool,
    pub(crate) ready: bool,
}

pub(crate) struct RecordingSession {
    pub(crate) stream: Stream,
    pub(crate) buffer: Arc<Mutex<Vec<f32>>>,
    pub(crate) sample_rate: u32,
    pub(crate) channels: usize,
    pub(crate) started_at: Instant,
    pub(crate) mode: RecordingMode,
    pub(crate) source_name: String,
    pub(crate) anchor: Option<platform::CaretAnchor>,
    pub(crate) stream_errors: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug)]
pub(crate) struct StartRecordingResponse {
    pub(crate) source_id: String,
    pub(crate) source_name: String,
    pub(crate) preview_buffer: Arc<Mutex<Vec<f32>>>,
    pub(crate) preview_sample_rate: u32,
    pub(crate) preview_channels: u16,
}

pub(crate) enum RecorderRequest {
    Start {
        selected_source_id: Option<String>,
        mode: RecordingMode,
        anchor: Option<platform::CaretAnchor>,
        response: mpsc::Sender<std::result::Result<StartRecordingResponse, String>>,
    },
    Stop {
        response: mpsc::Sender<std::result::Result<Option<CompletedRecording>, String>>,
    },
}

#[derive(Debug)]
pub(crate) struct AppCore {
    pub(crate) settings: Settings,
    pub(crate) history: Vec<HistoryItem>,
    pub(crate) last_transcript_text: Option<String>,
    pub(crate) sources: Vec<SourceInfo>,
    pub(crate) system_profile: SystemProfile,
    pub(crate) phase: AppPhase,
    pub(crate) status_message: String,
    pub(crate) shortcuts_active: bool,
    pub(crate) shortcut_message: String,
    pub(crate) error_message: Option<String>,
    pub(crate) preview_diagnostics: PreviewDiagnostics,
    pub(crate) capture_diagnostics: CaptureDiagnostics,
    pub(crate) model_status: ModelStatus,
    pub(crate) parakeet_model_status: ModelStatus,
    pub(crate) model_downloads: BTreeMap<String, ModelDownloadProgress>,
    pub(crate) overlay: OverlaySnapshot,
    pub(crate) indicator_window_size: Option<(i32, i32)>,
    pub(crate) recording_started_at: Option<Instant>,
}

impl AppCore {
    pub(crate) fn new(settings: Settings, history: Vec<HistoryItem>) -> Self {
        let last_transcript_text = history.first().map(|item| item.text.clone());
        Self {
            settings,
            history,
            last_transcript_text,
            sources: Vec::new(),
            system_profile: detect_system_profile(),
            phase: AppPhase::Idle,
            status_message: "Ready".to_string(),
            shortcuts_active: false,
            shortcut_message: "Checking global shortcuts".to_string(),
            error_message: None,
            preview_diagnostics: PreviewDiagnostics::default(),
            capture_diagnostics: CaptureDiagnostics::default(),
            model_status: ModelStatus::Missing,
            parakeet_model_status: ModelStatus::Missing,
            model_downloads: BTreeMap::new(),
            overlay: OverlaySnapshot {
                visible: false,
                title: String::new(),
                detail: String::new(),
                levels: default_overlay_levels(),
                elapsed_ms: 0,
                limit_ms: None,
                anchor: None,
            },
            indicator_window_size: None,
            recording_started_at: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SharedState(pub(crate) Arc<Mutex<AppCore>>);

impl SharedState {
    pub(crate) fn new(core: AppCore) -> Self {
        Self(Arc::new(Mutex::new(core)))
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, AppCore> {
        self.0.lock().expect("shared state poisoned")
    }
}

#[derive(Clone)]
pub(crate) struct RecorderHandle {
    pub(crate) sender: mpsc::Sender<RecorderRequest>,
}

#[derive(Clone, Default)]
pub(crate) struct TranscriberHandle(pub(crate) Arc<Mutex<TranscriberCache>>);

impl TranscriberHandle {
    pub(crate) fn lock(&self) -> MutexGuard<'_, TranscriberCache> {
        self.0.lock().expect("transcriber state poisoned")
    }
}

#[derive(Default)]
pub(crate) struct TranscriberCache {
    pub(crate) selected_key: Option<String>,
    pub(crate) engine: Option<TranscriberEngine>,
}

pub(crate) enum TranscriberEngine {
    Parakeet(parakeet::ParakeetTdt),
    ParakeetCtc(parakeet::ParakeetCtc),
}

#[derive(Clone, Default)]
pub(crate) struct PreviewControl(pub(crate) Arc<AtomicU64>);

impl PreviewControl {
    pub(crate) fn next_generation(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub(crate) fn current_generation(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub(crate) struct CompletedRecording {
    pub(crate) samples: Vec<f32>,
    pub(crate) captured_samples: Vec<f32>,
    pub(crate) captured_sample_rate: u32,
    pub(crate) captured_channels: u16,
    pub(crate) captured_sample_count: usize,
    pub(crate) stream_errors: Vec<String>,
    pub(crate) should_transcribe: bool,
    pub(crate) duration_ms: u64,
    pub(crate) source_name: String,
    pub(crate) mode: RecordingMode,
    pub(crate) anchor: Option<platform::CaretAnchor>,
}

#[derive(Default)]
pub(crate) struct PreviewStabilizer {
    pub(crate) last_partial: Option<Vec<String>>,
    pub(crate) stable_words: Vec<String>,
    pub(crate) divergence_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_values() {
        let settings = Settings::default();
        assert_eq!(settings.hold_shortcut, "F8");
        assert_eq!(settings.toggle_shortcut, "F9");
        assert_eq!(settings.paste_last_shortcut, "F10");
        assert!(settings.auto_paste);
        assert_eq!(settings.selected_model_id, "parakeet");
        assert!(settings.cleanup_enabled);
        assert!(settings.replacement_rules.is_empty());
        assert_eq!(settings.overlay_position, OverlayPosition::BottomCenter);
        assert_eq!(
            settings.overlay_animation_style,
            OverlayAnimationStyle::Spectrum
        );
        assert_eq!(
            settings.live_transcript_width,
            LiveTranscriptWidth::Balanced
        );
        assert_eq!(settings.live_transcript_lines, LiveTranscriptLines::One);
        assert!(!settings.show_recording_timer);
        assert!(!settings.show_live_transcription);
        assert_eq!(settings.color_theme, ColorTheme::System);
        assert!(settings.selected_source_id.is_none());
        assert!(settings.selected_model_path.is_none());
        assert!(settings.installed_model_paths.is_empty());
        assert!(settings.macos_model_runtime_preferences.is_empty());
    }

    #[test]
    fn settings_serialization_round_trip() {
        let mut settings = Settings::default();
        settings
            .macos_model_runtime_preferences
            .insert("parakeet".to_string(), MacosModelRuntimePreference::Coreml);
        settings.macos_model_runtime_preferences.insert(
            "nemotron-streaming".to_string(),
            MacosModelRuntimePreference::Webgpu,
        );
        let json = serde_json::to_string(&settings).expect("serialize");
        let deserialized: Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.hold_shortcut, settings.hold_shortcut);
        assert_eq!(deserialized.toggle_shortcut, settings.toggle_shortcut);
        assert_eq!(
            deserialized.paste_last_shortcut,
            settings.paste_last_shortcut
        );
        assert_eq!(deserialized.auto_paste, settings.auto_paste);
        assert_eq!(deserialized.selected_model_id, settings.selected_model_id);
        assert_eq!(deserialized.cleanup_enabled, settings.cleanup_enabled);
        assert_eq!(deserialized.replacement_rules, settings.replacement_rules);
        assert_eq!(
            deserialized.macos_model_runtime_preferences,
            settings.macos_model_runtime_preferences
        );
    }

    #[test]
    fn app_phase_variants_serialize() {
        let phases = vec![
            AppPhase::Idle,
            AppPhase::Recording,
            AppPhase::Transcribing,
            AppPhase::Error,
        ];
        for phase in phases {
            let json = serde_json::to_string(&phase).expect("serialize phase");
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn recording_mode_serde_round_trip() {
        let hold = RecordingMode::Hold;
        let toggle = RecordingMode::Toggle;
        let hold_json = serde_json::to_string(&hold).unwrap();
        let toggle_json = serde_json::to_string(&toggle).unwrap();
        assert_eq!(hold_json, "\"hold\"");
        assert_eq!(toggle_json, "\"toggle\"");
        let hold_back: RecordingMode = serde_json::from_str(&hold_json).unwrap();
        let toggle_back: RecordingMode = serde_json::from_str(&toggle_json).unwrap();
        assert_eq!(hold_back, RecordingMode::Hold);
        assert_eq!(toggle_back, RecordingMode::Toggle);
    }

    #[test]
    fn inference_provider_serde_round_trip() {
        let providers = [
            (InferenceProvider::Cpu, "\"cpu\""),
            (InferenceProvider::Coreml, "\"coreml\""),
            (InferenceProvider::Directml, "\"directml\""),
            (InferenceProvider::Webgpu, "\"webgpu\""),
        ];
        for (provider, expected_json) in providers {
            let json = serde_json::to_string(&provider).unwrap();
            assert_eq!(json, expected_json);
            let back: InferenceProvider = serde_json::from_str(&json).unwrap();
            assert_eq!(back, provider);
        }
    }

    #[test]
    fn inference_provider_is_accelerated() {
        assert!(!InferenceProvider::Cpu.is_accelerated());
        assert!(InferenceProvider::Coreml.is_accelerated());
        assert!(InferenceProvider::Directml.is_accelerated());
        assert!(InferenceProvider::Webgpu.is_accelerated());
    }

    #[test]
    fn inference_provider_display() {
        assert_eq!(format!("{}", InferenceProvider::Cpu), "CPU");
        assert_eq!(format!("{}", InferenceProvider::Coreml), "CoreML");
        assert_eq!(format!("{}", InferenceProvider::Directml), "DirectML");
        assert_eq!(format!("{}", InferenceProvider::Webgpu), "WebGPU");
    }

    #[test]
    fn macos_model_runtime_preference_serde_round_trip() {
        for (value, expected_json) in [
            (MacosModelRuntimePreference::Cpu, "\"cpu\""),
            (MacosModelRuntimePreference::Coreml, "\"coreml\""),
            (MacosModelRuntimePreference::Webgpu, "\"webgpu\""),
        ] {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(json, expected_json);
            let back: MacosModelRuntimePreference = serde_json::from_str(&json).unwrap();
            assert_eq!(back, value);
        }
    }

    #[test]
    fn history_item_serialization_round_trip() {
        let item = HistoryItem {
            id: "test-id".to_string(),
            text: "Hello world".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            source_name: "Microphone".to_string(),
            mode: RecordingMode::Hold,
            duration_ms: 1500,
            pasted: true,
            audio_path: Some("/tmp/test.wav".to_string()),
            capture: HistoryCaptureDetails::default(),
        };
        let json = serde_json::to_string(&item).expect("serialize");
        let back: HistoryItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, item.id);
        assert_eq!(back.text, item.text);
        assert_eq!(back.duration_ms, item.duration_ms);
        assert_eq!(back.pasted, item.pasted);
        assert_eq!(back.audio_path, item.audio_path);
        assert_eq!(back.mode, item.mode);
    }

    #[test]
    fn preview_control_generation_increment_and_read() {
        let control = PreviewControl::default();
        assert_eq!(control.current_generation(), 0);
        let gen1 = control.next_generation();
        assert_eq!(gen1, 1);
        assert_eq!(control.current_generation(), 1);
        let gen2 = control.next_generation();
        assert_eq!(gen2, 2);
        assert_eq!(control.current_generation(), 2);
    }

    #[test]
    fn app_core_uses_latest_history_item_for_last_transcript() {
        let core = AppCore::new(
            Settings::default(),
            vec![HistoryItem {
                id: "latest".to_string(),
                text: "Latest transcript".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                source_name: "Microphone".to_string(),
                mode: RecordingMode::Hold,
                duration_ms: 500,
                pasted: true,
                audio_path: None,
                capture: HistoryCaptureDetails::default(),
            }],
        );

        assert_eq!(
            core.last_transcript_text.as_deref(),
            Some("Latest transcript")
        );
    }
}

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use cpal::Stream;

use crate::constants::{DEFAULT_HOLD_SHORTCUT, DEFAULT_TOGGLE_SHORTCUT};
use crate::platform;
use crate::storage::detect_system_profile;
use crate::{default_cleanup_terms, default_overlay_levels, parakeet};

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
    Directml,
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
    pub(crate) selected_source_id: Option<String>,
    pub(crate) auto_paste: bool,
    pub(crate) selected_model_id: String,
    pub(crate) selected_model_kind: TranscriptionModelKind,
    pub(crate) selected_model_path: Option<String>,
    pub(crate) installed_model_paths: BTreeMap<String, String>,
    pub(crate) cleanup_enabled: bool,
    pub(crate) cleanup_terms: Vec<String>,
    pub(crate) audio_retention_policy: AudioRetentionPolicy,
    pub(crate) overlay_position: OverlayPosition,
    pub(crate) overlay_animation_style: OverlayAnimationStyle,
    pub(crate) show_recording_timer: bool,
    pub(crate) show_live_transcription: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hold_shortcut: DEFAULT_HOLD_SHORTCUT.to_string(),
            toggle_shortcut: DEFAULT_TOGGLE_SHORTCUT.to_string(),
            selected_source_id: None,
            auto_paste: true,
            selected_model_id: "parakeet".to_string(),
            selected_model_kind: TranscriptionModelKind::Parakeet,
            selected_model_path: None,
            installed_model_paths: BTreeMap::new(),
            cleanup_enabled: true,
            cleanup_terms: default_cleanup_terms(),
            audio_retention_policy: AudioRetentionPolicy::OneDay,
            overlay_position: OverlayPosition::BottomCenter,
            overlay_animation_style: OverlayAnimationStyle::Spectrum,
            show_recording_timer: false,
            show_live_transcription: false,
        }
    }
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
    pub(crate) selected_source_id: Option<String>,
    pub(crate) auto_paste: Option<bool>,
    pub(crate) selected_model_id: Option<String>,
    pub(crate) selected_model_kind: Option<TranscriptionModelKind>,
    pub(crate) selected_model_path: Option<Option<String>>,
    pub(crate) cleanup_enabled: Option<bool>,
    pub(crate) cleanup_terms: Option<Vec<String>>,
    pub(crate) audio_retention_policy: Option<AudioRetentionPolicy>,
    pub(crate) overlay_position: Option<OverlayPosition>,
    pub(crate) overlay_animation_style: Option<OverlayAnimationStyle>,
    pub(crate) show_recording_timer: Option<bool>,
    pub(crate) show_live_transcription: Option<bool>,
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
pub(crate) struct Snapshot {
    pub(crate) phase: AppPhase,
    pub(crate) settings: Settings,
    pub(crate) sources: Vec<SourceInfo>,
    pub(crate) history: Vec<HistoryItem>,
    pub(crate) model_status: ModelStatus,
    pub(crate) parakeet_model_status: ModelStatus,
    pub(crate) installed_model_sizes: BTreeMap<String, u64>,
    pub(crate) system_profile: SystemProfile,
    pub(crate) shortcuts_active: bool,
    pub(crate) shortcut_message: String,
    pub(crate) status_message: String,
    pub(crate) error_message: Option<String>,
    pub(crate) preview_diagnostics: PreviewDiagnostics,
    pub(crate) overlay: OverlaySnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemProfile {
    pub(crate) logical_cores: usize,
    pub(crate) total_memory_bytes: u64,
    pub(crate) gpu_name: Option<String>,
    pub(crate) gpu_memory_bytes: u64,
    pub(crate) directml_available: bool,
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
}

#[derive(Debug)]
pub(crate) struct StartRecordingResponse {
    pub(crate) source_id: String,
    pub(crate) source_name: String,
    pub(crate) preview_buffer: Arc<Mutex<Vec<f32>>>,
    pub(crate) preview_sample_rate: u32,
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
    pub(crate) sources: Vec<SourceInfo>,
    pub(crate) system_profile: SystemProfile,
    pub(crate) phase: AppPhase,
    pub(crate) status_message: String,
    pub(crate) shortcuts_active: bool,
    pub(crate) shortcut_message: String,
    pub(crate) error_message: Option<String>,
    pub(crate) preview_diagnostics: PreviewDiagnostics,
    pub(crate) model_status: ModelStatus,
    pub(crate) parakeet_model_status: ModelStatus,
    pub(crate) overlay: OverlaySnapshot,
    pub(crate) recording_started_at: Option<Instant>,
}

impl AppCore {
    pub(crate) fn new(settings: Settings, history: Vec<HistoryItem>) -> Self {
        Self {
            settings,
            history,
            sources: Vec::new(),
            system_profile: detect_system_profile(),
            phase: AppPhase::Idle,
            status_message: "Ready".to_string(),
            shortcuts_active: false,
            shortcut_message: "Checking global shortcuts".to_string(),
            error_message: None,
            preview_diagnostics: PreviewDiagnostics::default(),
            model_status: ModelStatus::Missing,
            parakeet_model_status: ModelStatus::Missing,
            overlay: OverlaySnapshot {
                visible: false,
                title: String::new(),
                detail: String::new(),
                levels: default_overlay_levels(),
                elapsed_ms: 0,
                limit_ms: None,
                anchor: None,
            },
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

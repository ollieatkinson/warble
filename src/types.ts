export type RecordingMode = "hold" | "toggle";
export type AppPhase = "idle" | "recording" | "transcribing" | "error";
export type ModelStatus = "ready" | "missing";
export type TranscriptionModelKind = "parakeet" | "parakeet-ctc";
export type InferenceProvider = "cpu" | "coreml" | "directml" | "webgpu";
export type AccelerationProvider = Exclude<InferenceProvider, "cpu">;
export type MacosModelRuntimePreference = Extract<
  InferenceProvider,
  "cpu" | "coreml" | "webgpu"
>;
export type CaptureSourceKind = "microphone" | "file";
export type PlatformKind = "windows" | "macos" | "linux";
export type AutoPasteSupport = "active-app" | "clipboard-only";
export type OverlayPosition =
  | "dynamic-island"
  | "top-center"
  | "top-left"
  | "top-right"
  | "bottom-center"
  | "bottom-left"
  | "bottom-right"
  | "caret";
export type OverlayAnimationStyle = "spectrum" | "waveform" | "radial";
export type LivePreviewModel =
  | "auto"
  | "nemotron-streaming"
  | "parakeet-eou";
export type LiveTranscriptWidth = "compact" | "balanced" | "wide";
export type LiveTranscriptLines = "one" | "two" | "three";
export type ShortcutFieldName =
  | "holdShortcut"
  | "toggleShortcut"
  | "pasteLastShortcut";
export type SectionId = "capture" | "models" | "vocabulary" | "history" | "settings" | "help";
export type SettingsPaneId =
  | "general"
  | "shortcuts"
  | "appearance";
export type ShellDialogId = "settings" | "troubleshooting" | "about";
export type ShellActionId =
  | "open-settings"
  | "open-troubleshooting"
  | "open-about"
  | "navigate-capture"
  | "navigate-models"
  | "navigate-vocabulary"
  | "navigate-history"
  | "transcribe-file"
  | "open-project-page";
export type ModelFilter = "all" | "available" | "streaming";
export type EditableOverlayPosition = Exclude<OverlayPosition, "caret"> | "caret";
export type AudioRetentionPolicy = "one-day" | "seven-days" | "thirty-days";
export type ColorTheme = "system" | "light" | "dark";
export type ModelFeatureIcon =
  | "spark"
  | "cpu"
  | "bolt"
  | "users"
  | "clock";
export type ModelFeatureItem = {
  id: string;
  label: string;
  icon: ModelFeatureIcon;
};

export type ReplacementRule = {
  id: string;
  variants: string[];
  replacement: string;
};

export type Settings = {
  holdShortcut: string;
  toggleShortcut: string;
  pasteLastShortcut: string;
  selectedSourceId: string | null;
  autoPaste: boolean;
  selectedModelId: string;
  selectedModelKind: TranscriptionModelKind;
  selectedModelPath: string | null;
  installedModelPaths: Record<string, string>;
  macosModelRuntimePreferences: Record<string, MacosModelRuntimePreference>;
  cleanupEnabled: boolean;
  cleanupTerms: string[];
  replacementRules: ReplacementRule[];
  audioRetentionPolicy: AudioRetentionPolicy;
  overlayPosition: OverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  livePreviewModel: LivePreviewModel;
  liveTranscriptWidth: LiveTranscriptWidth;
  liveTranscriptLines: LiveTranscriptLines;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
  colorTheme: ColorTheme;
};

export type SourceInfo = {
  id: string;
  name: string;
  sampleRate: number;
  channels: number;
  isDefault: boolean;
};

export type HistoryItem = {
  id: string;
  text: string;
  createdAt: string;
  sourceName: string;
  mode: RecordingMode;
  durationMs: number;
  pasted: boolean;
  audioPath: string | null;
  capture: {
    sourceKind: CaptureSourceKind;
    modelId: string;
    modelName: string;
    inferenceProvider: InferenceProvider;
    inputSampleRate: number;
    inputChannels: number;
    transcriptionSampleRate: number;
  };
};

export type OverlaySnapshot = {
  visible: boolean;
  title: string;
  detail: string;
  levels: number[];
  elapsedMs: number;
  limitMs: number | null;
};

export type ModelDownloadProgress = {
  displayName: string;
  fileName: string;
  downloadedBytes: number;
  totalBytes: number | null;
  bytesPerSecond: number | null;
  secondsRemaining: number | null;
};

export type SystemProfile = {
  logicalCores: number;
  totalMemoryBytes: number;
  gpuName: string | null;
  gpuMemoryBytes: number;
  supportedAccelerationProviders: AccelerationProvider[];
};

export type Snapshot = {
  phase: AppPhase;
  platform: PlatformKind;
  autoPasteSupport: AutoPasteSupport;
  dynamicIslandAvailable: boolean;
  settings: Settings;
  lastTranscriptAvailable: boolean;
  sources: SourceInfo[];
  history: HistoryItem[];
  modelStatus: ModelStatus;
  parakeetModelStatus: ModelStatus;
  installedModelSizes: Record<string, number>;
  modelDownloads: Record<string, ModelDownloadProgress>;
  systemProfile: SystemProfile;
  shortcutsActive: boolean;
  shortcutMessage: string;
  statusMessage: string;
  errorMessage: string | null;
  previewDiagnostics: {
    backend: string;
    status: string;
    detail: string;
    recentEvents: string[];
    logPath: string | null;
  };
  captureDiagnostics: {
    status: string;
    detail: string;
    recentEvents: string[];
    logPath: string | null;
    sourceName: string;
    sampleRate: number;
    channels: number;
    lastBufferedSamples: number;
  };
  overlay: OverlaySnapshot;
};

export type DebugLogs = {
  capture: string;
  livePreview: string;
};

export type SettingsDraft = {
  holdShortcut: string;
  toggleShortcut: string;
  pasteLastShortcut: string;
  selectedSourceId: string;
  autoPaste: boolean;
  cleanupEnabled: boolean;
  audioRetentionPolicy: AudioRetentionPolicy;
  overlayPosition: EditableOverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  livePreviewModel: LivePreviewModel;
  liveTranscriptWidth: LiveTranscriptWidth;
  liveTranscriptLines: LiveTranscriptLines;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
  colorTheme: ColorTheme;
};

export type FlashMessage =
  | {
      kind: "error";
      text: string;
    }
  | null;

export type ButtonFeedbackState = "working" | "done";
export type ModelRowState = "ready" | "downloadable" | "planned" | "incomplete";
export type StatusTone = "success" | "warning" | "danger" | "muted" | "accent";

export type ModelRow = {
  id: string;
  name: string;
  modelKind: TranscriptionModelKind;
  family: string;
  provider: string;
  architecture: string;
  languages: string;
  speechMode: string;
  speed: string;
  quality: string;
  footprint: string;
  runtime: string;
  license: string;
  supportsDefaultSelection?: boolean;
  supportedAccelerationProviders?: AccelerationProvider[];
  unlockedFeatures?: string[];
  state: ModelRowState;
  source: "built-in" | "catalog";
  managed: boolean;
  active: boolean;
  selectable: boolean;
  summary: string;
  note: string;
  bestFor: string;
  capabilities: string[];
  featureBadges: ModelFeatureItem[];
  highlights: string[];
  hfUrl?: string;
  artifactUrl?: string;
  artifactLabel?: string;
  path?: string;
  tags: string[];
  supportsInstall: boolean;
  supportsDownload: boolean;
  downloadSizeBytes?: number;
  diskSizeBytes?: number;
  audioLimitMs?: number | null;
  minimumGpuMemoryBytes?: number;
  recommendedGpuMemoryBytes?: number;
  minimumMemoryBytes?: number;
  recommendedMemoryBytes?: number;
  minimumCores?: number;
  recommendedCores?: number;
};

export type ChoiceOption = {
  id: string;
  label: string;
  description?: string;
};

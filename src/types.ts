export type RecordingMode = "hold" | "toggle";
export type AppPhase = "idle" | "recording" | "transcribing" | "error";
export type ModelStatus = "ready" | "missing";
export type TranscriptionModelKind = "parakeet" | "parakeet-ctc";
export type InferenceProvider = "cpu" | "directml";
export type CaptureSourceKind = "microphone" | "file";
export type OverlayPosition =
  | "bottom-center"
  | "bottom-left"
  | "bottom-right"
  | "caret";
export type OverlayAnimationStyle = "spectrum" | "waveform" | "radial";
export type ShortcutFieldName = "holdShortcut" | "toggleShortcut";
export type SectionId =
  | "overview"
  | "models"
  | "keybindings"
  | "interface"
  | "cleanup"
  | "inputs"
  | "history"
  | "about";
export type ModelFilter = "all" | "available" | "streaming" | "speaker";
export type EditableOverlayPosition = Exclude<OverlayPosition, "caret">;
export type AudioRetentionPolicy = "one-day" | "seven-days" | "thirty-days";
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

export type Settings = {
  holdShortcut: string;
  toggleShortcut: string;
  selectedSourceId: string | null;
  autoPaste: boolean;
  selectedModelId: string;
  selectedModelKind: TranscriptionModelKind;
  selectedModelPath: string | null;
  installedModelPaths: Record<string, string>;
  cleanupEnabled: boolean;
  cleanupTerms: string[];
  audioRetentionPolicy: AudioRetentionPolicy;
  overlayPosition: OverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
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

export type SystemProfile = {
  logicalCores: number;
  totalMemoryBytes: number;
  gpuName: string | null;
  gpuMemoryBytes: number;
  directmlAvailable: boolean;
};

export type Snapshot = {
  phase: AppPhase;
  settings: Settings;
  sources: SourceInfo[];
  history: HistoryItem[];
  modelStatus: ModelStatus;
  parakeetModelStatus: ModelStatus;
  installedModelSizes: Record<string, number>;
  systemProfile: SystemProfile;
  shortcutsActive: boolean;
  shortcutMessage: string;
  statusMessage: string;
  errorMessage: string | null;
  overlay: OverlaySnapshot;
};

export type SettingsDraft = {
  holdShortcut: string;
  toggleShortcut: string;
  selectedSourceId: string;
  autoPaste: boolean;
  cleanupEnabled: boolean;
  audioRetentionPolicy: AudioRetentionPolicy;
  overlayPosition: EditableOverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
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
  directmlCapable?: boolean;
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

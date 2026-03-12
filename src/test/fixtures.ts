import type {
  HistoryItem,
  ModelRow,
  Settings,
  SettingsDraft,
  Snapshot,
  SourceInfo,
  SystemProfile,
} from "../types";

export function createSettings(overrides?: Partial<Settings>): Settings {
  return {
    holdShortcut: "Ctrl+Shift+Space",
    toggleShortcut: "",
    selectedSourceId: "default-mic",
    autoPaste: true,
    selectedModelId: "parakeet",
    selectedModelKind: "parakeet",
    selectedModelPath: null,
    installedModelPaths: {},
    cleanupEnabled: true,
    cleanupTerms: ["um", "uh", "erm"],
    audioRetentionPolicy: "seven-days",
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    livePreviewModel: "auto",
    liveTranscriptWidth: "balanced",
    liveTranscriptLines: "two",
    showRecordingTimer: true,
    showLiveTranscription: true,
    colorTheme: "system",
    ...overrides,
  };
}

export function createSettingsDraft(
  overrides?: Partial<SettingsDraft>,
): SettingsDraft {
  return {
    holdShortcut: "Ctrl+Shift+Space",
    toggleShortcut: "",
    selectedSourceId: "default-mic",
    autoPaste: true,
    cleanupEnabled: true,
    audioRetentionPolicy: "seven-days",
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    livePreviewModel: "auto",
    liveTranscriptWidth: "balanced",
    liveTranscriptLines: "two",
    showRecordingTimer: true,
    showLiveTranscription: true,
    colorTheme: "system",
    ...overrides,
  };
}

export function createSourceInfo(
  overrides?: Partial<SourceInfo>,
): SourceInfo {
  return {
    id: "default-mic",
    name: "Built-in Microphone",
    sampleRate: 48000,
    channels: 2,
    isDefault: true,
    ...overrides,
  };
}

export function createHistoryItem(
  overrides?: Partial<HistoryItem>,
): HistoryItem {
  return {
    id: "hist-001",
    text: "Hello world",
    createdAt: "2025-01-15T10:30:00Z",
    sourceName: "Built-in Microphone",
    mode: "hold",
    durationMs: 2500,
    pasted: true,
    audioPath: null,
    capture: {
      sourceKind: "microphone",
      modelId: "parakeet",
      modelName: "Parakeet TDT",
      inferenceProvider: "cpu",
      inputSampleRate: 48000,
      inputChannels: 2,
      transcriptionSampleRate: 16000,
    },
    ...overrides,
  };
}

export function createSystemProfile(
  overrides?: Partial<SystemProfile>,
): SystemProfile {
  return {
    logicalCores: 8,
    totalMemoryBytes: 16 * 1024 ** 3,
    gpuName: "NVIDIA RTX 3060",
    gpuMemoryBytes: 6 * 1024 ** 3,
    supportedAccelerationProviders: ["directml"],
    ...overrides,
  };
}

export function createSnapshot(overrides?: Partial<Snapshot>): Snapshot {
  return {
    phase: "idle",
    platform: "windows",
    autoPasteSupport: "active-app",
    settings: createSettings(overrides?.settings),
    sources: [createSourceInfo()],
    history: [],
    modelStatus: "ready",
    parakeetModelStatus: "ready",
    installedModelSizes: {},
    modelDownloads: {},
    systemProfile: createSystemProfile(overrides?.systemProfile),
    shortcutsActive: true,
    shortcutMessage: "",
    statusMessage: "Idle",
    errorMessage: null,
    previewDiagnostics: {
      backend: "batch",
      status: "idle",
      detail: "",
      recentEvents: [],
      logPath: null,
    },
    overlay: {
      visible: false,
      title: "",
      detail: "",
      levels: Array(12).fill(0),
      elapsedMs: 0,
      limitMs: null,
    },
    ...overrides,
  };
}

export function createModelRow(overrides?: Partial<ModelRow>): ModelRow {
  return {
    id: "parakeet",
    name: "Parakeet TDT",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "FastConformer + TDT",
    languages: "25 languages",
    speechMode: "Batch ASR",
    speed: "Fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Ready in app",
    license: "See model card",
    state: "ready",
    source: "built-in",
    managed: false,
    active: true,
    selectable: true,
    summary: "Multilingual long-form offline dictation model.",
    note: "Default transcription engine.",
    bestFor: "Long-form multilingual dictation",
    capabilities: ["TDT decoder", "Auto language detection"],
    featureBadges: [
      { id: "local", label: "Local", icon: "cpu" },
      { id: "tdt", label: "TDT", icon: "spark" },
    ],
    highlights: ["Current default"],
    tags: ["nvidia", "offline", "timestamps", "default"],
    supportsInstall: false,
    supportsDownload: true,
    downloadSizeBytes: 593_000_000,
    audioLimitMs: 24 * 60 * 1000,
    ...overrides,
  };
}

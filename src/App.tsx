import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode, SVGProps } from "react";

type RecordingMode = "hold" | "toggle";
type AppPhase = "idle" | "recording" | "transcribing" | "error";
type ModelStatus = "ready" | "missing";
type TranscriptionModelKind = "parakeet" | "parakeet-ctc";
type OverlayPosition =
  | "bottom-center"
  | "bottom-left"
  | "bottom-right"
  | "caret";
type OverlayAnimationStyle = "spectrum" | "waveform" | "radial";
type ShortcutFieldName = "holdShortcut" | "toggleShortcut";
type SectionId =
  | "overview"
  | "models"
  | "keybindings"
  | "interface"
  | "cleanup"
  | "inputs"
  | "history"
  | "about";
type ModelFilter = "all" | "available" | "multilingual" | "future";
type EditableOverlayPosition = Exclude<OverlayPosition, "caret">;
type AudioRetentionPolicy = "one-day" | "seven-days" | "thirty-days";

type Settings = {
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
  showLiveTranscription: boolean;
};

type SourceInfo = {
  id: string;
  name: string;
  sampleRate: number;
  channels: number;
  isDefault: boolean;
};

type HistoryItem = {
  id: string;
  text: string;
  createdAt: string;
  sourceName: string;
  mode: RecordingMode;
  durationMs: number;
  pasted: boolean;
  audioPath: string | null;
};

type OverlaySnapshot = {
  visible: boolean;
  title: string;
  detail: string;
  levels: number[];
};

type SystemProfile = {
  logicalCores: number;
  totalMemoryBytes: number;
};

type Snapshot = {
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

type SettingsDraft = {
  holdShortcut: string;
  toggleShortcut: string;
  selectedSourceId: string;
  autoPaste: boolean;
  cleanupEnabled: boolean;
  audioRetentionPolicy: AudioRetentionPolicy;
  overlayPosition: EditableOverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
};

type FlashMessage = {
  kind: "error";
  text: string;
} | null;

type ButtonFeedbackState = "working" | "done";
type ModelRowState = "ready" | "downloadable" | "planned" | "incomplete";
type StatusTone = "success" | "warning" | "danger" | "muted" | "accent";

type ModelRow = {
  id: string;
  name: string;
  modelKind: TranscriptionModelKind;
  family: string;
  provider: string;
  architecture: string;
  languages: string;
  speed: string;
  quality: string;
  footprint: string;
  runtime: string;
  license: string;
  state: ModelRowState;
  source: "built-in" | "catalog";
  managed: boolean;
  active: boolean;
  selectable: boolean;
  summary: string;
  note: string;
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
  minimumMemoryBytes?: number;
  recommendedMemoryBytes?: number;
  minimumCores?: number;
  recommendedCores?: number;
};

type ChoiceOption = {
  id: string;
  label: string;
  description?: string;
};

type IconProps = SVGProps<SVGSVGElement>;

const SNAPSHOT_EVENT = "transcribed://snapshot";
const SIDEBAR_COLLAPSED_KEY = "transcribed:sidebar-collapsed";
const isIndicatorWindow = new URLSearchParams(window.location.search).has(
  "indicator",
);

const sections: Array<{
  id: SectionId;
  label: string;
}> = [
  { id: "overview", label: "Overview" },
  { id: "models", label: "Models" },
  { id: "keybindings", label: "Keys" },
  { id: "interface", label: "Interface" },
  { id: "cleanup", label: "Cleanup" },
  { id: "inputs", label: "Input" },
  { id: "history", label: "History" },
  { id: "about", label: "About" },
];

const modelFilters: Array<{
  id: ModelFilter;
  label: string;
}> = [
  { id: "all", label: "All" },
  { id: "available", label: "Available" },
  { id: "multilingual", label: "Multilingual" },
  { id: "future", label: "Future" },
];

const modelTypeTabs = [
  { id: "speech", label: "Speech", active: true },
  { id: "language", label: "Language", active: false },
  { id: "embedding", label: "Embedding", active: false },
];

const overlayPositionOptions: Array<{
  id: EditableOverlayPosition;
  label: string;
}> = [
  { id: "bottom-center", label: "Center" },
  { id: "bottom-left", label: "Left" },
  { id: "bottom-right", label: "Right" },
];

const overlayAnimationOptions: Array<{
  id: OverlayAnimationStyle;
  label: string;
  description: string;
}> = [
  { id: "waveform", label: "Wave", description: "Static centered waveform" },
  { id: "spectrum", label: "Bars", description: "Fixed reactive bars" },
  { id: "radial", label: "Radial", description: "Static reactive ring" },
];

const DEMO_LEVELS = [0.18, 0.34, 0.62, 0.28, 0.82, 0.46, 0.24, 0.58, 0.38, 0.22, 0.48, 0.26];
const CLEANUP_SUGGESTIONS = ["um", "uh", "erm", "uhm", "hmm", "you know"];
const audioRetentionOptions: Array<{
  id: AudioRetentionPolicy;
  label: string;
  description: string;
}> = [
  { id: "one-day", label: "24h", description: "Keep clip audio for one day." },
  { id: "seven-days", label: "7d", description: "Keep clip audio for seven days." },
  { id: "thirty-days", label: "30d", description: "Keep clip audio for thirty days." },
];

async function getSnapshot() {
  return invoke<Snapshot>("get_snapshot");
}

function formatDuration(durationMs: number) {
  const seconds = durationMs / 1000;
  return `${seconds.toFixed(seconds > 10 ? 0 : 1)}s`;
}

function formatPhaseLabel(phase: AppPhase) {
  switch (phase) {
    case "recording":
      return "Recording";
    case "transcribing":
      return "Transcribing";
    case "error":
      return "Attention";
    case "idle":
    default:
      return "Idle";
  }
}

function formatOverlayPosition(position: OverlayPosition) {
  switch (position) {
    case "bottom-left":
      return "Bottom left";
    case "bottom-right":
      return "Bottom right";
    case "caret":
      return "Near caret";
    case "bottom-center":
    default:
      return "Bottom center";
  }
}

function formatOverlayAnimationStyle(style: OverlayAnimationStyle) {
  switch (style) {
    case "radial":
      return "Radial";
    case "waveform":
      return "Wave";
    case "spectrum":
    default:
      return "Bars";
  }
}

function formatAudioRetentionPolicy(policy: AudioRetentionPolicy) {
  switch (policy) {
    case "seven-days":
      return "7 days";
    case "thirty-days":
      return "30 days";
    case "one-day":
    default:
      return "24 hours";
  }
}

function formatBytes(bytes?: number | null) {
  if (!bytes || bytes <= 0) {
    return "Not installed";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const decimals = value >= 100 || unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(decimals)} ${units[unitIndex]}`;
}

function formatSystemProfile(profile: SystemProfile) {
  const memoryLabel =
    profile.totalMemoryBytes > 0
      ? `${formatBytes(profile.totalMemoryBytes)} RAM`
      : "RAM unknown";
  const coreLabel = `${profile.logicalCores} threads`;
  return `${memoryLabel} · ${coreLabel}`;
}

function formatHardwareTarget(row: ModelRow) {
  const memoryLabel = row.recommendedMemoryBytes
    ? formatBytes(row.recommendedMemoryBytes)
    : row.minimumMemoryBytes
      ? formatBytes(row.minimumMemoryBytes)
      : null;
  const coreLabel = row.recommendedCores ?? row.minimumCores ?? null;

  if (memoryLabel && coreLabel) {
    return `${memoryLabel} RAM and ${coreLabel}+ threads`;
  }

  if (memoryLabel) {
    return `${memoryLabel} RAM`;
  }

  if (coreLabel) {
    return `${coreLabel}+ threads`;
  }

  return "unknown hardware target";
}

function describeHardwareFit(row: ModelRow, profile: SystemProfile) {
  if (!row.minimumMemoryBytes && !row.minimumCores) {
    return {
      label: row.supportsDownload || row.selectable ? "Unknown" : "Planned",
      tone: row.supportsDownload || row.selectable ? "muted" : "warning",
      detail:
        row.supportsDownload || row.selectable
          ? "No hardware estimate for this model yet."
          : "This catalog entry is reference-only for now.",
    } as const;
  }

  if (profile.totalMemoryBytes <= 0) {
    return {
      label: "Unknown",
      tone: "muted",
      detail: `Need RAM info to estimate fit. Target: ${formatHardwareTarget(row)}.`,
    } as const;
  }

  const memory = profile.totalMemoryBytes;
  const cores = profile.logicalCores;
  const minimumMemory = row.minimumMemoryBytes ?? 0;
  const recommendedMemory = row.recommendedMemoryBytes ?? minimumMemory;
  const minimumCores = row.minimumCores ?? 1;
  const recommendedCores = row.recommendedCores ?? minimumCores;

  if (memory >= recommendedMemory && cores >= recommendedCores) {
    return {
      label: "Great fit",
      tone: "success",
      detail: `${formatSystemProfile(profile)} should run ${row.name} comfortably.`,
    } as const;
  }

  if (memory >= minimumMemory && cores >= minimumCores) {
    return {
      label: "Should work",
      tone: "accent",
      detail: `${formatSystemProfile(profile)} should handle ${row.name}, but expect heavier CPU/RAM use than the recommended target of ${formatHardwareTarget(row)}.`,
    } as const;
  }

  return {
    label: "Heavy",
    tone: "warning",
    detail: `${formatSystemProfile(profile)} is below the recommended target of ${formatHardwareTarget(row)}.`,
  } as const;
}

function formatModelSizeLabel(row: ModelRow) {
  if (row.diskSizeBytes && row.diskSizeBytes > 0) {
    return formatBytes(row.diskSizeBytes);
  }

  if (row.downloadSizeBytes && row.downloadSizeBytes > 0) {
    return formatBytes(row.downloadSizeBytes);
  }

  return row.footprint;
}

function modelSpeedScore(row: ModelRow) {
  switch (row.id) {
    case "parakeet":
      return 4.6;
    case "parakeet-ctc":
      return 4.3;
    case "parakeet-eou":
      return 4.8;
    case "nemotron-streaming":
      return 4.1;
    default:
      return 3;
  }
}

function modelAccuracyScore(row: ModelRow) {
  switch (row.id) {
    case "parakeet":
      return 4.3;
    case "parakeet-ctc":
      return 4.1;
    case "parakeet-eou":
      return 3.9;
    case "nemotron-streaming":
      return 4.4;
    default:
      return 3.5;
  }
}

function modelFeatureItems(row: ModelRow) {
  const items = [
    {
      id: "runtime",
      label:
        row.state === "planned"
          ? "Research"
          : row.supportsDownload || row.selectable
            ? "Local"
            : "Catalog",
      icon: row.state === "planned" ? (
        <SparkIcon className="small-icon" />
      ) : (
        <CpuIcon className="small-icon" />
      ),
    },
    {
      id: "language",
      label: row.tags.includes("multilingual") ? "Multilingual" : "English",
      icon: row.tags.includes("multilingual") ? (
        <GlobeIcon className="small-icon" />
      ) : (
        <InputIcon className="small-icon" />
      ),
    },
    {
      id: "focus",
      label:
        row.tags.includes("streaming")
          ? "Streaming"
          : row.id === "parakeet"
            ? "Dictation"
            : row.id === "parakeet-ctc"
              ? "English"
              : "Reference",
      icon:
        row.tags.includes("streaming") ? (
          <BoltIcon className="small-icon" />
        ) : row.id === "parakeet" ? (
          <InputIcon className="small-icon" />
        ) : (
          <SparkIcon className="small-icon" />
        ),
    },
  ];

  return items;
}

function normalizeEditableOverlayPosition(
  position: OverlayPosition,
): EditableOverlayPosition {
  switch (position) {
    case "bottom-left":
    case "bottom-right":
      return position;
    case "caret":
    case "bottom-center":
    default:
      return "bottom-center";
  }
}

function formatInvokeError(error: unknown) {
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "Something went wrong.";
}

function formatShortcutKey(key: string) {
  if (key === " ") {
    return "Space";
  }

  if (/^f\d{1,2}$/i.test(key)) {
    return key.toUpperCase();
  }

  const keyMap: Record<string, string> = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
  };

  if (keyMap[key]) {
    return keyMap[key];
  }

  if (key.length === 1) {
    return key.toUpperCase();
  }

  return key;
}

function captureShortcut(event: KeyboardEvent<HTMLButtonElement>) {
  const ignored = new Set(["Control", "Shift", "Alt", "Meta"]);
  const mainKey = formatShortcutKey(event.key);

  if (ignored.has(event.key) || !mainKey) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }

  parts.push(mainKey);
  return parts.join("+");
}

function matchesHistory(item: HistoryItem, query: string) {
  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  if (terms.length === 0) {
    return true;
  }

  const haystack = [
    item.text,
    item.sourceName,
    item.mode,
    item.pasted ? "pasted" : "saved",
  ]
    .join(" ")
    .toLowerCase();

  return terms.every((term) => haystack.includes(term));
}

function buildSettingsUpdate(draft: SettingsDraft) {
  return {
    holdShortcut: draft.holdShortcut,
    toggleShortcut: draft.toggleShortcut,
    selectedSourceId: draft.selectedSourceId || undefined,
    autoPaste: draft.autoPaste,
    cleanupEnabled: draft.cleanupEnabled,
    audioRetentionPolicy: draft.audioRetentionPolicy,
    overlayPosition: draft.overlayPosition,
    overlayAnimationStyle: draft.overlayAnimationStyle,
    showLiveTranscription: draft.showLiveTranscription,
  };
}

function resampleLevels(sourceLevels: number[], count: number) {
  return Array.from({ length: count }, (_, index) => {
    if (count <= 1 || sourceLevels.length === 1) {
      return sourceLevels[0] ?? 0.14;
    }

    const position = (index / (count - 1)) * (sourceLevels.length - 1);
    const nearestIndex = Math.round(position);
    return sourceLevels[nearestIndex] ?? sourceLevels[0] ?? 0.14;
  });
}

function smoothLevels(levels: number[]) {
  return levels.map((level, index, values) => {
    const previous = values[index - 1] ?? level;
    const next = values[index + 1] ?? level;
    const fartherPrevious = values[index - 2] ?? previous;
    const fartherNext = values[index + 2] ?? next;
    return (
      fartherPrevious * 0.1 +
      previous * 0.2 +
      level * 0.4 +
      next * 0.2 +
      fartherNext * 0.1
    );
  });
}

const MODEL_CATALOG: Array<
  Omit<ModelRow, "state" | "source" | "managed" | "active" | "selectable" | "path">
> = [
  {
    id: "parakeet",
    name: "Parakeet TDT",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "FastConformer + TDT",
    languages: "25 languages",
    speed: "Fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Ready in app",
    license: "See model card",
    summary: "Multilingual Parakeet TDT with local ONNX inference and fast dictation latency.",
    note: "Best current in-app path for local dictation, auto language detection, and broader language support.",
    highlights: [
      "Current default engine in Transcribed",
      "Uses parakeet-rs in the Rust backend",
      "Supports timestamps and DirectML-capable ONNX execution",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3",
    artifactUrl: "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8",
    artifactLabel: "ONNX export bundle",
    tags: ["multilingual", "nvidia", "available", "timestamps"],
    supportsInstall: true,
    supportsDownload: true,
    downloadSizeBytes: 593_000_000,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 4,
    recommendedCores: 8,
  },
  {
    id: "parakeet-ctc",
    name: "Parakeet CTC",
    modelKind: "parakeet-ctc",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "FastConformer + CTC",
    languages: "English",
    speed: "Very fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Backend ready",
    license: "See model card",
    summary: "English-only CTC model with punctuation and capitalization through parakeet-rs.",
    note: "Good fit when you want a simpler English-first Parakeet path without the TDT multilingual stack.",
    highlights: [
      "Uses parakeet-rs in the Rust backend",
      "Strong English punctuation and capitalization",
      "Timestamp-capable via the underlying crate",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-ctc-0.6b",
    artifactUrl: "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/tree/main/onnx",
    artifactLabel: "ONNX export bundle",
    tags: ["english", "nvidia", "available", "timestamps"],
    supportsInstall: true,
    supportsDownload: true,
    downloadSizeBytes: 1_240_000_000,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 4,
    recommendedCores: 8,
  },
  {
    id: "parakeet-eou",
    name: "Parakeet Realtime EOU",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "Streaming encoder-decoder + EOU",
    languages: "English",
    speed: "Realtime",
    quality: "Balanced",
    footprint: "120M",
    runtime: "Streaming integration pending",
    license: "See model card",
    summary: "Realtime Parakeet model with end-of-utterance detection, exposed by parakeet-rs.",
    note: "Best next target for a truer live transcription experience, but not yet wired into the app session flow.",
    highlights: [
      "Designed for chunked live transcription",
      "End-of-utterance aware",
      "Good candidate for a future streaming HUD mode",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet_realtime_eou_120m-v1",
    tags: ["english", "nvidia", "future", "streaming"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
  {
    id: "nemotron-streaming",
    name: "Nemotron Streaming",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "Cache-aware streaming RNNT",
    languages: "English",
    speed: "Realtime",
    quality: "High",
    footprint: "0.6B",
    runtime: "Streaming integration pending",
    license: "See model card",
    summary: "Streaming Nemotron speech model supported by parakeet-rs for chunked ASR.",
    note: "Promising for future low-latency dictation with punctuation, but not yet connected to the current app flow.",
    highlights: [
      "Cache-aware streaming path",
      "Good punctuation-oriented future option",
      "Not yet supported in-app",
    ],
    hfUrl: "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b",
    tags: ["english", "nvidia", "future", "streaming"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 8 * 1024 ** 3,
    recommendedMemoryBytes: 16 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
];

function buildModelRows(snapshot: Snapshot): ModelRow[] {
  const activeModelId = snapshot.settings.selectedModelId;
  const rows = MODEL_CATALOG.map<ModelRow>((entry) => {
    if (entry.id === "parakeet") {
      const installedPath = snapshot.settings.installedModelPaths.parakeet ?? null;
      const diskSizeBytes = snapshot.installedModelSizes.parakeet ?? 0;
      const builtInReady = snapshot.parakeetModelStatus === "ready";
      const isReady = builtInReady || Boolean(installedPath);
      const active =
        activeModelId === "parakeet" &&
        snapshot.settings.selectedModelKind === "parakeet" &&
        snapshot.modelStatus === "ready";

      return {
        ...entry,
        state: isReady ? "ready" : "downloadable",
        source: "built-in",
        managed: Boolean(installedPath && isManagedModelPath(installedPath)),
        active,
        selectable: isReady,
        runtime:
          builtInReady || installedPath
            ? "Ready in app"
            : "Download in app",
        note:
          builtInReady
            ? entry.note
            : installedPath
              ? "Downloaded into Transcribed and ready to use as the local Parakeet engine."
              : "Parakeet isn't bundled on this machine right now, but you can download the compatible ONNX bundle directly in the app.",
        path: installedPath,
        diskSizeBytes,
      };
    }

    const installedPath = snapshot.settings.installedModelPaths[entry.id] ?? null;
    const isSelectedEngine =
      activeModelId === entry.id &&
      snapshot.settings.selectedModelKind === entry.modelKind;
    const isReady = Boolean(installedPath);
    const supportsDownload = Boolean(entry.supportsDownload);
    const isManaged = Boolean(installedPath && isManagedModelPath(installedPath));

    return {
      ...entry,
      state: isReady ? "ready" : supportsDownload ? "downloadable" : "planned",
      source: "catalog",
      managed: isManaged,
      active: isSelectedEngine && snapshot.modelStatus === "ready",
      selectable: isReady,
      runtime: isReady ? "Ready in app" : supportsDownload ? "Download in app" : entry.runtime,
      note: isReady
        ? isManaged
          ? "Downloaded into Transcribed and ready to use locally."
          : "Linked to a local Parakeet model folder. You can activate it from this catalog entry."
        : supportsDownload
          ? "Download this Parakeet variant from Hugging Face or point Transcribed at an existing compatible model folder."
          : "Reference-only for now. Browse the model card, but the runtime is not wired into Transcribed yet.",
      path: installedPath,
      diskSizeBytes: snapshot.installedModelSizes[entry.id] ?? 0,
      tags: isReady
        ? Array.from(new Set([...entry.tags, "available"]))
        : entry.tags,
    };
  });

  return rows;
}

function isManagedModelPath(path: string) {
  return /[\\/]catalog-models(?:[\\/]|$)/i.test(path);
}

function matchesModel(row: ModelRow, query: string, filter: ModelFilter) {
  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery) {
    const haystack = [
      row.name,
      row.family,
      row.provider,
      row.architecture,
      row.languages,
      row.speed,
      row.quality,
      row.footprint,
      formatModelSizeLabel(row),
      row.runtime,
      row.license,
      row.source,
      row.summary,
      row.note,
      row.artifactLabel ?? "",
      ...row.highlights,
    ]
      .join(" ")
      .toLowerCase();
    if (!haystack.includes(normalizedQuery)) {
      return false;
    }
  }

  switch (filter) {
    case "available":
      return row.state === "ready" || row.state === "downloadable";
    case "multilingual":
      return row.tags.includes("multilingual");
    case "future":
      return row.state === "planned";
    case "all":
    default:
      return true;
  }
}

function useSnapshotState() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      const current = await getSnapshot();
      if (mounted) {
        setSnapshot(current);
      }

      unlisten = await listen<Snapshot>(SNAPSHOT_EVENT, (event) => {
        setSnapshot(event.payload);
      });
    })();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return [snapshot, setSnapshot] as const;
}

function GlyphBase({
  children,
  className,
  ...props
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

function OverviewIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.75" y="4.75" width="5.25" height="5.25" rx="1.35" />
      <rect x="14" y="4.75" width="5.25" height="5.25" rx="1.35" />
      <rect x="4.75" y="14" width="5.25" height="5.25" rx="1.35" />
      <rect x="14" y="14" width="5.25" height="5.25" rx="1.35" />
    </GlyphBase>
  );
}

function ModelsIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M6 8.5 12 5l6 3.5-6 3.5L6 8.5Z" />
      <path d="M6 12.5 12 16l6-3.5" />
      <path d="M6 16.5 12 20l6-3.5" />
    </GlyphBase>
  );
}

function KeysIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="6.5" width="15" height="11" rx="3" />
      <path d="M8 10.5h2" />
      <path d="M12 10.5h4" />
      <path d="M8 14.5h8" />
    </GlyphBase>
  );
}

function InputIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5a3 3 0 0 1 3 3v4a3 3 0 0 1-6 0v-4a3 3 0 0 1 3-3Z" />
      <path d="M7.5 10.5a4.5 4.5 0 1 0 9 0" />
      <path d="M12 15v4.5" />
    </GlyphBase>
  );
}

function CleanupIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M5 18.5h11.5" />
      <path d="M8 18.5 15.5 5" />
      <path d="M12 18.5 18.5 9" />
      <path d="M14.5 5h4" />
    </GlyphBase>
  );
}

function InterfaceIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="5.5" width="15" height="13" rx="3" />
      <path d="M7.5 9h9" />
      <rect x="7" y="12" width="10" height="3.5" rx="1.75" />
    </GlyphBase>
  );
}

function HistoryIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3" />
      <path d="M4.5 5.5v4h4" />
      <path d="M12 8.5v4l2.5 1.5" />
    </GlyphBase>
  );
}

function AboutIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 10v5" />
      <path d="M12 7.5h.01" />
    </GlyphBase>
  );
}

function SparkIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 3.5 13.7 8l4.8 1.7L13.7 11.4 12 16l-1.7-4.6L5.5 9.7 10.3 8 12 3.5Z" />
    </GlyphBase>
  );
}

function CheckIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m6.5 12 3.5 3.5 7-7" />
    </GlyphBase>
  );
}

function CloseIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m7 7 10 10" />
      <path d="m17 7-10 10" />
    </GlyphBase>
  );
}

function ChevronDownIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m6 9 6 6 6-6" />
    </GlyphBase>
  );
}

function SearchIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="11" cy="11" r="5.5" />
      <path d="m16 16 3.5 3.5" />
    </GlyphBase>
  );
}

function RefreshIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M19.5 11.5A7.5 7.5 0 0 0 6.7 6.2" />
      <path d="M6.5 4.5v3.8h3.8" />
      <path d="M4.5 12.5a7.5 7.5 0 0 0 12.8 5.3" />
      <path d="M17.5 19.5v-3.8h-3.8" />
    </GlyphBase>
  );
}

function DownloadIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5v10" />
      <path d="m8.5 11 3.5 3.5 3.5-3.5" />
      <path d="M5 18.5h14" />
    </GlyphBase>
  );
}

function BoltIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M13.5 3.5 7.5 13h4l-1 7.5 6-9h-4l1-8Z" />
    </GlyphBase>
  );
}

function GlobeIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M4.5 12h15" />
      <path d="M12 4.5a12 12 0 0 1 0 15" />
      <path d="M12 4.5a12 12 0 0 0 0 15" />
    </GlyphBase>
  );
}

function CpuIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="7.5" y="7.5" width="9" height="9" rx="2" />
      <path d="M9.5 2.5v3" />
      <path d="M14.5 2.5v3" />
      <path d="M9.5 18.5v3" />
      <path d="M14.5 18.5v3" />
      <path d="M2.5 9.5h3" />
      <path d="M2.5 14.5h3" />
      <path d="M18.5 9.5h3" />
      <path d="M18.5 14.5h3" />
    </GlyphBase>
  );
}

function FolderIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 8.5h5l1.5 2h8.5v7a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2Z" />
    </GlyphBase>
  );
}

function CopyIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="8" y="8" width="10" height="10" rx="2" />
      <path d="M6.5 15.5h-1A2.5 2.5 0 0 1 3 13V5.5A2.5 2.5 0 0 1 5.5 3H13a2.5 2.5 0 0 1 2.5 2.5v1" />
    </GlyphBase>
  );
}

function ExternalIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M14 5h5v5" />
      <path d="m10 14 9-9" />
      <path d="M19 13v4a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4" />
    </GlyphBase>
  );
}

function TrashIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M5.5 7.5h13" />
      <path d="M9 7.5V5.8A1.8 1.8 0 0 1 10.8 4h2.4A1.8 1.8 0 0 1 15 5.8v1.7" />
      <path d="M7.5 7.5 8.2 18a2 2 0 0 0 2 1.9h3.6a2 2 0 0 0 2-1.9l.7-10.5" />
      <path d="M10 11v5" />
      <path d="M14 11v5" />
    </GlyphBase>
  );
}

function SectionIcon({
  section,
  className,
}: {
  section: SectionId;
  className?: string;
}) {
  switch (section) {
    case "models":
      return <ModelsIcon className={className} />;
    case "keybindings":
      return <KeysIcon className={className} />;
    case "interface":
      return <InterfaceIcon className={className} />;
    case "cleanup":
      return <CleanupIcon className={className} />;
    case "inputs":
      return <InputIcon className={className} />;
    case "history":
      return <HistoryIcon className={className} />;
    case "about":
      return <AboutIcon className={className} />;
    case "overview":
    default:
      return <OverviewIcon className={className} />;
  }
}

function toneForPhase(phase: AppPhase): StatusTone {
  switch (phase) {
    case "recording":
      return "danger";
    case "transcribing":
      return "warning";
    case "error":
      return "danger";
    case "idle":
    default:
      return "success";
  }
}

function SignalBars({
  phase,
  levels,
  count = 24,
  compact = false,
  animationStyle = "spectrum",
}: {
  phase: AppPhase;
  levels?: number[];
  count?: number;
  compact?: boolean;
  animationStyle?: OverlayAnimationStyle;
}) {
  const tone =
    phase === "transcribing"
      ? "warm"
      : phase === "recording"
        ? "live"
        : "idle";
  const sourceLevels = levels && levels.length > 0 ? levels : [0];

  if (animationStyle === "spectrum") {
    const width = compact ? 60 : 94;
    const height = compact ? 18 : 26;
    const baselineY = height - (compact ? 1.5 : 2);
    const barCount = compact ? 12 : 16;
    const inset = compact ? 3.5 : 0;
    const gap = compact ? 1.8 : 2.2;
    const innerWidth = width - inset * 2;
    const barWidth = (innerWidth - gap * (barCount - 1)) / barCount;
    const minHeight = compact ? 2.2 : 3;
    const maxHeight = compact ? 14 : 21;
    const sampled = resampleLevels(sourceLevels, barCount);

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className={`signal-bars signal-bars-${tone} ${compact ? "signal-bars-compact" : ""}`}
        aria-hidden="true"
      >
        <path
          d={`M ${inset} ${baselineY} L ${width - inset} ${baselineY}`}
          className="signal-bars-base"
        />
        {sampled.map((level, index) => {
          const normalized = Math.max(0, level);
          const barHeight = minHeight + normalized * (maxHeight - minHeight);
          const x = inset + index * (barWidth + gap);
          const y = baselineY - barHeight;

          return (
            <rect
              key={`bar-${index}`}
              x={x}
              y={y}
              width={barWidth}
              height={barHeight}
              rx={barWidth / 2}
              className="signal-bars-bar"
            />
          );
        })}
      </svg>
    );
  }

  if (animationStyle === "radial") {
    const width = compact ? 26 : 34;
    const height = compact ? 26 : 34;
    const cx = width / 2;
    const cy = height / 2;
    const pointCount = compact ? 18 : 24;
    const baseRadius = compact ? 8.3 : 11.4;
    const maxExtension = compact ? 3.2 : 4.6;
    const sampled = resampleLevels(sourceLevels, pointCount);

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className={`signal-radial signal-radial-${tone} ${compact ? "signal-radial-compact" : ""}`}
        aria-hidden="true"
      >
        <circle cx={cx} cy={cy} r={baseRadius} className="signal-radial-ring" />
        {sampled.map((level, index) => {
          const angle = (index / pointCount) * Math.PI * 2 - Math.PI / 2;
          const normalized = Math.max(0, level);
          const innerRadius = baseRadius - 0.2;
          const outerRadius = baseRadius + 0.7 + normalized * maxExtension;
          const x1 = cx + Math.cos(angle) * innerRadius;
          const y1 = cy + Math.sin(angle) * innerRadius;
          const x2 = cx + Math.cos(angle) * outerRadius;
          const y2 = cy + Math.sin(angle) * outerRadius;

          return (
            <line
              key={`spoke-${index}`}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              className="signal-radial-spoke"
            />
          );
        })}
        <circle
          cx={cx}
          cy={cy}
          r={compact ? 5.8 : 7}
          className="signal-radial-core-halo"
        />
        <circle
          cx={cx}
          cy={cy}
          r={compact ? 3.8 : 4.8}
          className="signal-radial-core"
        />
      </svg>
    );
  }

  const smoothedLevels = smoothLevels(
    resampleLevels(sourceLevels, compact ? 24 : count),
  );
  const width = compact ? 62 : 108;
  const height = compact ? 20 : 34;
  const inset = compact ? 3 : 0;
  const centerY = height / 2;
  const amplitude = compact ? 5.3 : 8.6;
  const drawableWidth = width - inset * 2;
  const step =
    smoothedLevels.length > 1
      ? drawableWidth / (smoothedLevels.length - 1)
      : drawableWidth;
  const topPoints = smoothedLevels.map((level, index) => {
    const x = inset + index * step;
    const normalized = Math.max(0, level);
    const offset = (compact ? 1.3 : 2) + normalized * amplitude;
    return { x, y: centerY - offset };
  });
  const bottomPoints = smoothedLevels.map((level, index) => {
    const x = inset + index * step;
    const normalized = Math.max(0, level);
    const offset = (compact ? 1.3 : 2) + normalized * amplitude;
    return { x, y: centerY + offset };
  });
  const areaPath = [
    `M 0 ${centerY}`,
    ...topPoints.map((point) => `L ${point.x} ${point.y}`),
    `L ${width} ${centerY}`,
    ...bottomPoints
      .slice()
      .reverse()
      .map((point) => `L ${point.x} ${point.y}`),
    "Z",
  ].join(" ");
  const topPath = topPoints.length
    ? `M ${topPoints.map((point) => `${point.x} ${point.y}`).join(" L ")}`
    : "";
  const bottomPath = bottomPoints.length
    ? `M ${bottomPoints
        .map((point) => `${point.x} ${point.y}`)
        .join(" L ")}`
    : "";

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className={`signal-wave signal-wave-${tone} ${compact ? "signal-wave-compact" : ""}`}
      aria-hidden="true"
    >
      <path d={areaPath} className="signal-wave-fill" />
      <path d={topPath} className="signal-wave-line" />
      <path d={bottomPath} className="signal-wave-line" />
      <path
        d={`M 0 ${centerY} L ${width} ${centerY}`}
        className="signal-wave-center"
      />
    </svg>
  );
}

function ActionButton({
  className,
  state,
  idleLabel,
  workingLabel,
  doneLabel,
  idleIcon,
  workingIcon,
  doneIcon,
  onClick,
  disabled,
  iconOnly,
}: {
  className?: string;
  state?: ButtonFeedbackState;
  idleLabel: string;
  workingLabel?: string;
  doneLabel?: string;
  idleIcon?: ReactNode;
  workingIcon?: ReactNode;
  doneIcon?: ReactNode;
  onClick: () => void | Promise<void>;
  disabled?: boolean;
  iconOnly?: boolean;
}) {
  const label =
    state === "working"
      ? (workingLabel ?? idleLabel)
      : state === "done"
        ? (doneLabel ?? idleLabel)
        : idleLabel;
  const icon =
    state === "done"
      ? (doneIcon ?? idleIcon)
      : state === "working"
        ? (workingIcon ?? idleIcon)
        : idleIcon;

  return (
    <button
      className={[
        className,
        iconOnly ? "icon-only-button" : icon ? "icon-button" : "",
        state === "working" ? "action-button-working" : "",
        state === "done" ? "action-button-done" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
    >
      {icon ? (
        <span
          className={[
            "action-button-icon",
            state === "working" ? "action-button-icon-spin" : "",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          {icon}
        </span>
      ) : null}
      {iconOnly ? null : <span>{label}</span>}
    </button>
  );
}

function TranscriptionPill({
  phase,
  title,
  detail,
  levels,
  animationStyle,
  showLiveTranscription,
  onCancel,
}: {
  phase: AppPhase;
  title: string;
  detail: string;
  levels: number[];
  animationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
  onCancel?: (() => void) | null;
}) {
  const copy = detail.trim() || title;
  const usesRadialCore = animationStyle === "radial";
  const canCancel = phase === "recording" || phase === "transcribing";

  return (
    <div
      className={[
        "indicator-shell",
        `indicator-shell-${phase}`,
        "indicator-shell-inline",
        showLiveTranscription ? "indicator-shell-detail" : "indicator-shell-compact",
      ].join(" ")}
    >
      <div className="indicator-mark">
        {usesRadialCore ? null : (
          <button
            type="button"
            className={[
              "indicator-status-button",
              "indicator-status-button-inline",
              canCancel && onCancel ? "indicator-status-button-cancelable" : "",
              `indicator-status-button-${phase}`,
            ]
              .filter(Boolean)
              .join(" ")}
            onClick={() => onCancel?.()}
            disabled={!canCancel || !onCancel}
            aria-label={canCancel ? "Cancel current dictation" : "Dictation status"}
            title={canCancel ? "Cancel current dictation" : "Dictation status"}
          >
            <span className="indicator-dot" />
          </button>
        )}
        <div
          className={[
            "indicator-signal",
            usesRadialCore ? "" : "indicator-signal-linear",
            usesRadialCore ? "indicator-signal-radial" : "",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          <SignalBars
            phase={phase}
            levels={levels}
            compact
            animationStyle={animationStyle}
          />
          {usesRadialCore ? (
            <button
              type="button"
              className={[
                "indicator-status-button",
                "indicator-status-button-radial",
                canCancel && onCancel ? "indicator-status-button-cancelable" : "",
                `indicator-status-button-${phase}`,
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onCancel?.()}
              disabled={!canCancel || !onCancel}
              aria-label={canCancel ? "Cancel current dictation" : "Dictation status"}
              title={canCancel ? "Cancel current dictation" : "Dictation status"}
            >
              <span className="indicator-dot" />
            </button>
          ) : null}
        </div>
      </div>
      {showLiveTranscription ? (
        <div className="indicator-copy">
          <span>{copy}</span>
        </div>
      ) : null}
    </div>
  );
}

function IndicatorApp({ snapshot }: { snapshot: Snapshot | null }) {
  async function cancelFromOverlay() {
    try {
      await invoke("cancel_current_operation_command");
    } catch {
      // Keep the overlay interaction quiet if cancel fails.
    }
  }

  if (!snapshot || !snapshot.overlay.visible) {
    return <div className="indicator-root indicator-root-hidden" />;
  }

  return (
    <main className="indicator-root">
      <TranscriptionPill
        phase={snapshot.phase}
        title={snapshot.overlay.title}
        detail={snapshot.overlay.detail}
        levels={snapshot.overlay.levels}
        animationStyle={snapshot.settings.overlayAnimationStyle}
        showLiveTranscription={snapshot.settings.showLiveTranscription}
        onCancel={cancelFromOverlay}
      />
    </main>
  );
}

function ShortcutField({
  label,
  value,
  armed,
  onArm,
  onCapture,
  onCancel,
}: {
  label: string;
  value: string;
  armed: boolean;
  onArm: () => void;
  onCapture: (value: string) => void;
  onCancel: () => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <button
        type="button"
        className={`shortcut-button ${armed ? "shortcut-button-armed" : ""}`}
        onClick={onArm}
        onKeyDown={(event) => {
          event.preventDefault();
          const captured = captureShortcut(event);
          if (captured) {
            onCapture(captured);
          }
        }}
        onBlur={onCancel}
      >
        {armed ? "Press shortcut..." : value}
      </button>
    </label>
  );
}

function OverlayPositionPreview({
  position,
  large = false,
}: {
  position: EditableOverlayPosition;
  large?: boolean;
}) {
  return (
    <div
      className={[
        "position-preview",
        large ? "position-preview-large" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className={`position-preview-screen position-preview-screen-${position}`}>
        <span className="position-preview-pill" />
      </div>
    </div>
  );
}

function AnimationOptionPreview({
  style,
  large = false,
}: {
  style: OverlayAnimationStyle;
  large?: boolean;
}) {
  return (
    <div
      className={[
        "animation-choice-preview",
        large ? "animation-choice-preview-large" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="animation-choice-pill">
        <span className="animation-choice-dot" />
        <SignalBars
          phase="recording"
          levels={DEMO_LEVELS}
          compact
          animationStyle={style}
        />
      </div>
    </div>
  );
}

function InterfacePreviewCard({
  overlayPosition,
  animationStyle,
  showLiveTranscription,
}: {
  overlayPosition: EditableOverlayPosition;
  animationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
}) {
  return (
    <div className="interface-demo-frame">
      <div className={`interface-demo-screen interface-demo-screen-${overlayPosition}`}>
        <div className="interface-demo-pill">
          <TranscriptionPill
            phase="recording"
            title="Listening"
            detail="Live preview text"
            levels={DEMO_LEVELS}
            animationStyle={animationStyle}
            showLiveTranscription={showLiveTranscription}
          />
        </div>
      </div>
    </div>
  );
}

function ChoiceDropdown({
  label,
  value,
  options,
  onChange,
  placeholder = "Select",
  renderPreview,
}: {
  label: string;
  value: string;
  options: ChoiceOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  renderPreview?: (value: string, mode: "trigger" | "option") => ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const selected =
    options.find((option) => option.id === value) ?? options[0] ?? null;

  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: MouseEvent) {
      if (
        rootRef.current &&
        event.target instanceof Node &&
        !rootRef.current.contains(event.target)
      ) {
        setOpen(false);
      }
    }

    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open]);

  return (
    <div className="field">
      <span>{label}</span>
      <div
        ref={rootRef}
        className={`choice-dropdown ${open ? "choice-dropdown-open" : ""}`}
      >
        <button
          type="button"
          className="choice-trigger"
          onClick={() => setOpen((current) => !current)}
          aria-expanded={open}
          aria-haspopup="listbox"
          disabled={options.length === 0}
        >
          {selected && renderPreview ? (
            <span className="choice-preview">{renderPreview(selected.id, "trigger")}</span>
          ) : null}
          <div className="choice-trigger-copy">
            <strong>{selected?.label ?? placeholder}</strong>
            {selected?.description ? <span>{selected.description}</span> : null}
          </div>
          <ChevronDownIcon className="choice-chevron" />
        </button>

        {open && options.length > 0 ? (
          <div className="choice-menu" role="listbox">
            {options.map((option) => (
              <button
                key={option.id}
                type="button"
                className={`choice-option ${value === option.id ? "choice-option-active" : ""}`}
                onClick={() => {
                  onChange(option.id);
                  setOpen(false);
                }}
              >
                <div className="choice-option-main">
                  {renderPreview ? (
                    <span className="choice-preview choice-preview-option">
                      {renderPreview(option.id, "option")}
                    </span>
                  ) : null}
                  <div className="choice-option-copy">
                    <strong>{option.label}</strong>
                    {option.description ? <span>{option.description}</span> : null}
                  </div>
                </div>
                {value === option.id ? <CheckIcon className="choice-check" /> : null}
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function SidebarButton({
  active,
  label,
  section,
  collapsed,
  onClick,
}: {
  active: boolean;
  label: string;
  section: SectionId;
  collapsed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`sidebar-button ${active ? "sidebar-button-active" : ""}`}
      onClick={onClick}
      aria-label={label}
      title={label}
    >
      <SectionIcon section={section} className="sidebar-icon" />
      {collapsed ? null : <span>{label}</span>}
    </button>
  );
}

function StatusChip({
  label,
  tone = "muted",
  icon,
}: {
  label: string;
  tone?: "success" | "warning" | "danger" | "muted" | "accent";
  icon?: ReactNode;
}) {
  return (
    <span className={`status-chip status-chip-${tone}`}>
      {icon ? <span className="status-chip-icon">{icon}</span> : null}
      {label}
    </span>
  );
}

function SidebarToggleIcon({
  collapsed,
  className,
}: {
  collapsed: boolean;
  className?: string;
}) {
  return (
    <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" className={className}>
      <rect x="3.5" y="4.5" width="13" height="11" rx="2.4" />
      {collapsed ? (
        <path d="M 10.75 7.2 L 13.4 10 L 10.75 12.8" strokeLinecap="round" strokeLinejoin="round" />
      ) : (
        <path d="M 9.25 7.2 L 6.6 10 L 9.25 12.8" strokeLinecap="round" strokeLinejoin="round" />
      )}
      <path d="M 7.1 4.5 V 15.5" />
    </svg>
  );
}

function ModelPickerPreview({
  active = false,
  selectable = false,
}: {
  active?: boolean;
  selectable?: boolean;
}) {
  return (
    <span
      className={[
        "model-picker-dot",
        active ? "model-picker-dot-active" : "",
        !active && selectable ? "model-picker-dot-ready" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    />
  );
}

function ScoreMeter({
  value,
  kind,
}: {
  value: number;
  kind: "speed" | "accuracy";
}) {
  const rounded = Math.max(0, Math.min(5, Math.round(value)));

  return (
    <div className={`score-meter score-meter-${kind}`}>
      <div className="score-meter-icons" aria-hidden="true">
        {Array.from({ length: 5 }, (_, index) =>
          kind === "speed" ? (
            <BoltIcon
              key={`${kind}-${index}`}
              className={`score-bolt ${index < rounded ? "score-bolt-on" : ""}`}
            />
          ) : (
            <span
              key={`${kind}-${index}`}
              className={`score-dot ${index < rounded ? "score-dot-on" : ""}`}
            />
          ),
        )}
      </div>
      <span>{value.toFixed(1)}</span>
    </div>
  );
}

function ModelFeatureBadge({
  icon,
  label,
}: {
  icon: ReactNode;
  label: string;
}) {
  return (
    <span className="model-feature-badge" title={label} aria-label={label}>
      {icon}
    </span>
  );
}

function StatTile({
  icon,
  label,
  value,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  tone: "success" | "warning" | "danger" | "muted" | "accent";
}) {
  return (
    <article className={`tile tile-${tone}`}>
      <div className="tile-icon">{icon}</div>
      <div className="tile-copy">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </article>
  );
}

function NoticeBanner({
  kind,
  text,
  onDismiss,
}: {
  kind: "error";
  text: string;
  onDismiss: () => void;
}) {
  return (
    <div className={`notice notice-${kind}`}>
      <div className="notice-copy">{text}</div>
      <button
        type="button"
        className="notice-dismiss"
        onClick={onDismiss}
        aria-label="Dismiss message"
        title="Dismiss"
      >
        <CloseIcon className="small-icon" />
      </button>
    </div>
  );
}

function ControlApp({
  snapshot,
  setSnapshot,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
}) {
  const [activeSection, setActiveSection] = useState<SectionId>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "1";
    } catch {
      return false;
    }
  });
  const [message, setMessage] = useState<FlashMessage>(null);
  const [capturing, setCapturing] = useState<ShortcutFieldName | null>(null);
  const [historyQuery, setHistoryQuery] = useState("");
  const [modelQuery, setModelQuery] = useState("");
  const [modelFilter, setModelFilter] = useState<ModelFilter>("all");
  const [cleanupInput, setCleanupInput] = useState("");
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [buttonFeedback, setButtonFeedback] = useState<
    Record<string, ButtonFeedbackState>
  >({});
  const buttonFeedbackTimersRef = useRef<Record<string, number>>({});
  const [draft, setDraft] = useState<SettingsDraft>({
    holdShortcut: "",
    toggleShortcut: "",
    selectedSourceId: "",
    autoPaste: true,
    cleanupEnabled: true,
    audioRetentionPolicy: "one-day",
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    showLiveTranscription: false,
  });
  const draftRef = useRef(draft);

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        SIDEBAR_COLLAPSED_KEY,
        sidebarCollapsed ? "1" : "0",
      );
    } catch {
      // Ignore local preference persistence issues.
    }
  }, [sidebarCollapsed]);

  useEffect(() => {
    return () => {
      Object.values(buttonFeedbackTimersRef.current).forEach((timer) =>
        window.clearTimeout(timer),
      );
    };
  }, []);

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    setDraft({
      holdShortcut: snapshot.settings.holdShortcut,
      toggleShortcut: snapshot.settings.toggleShortcut,
      selectedSourceId:
        snapshot.settings.selectedSourceId ?? snapshot.sources[0]?.id ?? "",
      autoPaste: snapshot.settings.autoPaste,
      cleanupEnabled: snapshot.settings.cleanupEnabled,
      audioRetentionPolicy: snapshot.settings.audioRetentionPolicy,
      overlayPosition: normalizeEditableOverlayPosition(
        snapshot.settings.overlayPosition,
      ),
      overlayAnimationStyle: snapshot.settings.overlayAnimationStyle,
      showLiveTranscription: snapshot.settings.showLiveTranscription,
    });
  }, [snapshot]);

  async function refreshSnapshot() {
    const current = await getSnapshot();
    setSnapshot(current);
  }

  async function sendSettingsUpdate(update: Record<string, unknown>) {
    setMessage(null);

    try {
      await invoke("update_settings_command", { update });
    } catch (error) {
      await refreshSnapshot();
      throw error;
    }
  }

  async function dismissSnapshotError() {
    try {
      await invoke("clear_error_message_command");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  function clearButtonFeedback(actionId: string) {
    const timer = buttonFeedbackTimersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete buttonFeedbackTimersRef.current[actionId];
    }

    setButtonFeedback((current) => {
      if (!(actionId in current)) {
        return current;
      }

      const next = { ...current };
      delete next[actionId];
      return next;
    });
  }

  function setButtonFeedbackState(actionId: string, state: ButtonFeedbackState) {
    const timer = buttonFeedbackTimersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete buttonFeedbackTimersRef.current[actionId];
    }

    setButtonFeedback((current) => ({
      ...current,
      [actionId]: state,
    }));
  }

  function finishButtonFeedback(actionId: string, holdMs = 1200) {
    setButtonFeedbackState(actionId, "done");
    buttonFeedbackTimersRef.current[actionId] = window.setTimeout(() => {
      clearButtonFeedback(actionId);
    }, holdMs);
  }

  async function applySettings(update: Partial<SettingsDraft>) {
    if (!snapshot) {
      return;
    }

    const nextDraft = { ...draftRef.current, ...update };
    setDraft(nextDraft);
    draftRef.current = nextDraft;

    try {
      await sendSettingsUpdate(buildSettingsUpdate(nextDraft));
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function refreshDevices() {
    setMessage(null);
    setButtonFeedbackState("refresh-inputs", "working");

    try {
      await invoke("refresh_devices");
      await refreshSnapshot();
      finishButtonFeedback("refresh-inputs");
    } catch (error) {
      clearButtonFeedback("refresh-inputs");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function startRecording(mode: RecordingMode) {
    try {
      setMessage(null);
      await invoke("start_manual_recording", { mode });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function stopRecording() {
    try {
      setMessage(null);
      await invoke("stop_manual_recording");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function cancelCurrentOperation() {
    try {
      setMessage(null);
      await invoke("cancel_current_operation_command");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function copyHistory(id: string, text: string) {
    const actionId = `copy:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await navigator.clipboard.writeText(text);
      finishButtonFeedback(actionId, 1000);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openHistoryAudio(item: HistoryItem) {
    if (!item.audioPath) {
      return;
    }

    try {
      setMessage(null);
      await openPath(item.audioPath);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeHistoryItem(id: string) {
    const actionId = `history-remove:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_history_item", { id });
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  function selectModel(row: ModelRow) {
    setSelectedModelId(row.id);
  }

  async function activateModel(row: ModelRow) {
    if (!row.selectable || row.active) {
      return;
    }

    setMessage(null);
    try {
      await sendSettingsUpdate({
        selectedModelId: row.id,
        selectedModelKind: row.modelKind,
        selectedModelPath: row.source === "built-in" ? null : row.path ?? null,
      });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function linkCatalogModel(row: ModelRow) {
    if (!row.supportsInstall) {
      return;
    }

    const actionId = `model-link:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
      });

      if (!selected || Array.isArray(selected)) {
        clearButtonFeedback(actionId);
        return;
      }

      await invoke("install_catalog_model", {
        modelId: row.id,
        modelKind: row.modelKind,
        path: selected,
      });
      finishButtonFeedback(actionId);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function downloadCatalogModel(row: ModelRow) {
    if (!row.supportsDownload) {
      return;
    }

    const actionId = `model-download:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("download_catalog_model", {
        modelId: row.id,
      });
      finishButtonFeedback(actionId, 1500);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeCatalogModel(row: ModelRow) {
    if (!row.managed) {
      return;
    }

    const actionId = `model-remove:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_catalog_model", {
        modelId: row.id,
      });
      finishButtonFeedback(actionId, 1200);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openModelReference(row: ModelRow) {
    if (!row.hfUrl) {
      return;
    }

    try {
      await openUrl(row.hfUrl);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openModelArtifact(row: ModelRow) {
    if (!row.artifactUrl) {
      return;
    }

    try {
      await openUrl(row.artifactUrl);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function addCleanupTerm(term = cleanupInput) {
    const trimmed = term.trim();
    if (!trimmed) {
      setMessage({
        kind: "error",
        text: "Enter a filler word or phrase to remove.",
      });
      return;
    }

    setMessage(null);
    setButtonFeedbackState("cleanup-add", "working");

    try {
      await invoke("add_cleanup_term", { term: trimmed });
      setCleanupInput("");
      finishButtonFeedback("cleanup-add");
    } catch (error) {
      clearButtonFeedback("cleanup-add");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeCleanupTerm(term: string) {
    const actionId = `cleanup-remove:${term}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_cleanup_term", { term });
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function restoreCleanupDefaults() {
    setMessage(null);
    setButtonFeedbackState("cleanup-restore", "working");

    try {
      await invoke("restore_default_cleanup_terms");
      finishButtonFeedback("cleanup-restore");
    } catch (error) {
      clearButtonFeedback("cleanup-restore");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  const modelRows = snapshot ? buildModelRows(snapshot) : [];
  const activeModelId = snapshot?.settings.selectedModelId ?? "parakeet";
  const selectedRowExists = selectedModelId
    ? modelRows.some((row) => row.id === selectedModelId)
    : false;
  const resolvedSelectedModelId = selectedRowExists
    ? selectedModelId
    : modelRows.some((row) => row.id === activeModelId)
      ? activeModelId
      : modelRows[0]?.id ?? "parakeet";
  const selectedModel =
    modelRows.find((row) => row.id === resolvedSelectedModelId) ?? modelRows[0];
  const activeModel =
    modelRows.find((row) => row.active) ??
    modelRows.find((row) => row.id === activeModelId) ??
    null;
  const selectedModelFit = snapshot && selectedModel
    ? describeHardwareFit(selectedModel, snapshot.systemProfile)
    : null;
  const readyModelOptions: ChoiceOption[] = modelRows
    .filter((row) => row.selectable)
    .map((row) => ({
      id: row.id,
      label: row.name,
      description: `${row.provider} · ${formatModelSizeLabel(row)}`,
    }));
  const activeReadyModelId =
    activeModel?.selectable && activeModel ? activeModel.id : readyModelOptions[0]?.id ?? "";
  const selectedModelMeta = selectedModel
    ? [
        ["Runtime", selectedModel.runtime],
        [
          "Download size",
          selectedModel.downloadSizeBytes
            ? formatBytes(selectedModel.downloadSizeBytes)
            : "Included / n.a.",
        ],
        ["Size on disk", formatBytes(selectedModel.diskSizeBytes)],
        ["Languages", selectedModel.languages],
        ["Speed", selectedModel.speed],
        ["Quality", selectedModel.quality],
        ["Footprint", selectedModel.footprint],
        ["License", selectedModel.license],
        ["This Mac", selectedModelFit?.label ?? "Unknown"],
        ["Hardware", snapshot ? formatSystemProfile(snapshot.systemProfile) : "Unknown"],
      ]
    : [];

  useEffect(() => {
    if (!selectedRowExists && modelRows[0]) {
      setSelectedModelId(activeModelId || modelRows[0].id);
    }
  }, [activeModelId, modelRows, selectedRowExists]);

  useEffect(() => {
    function handleCancelEscape(event: globalThis.KeyboardEvent) {
      if (event.key !== "Escape") {
        return;
      }

      if (!snapshot || (snapshot.phase !== "recording" && snapshot.phase !== "transcribing")) {
        return;
      }

      event.preventDefault();
      void cancelCurrentOperation();
    }

    document.addEventListener("keydown", handleCancelEscape);
    return () => {
      document.removeEventListener("keydown", handleCancelEscape);
    };
  }, [snapshot]);

  async function chooseDefaultModel(modelId: string) {
    const row = modelRows.find((candidate) => candidate.id === modelId);
    if (!row) {
      return;
    }

    setSelectedModelId(row.id);
    if (row.selectable) {
      await activateModel(row);
    }
  }

  if (!snapshot) {
    return <main className="loading-shell">Loading...</main>;
  }

  const activeSource =
    snapshot.sources.find((source) => source.id === draft.selectedSourceId) ??
    snapshot.sources.find((source) => source.isDefault) ??
    snapshot.sources[0] ??
    null;
  const sourceOptions: ChoiceOption[] = snapshot.sources.map((source) => ({
    id: source.id,
    label: source.name,
    description: `${source.sampleRate} Hz · ${source.channels} ch${source.isDefault ? " · default" : ""}`,
  }));
  const recentTranscript = snapshot.history[0] ?? null;
  const filteredHistory = snapshot.history.filter((item) =>
    matchesHistory(item, historyQuery),
  );
  const previewTitle =
    snapshot.phase === "recording"
      ? "Listening"
      : snapshot.phase === "transcribing"
        ? "Transcribing"
        : "Ready";
  const previewDetail = snapshot.overlay.detail || previewTitle;
  const filteredModels = modelRows.filter((row) =>
    matchesModel(row, modelQuery, modelFilter),
  );
  const cleanupTerms = snapshot.settings.cleanupTerms;
  const availableCleanupSuggestions = CLEANUP_SUGGESTIONS.filter(
    (term) => !cleanupTerms.includes(term),
  );

  return (
    <main className={`workspace-shell ${sidebarCollapsed ? "workspace-shell-collapsed" : ""}`}>
      <aside className={`sidebar ${sidebarCollapsed ? "sidebar-collapsed" : ""}`}>
        <div className="sidebar-brand">
          <div className="sidebar-brand-mark">
            <SparkIcon className="brand-icon" />
          </div>
          {sidebarCollapsed ? null : (
            <div className="sidebar-brand-copy">
              <strong>Transcribed</strong>
              <span>Local dictation</span>
            </div>
          )}
          <button
            type="button"
            className="sidebar-toggle"
            onClick={() => setSidebarCollapsed((value) => !value)}
            aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <SidebarToggleIcon collapsed={sidebarCollapsed} className="sidebar-toggle-icon" />
          </button>
        </div>

        <nav className="sidebar-nav">
          {sections.map((section) => (
            <SidebarButton
              key={section.id}
              active={activeSection === section.id}
              label={section.label}
              section={section.id}
              collapsed={sidebarCollapsed}
              onClick={() => setActiveSection(section.id)}
            />
          ))}
        </nav>

        {sidebarCollapsed ? null : (
          <div className="sidebar-footer">
            <StatusChip
              label={formatPhaseLabel(snapshot.phase)}
              tone={toneForPhase(snapshot.phase)}
            />
            <StatusChip label="Local" tone="accent" />
          </div>
        )}
      </aside>

      <section className="workspace-main">
        <header className="workspace-header">
          <div className="workspace-title">
            <SectionIcon section={activeSection} className="workspace-title-icon" />
            <h1>{sections.find((section) => section.id === activeSection)?.label}</h1>
          </div>

          <div className="header-actions">
            <StatusChip
              label={
                snapshot.modelStatus === "ready"
                  ? `${activeModel?.family ?? "Model"} ready`
                  : `${activeModel?.family ?? "Model"} missing`
              }
              tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
              icon={<CheckIcon className="chip-icon-svg" />}
            />
            <StatusChip
              label={snapshot.shortcutsActive ? "Keys active" : "Keys off"}
              tone={snapshot.shortcutsActive ? "accent" : "warning"}
            />
          </div>
        </header>

        {message ? (
          <NoticeBanner
            kind={message.kind}
            text={message.text}
            onDismiss={() => setMessage(null)}
          />
        ) : null}
        {!message && snapshot.errorMessage ? (
          <NoticeBanner
            kind="error"
            text={snapshot.errorMessage}
            onDismiss={() => {
              void dismissSnapshotError();
            }}
          />
        ) : null}

        <div className="content-stack">
          {activeSection === "overview" ? (
            <>
              <section className="tile-grid">
                <StatTile
                  icon={<CheckIcon className="tile-icon-svg" />}
                  label="Engine"
                  value={
                    activeModel
                      ? `${activeModel.name}${snapshot.modelStatus === "ready" ? "" : " (missing)"}`
                      : "No model"
                  }
                  tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
                />
                <StatTile
                  icon={<InputIcon className="tile-icon-svg" />}
                  label="Input"
                  value={activeSource?.name ?? "No source"}
                  tone="accent"
                />
                <StatTile
                  icon={<HistoryIcon className="tile-icon-svg" />}
                  label="History"
                  value={`${snapshot.history.length} saved`}
                  tone="muted"
                />
                <StatTile
                  icon={<KeysIcon className="tile-icon-svg" />}
                  label="Keys"
                  value={snapshot.shortcutsActive ? "Active" : "Unavailable"}
                  tone={snapshot.shortcutsActive ? "accent" : "warning"}
                />
              </section>

              <section className="surface preview-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Live</span>
                    <StatusChip
                      label={previewTitle}
                      tone={toneForPhase(snapshot.phase)}
                    />
                  </div>
                  <div className="inline-actions">
                    {snapshot.phase === "recording" ? (
                      <button className="secondary" onClick={stopRecording}>
                        Stop
                      </button>
                    ) : (
                      <>
                        <button
                          className="secondary"
                          onClick={() => startRecording("hold")}
                        >
                          Hold
                        </button>
                        <button
                          className="secondary"
                          onClick={() => startRecording("toggle")}
                        >
                          Toggle
                        </button>
                      </>
                    )}
                  </div>
                </div>

                <div className="pill-stage">
                  <TranscriptionPill
                    phase={snapshot.phase}
                    title={snapshot.overlay.title || previewTitle}
                    detail={previewDetail}
                    levels={snapshot.overlay.levels}
                    animationStyle={draft.overlayAnimationStyle}
                    showLiveTranscription={draft.showLiveTranscription}
                    onCancel={() => {
                      void cancelCurrentOperation();
                    }}
                  />
                </div>

                <div className="mini-meta-row">
                  <span>{activeSource?.sampleRate ?? 0} Hz</span>
                  <span>{activeSource?.channels ?? 0} ch</span>
                  <span>{recentTranscript ? "Last saved locally" : "No transcript yet"}</span>
                </div>
              </section>
            </>
          ) : null}

          {activeSection === "models" ? (
            <>
              <section className="surface model-library-surface">
                <div className="model-library-head">
                  <div className="model-library-copy">
                    <span className="surface-title-label">Speech models</span>
                    <p>Choose the local ASR engine Transcribed should use by default.</p>
                  </div>

                  <div className="model-type-tabs" aria-label="Model categories">
                    {modelTypeTabs.map((tab) => (
                      <button
                        key={tab.id}
                        type="button"
                        className={`model-type-tab ${tab.active ? "model-type-tab-active" : ""}`}
                        disabled={!tab.active}
                      >
                        {tab.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="model-selector-row">
                  <div className="model-selector-card">
                    <ChoiceDropdown
                      label="Default speech model"
                      value={activeReadyModelId}
                      options={readyModelOptions}
                      placeholder="Download a model"
                      renderPreview={(value) => {
                        const row = modelRows.find((candidate) => candidate.id === value);
                        return (
                          <ModelPickerPreview
                            active={Boolean(row?.active)}
                            selectable={Boolean(row?.selectable)}
                          />
                        );
                      }}
                      onChange={(value) => {
                        void chooseDefaultModel(value);
                      }}
                    />
                  </div>

                  <div className="model-selector-meta">
                    <StatusChip
                      label={
                        activeModel
                          ? `${activeModel.name} active`
                          : "No active model"
                      }
                      tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
                    />
                    {selectedModelFit ? (
                      <StatusChip
                        label={selectedModelFit.label}
                        tone={selectedModelFit.tone}
                      />
                    ) : null}
                    <span className="model-selector-footnote">
                      {formatSystemProfile(snapshot.systemProfile)}
                    </span>
                  </div>
                </div>

                <div className="toolbar model-toolbar">
                  <label className="search-field">
                    <SearchIcon className="search-icon" />
                    <input
                      type="search"
                      value={modelQuery}
                      onChange={(event) => setModelQuery(event.currentTarget.value)}
                      placeholder="Filter models"
                    />
                  </label>

                  <div className="segmented">
                    {modelFilters.map((filter) => (
                      <button
                        key={filter.id}
                        type="button"
                        className={`segment ${modelFilter === filter.id ? "segment-active" : ""}`}
                        onClick={() => setModelFilter(filter.id)}
                      >
                        {filter.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="table-shell model-library-table-shell">
                  <table className="model-table model-library-table">
                    <thead>
                      <tr>
                        <th />
                        <th>Model</th>
                        <th>Features</th>
                        <th>Speed</th>
                        <th>Accuracy</th>
                        <th />
                      </tr>
                    </thead>
                    <tbody>
                      {filteredModels.map((row) => {
                        const hardwareFit = describeHardwareFit(
                          row,
                          snapshot.systemProfile,
                        );

                        return (
                          <tr
                            key={row.id}
                            className={[
                              "model-row",
                              row.id === resolvedSelectedModelId
                                ? "model-row-selected"
                                : "",
                              row.active ? "model-row-active" : "",
                              row.state === "planned" || row.state === "incomplete"
                                ? "model-row-dim"
                                : "",
                            ]
                              .filter(Boolean)
                              .join(" ")}
                            onClick={() => {
                              selectModel(row);
                            }}
                          >
                            <td className="model-radio-cell">
                              <span
                                className={[
                                  "model-radio",
                                  row.id === resolvedSelectedModelId
                                    ? "model-radio-selected"
                                    : "",
                                  row.active ? "model-radio-active" : "",
                                ]
                                  .filter(Boolean)
                                  .join(" ")}
                              />
                            </td>
                            <td className="model-main-cell">
                              <div className="model-entry">
                                <div className="model-entry-head">
                                  <strong>{row.name}</strong>
                                  {row.active ? (
                                    <span className="model-inline-pill">Default</span>
                                  ) : null}
                                </div>
                                <div className="model-entry-meta">
                                  <span>{row.provider}</span>
                                  <span>{row.languages}</span>
                                  <span>{formatModelSizeLabel(row)}</span>
                                </div>
                              </div>
                            </td>
                            <td>
                              <div className="model-feature-list">
                                {modelFeatureItems(row).map((feature) => (
                                  <ModelFeatureBadge
                                    key={`${row.id}-${feature.id}`}
                                    icon={feature.icon}
                                    label={feature.label}
                                  />
                                ))}
                              </div>
                            </td>
                            <td>
                              <ScoreMeter value={modelSpeedScore(row)} kind="speed" />
                            </td>
                            <td>
                              <ScoreMeter value={modelAccuracyScore(row)} kind="accuracy" />
                            </td>
                            <td className="model-actions-cell">
                              <div className="model-row-actions">
                                {row.active ? (
                                  <StatusChip label="Active" tone="success" />
                                ) : row.selectable ? (
                                  <button
                                    className="secondary small"
                                    onClick={(event) => {
                                      event.stopPropagation();
                                      void activateModel(row);
                                    }}
                                  >
                                    Use
                                  </button>
                                ) : null}
                                {row.supportsDownload && !row.selectable ? (
                                  <ActionButton
                                    className="secondary small"
                                    state={buttonFeedback[`model-download:${row.id}`]}
                                    idleLabel="Download"
                                    workingLabel="Downloading"
                                    doneLabel="Downloaded"
                                    idleIcon={<DownloadIcon className="small-icon" />}
                                    workingIcon={<DownloadIcon className="small-icon" />}
                                    doneIcon={<CheckIcon className="small-icon" />}
                                    onClick={() => downloadCatalogModel(row)}
                                    iconOnly
                                  />
                                ) : null}
                                {row.supportsInstall ? (
                                  <ActionButton
                                    className="secondary small"
                                    state={buttonFeedback[`model-link:${row.id}`]}
                                    idleLabel={row.path ? "Choose another file" : "Use local file"}
                                    workingLabel="Linking"
                                    doneLabel="Linked"
                                    idleIcon={<FolderIcon className="small-icon" />}
                                    doneIcon={<CheckIcon className="small-icon" />}
                                    onClick={() => linkCatalogModel(row)}
                                    iconOnly
                                  />
                                ) : null}
                                {row.managed ? (
                                  <ActionButton
                                    className="secondary small"
                                    state={buttonFeedback[`model-remove:${row.id}`]}
                                    idleLabel="Delete downloaded model"
                                    workingLabel="Deleting"
                                    doneLabel="Deleted"
                                    idleIcon={<TrashIcon className="small-icon" />}
                                    doneIcon={<CheckIcon className="small-icon" />}
                                    onClick={() => removeCatalogModel(row)}
                                    iconOnly
                                  />
                                ) : null}
                                {row.hfUrl ? (
                                  <button
                                    className="secondary small icon-only-button"
                                    onClick={(event) => {
                                      event.stopPropagation();
                                      void openModelReference(row);
                                    }}
                                    aria-label="Open Hugging Face"
                                    title="Open Hugging Face"
                                  >
                                    <ExternalIcon className="small-icon" />
                                  </button>
                                ) : null}
                              </div>
                              <span className="model-row-footnote">
                                {row.state === "planned"
                                  ? "Reference only"
                                  : row.diskSizeBytes
                                    ? formatBytes(row.diskSizeBytes)
                                    : row.downloadSizeBytes
                                      ? `~${formatBytes(row.downloadSizeBytes)}`
                                      : hardwareFit.label}
                              </span>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>

                  {filteredModels.length === 0 ? (
                    <div className="empty-state">No models match.</div>
                  ) : null}
                </div>
                {selectedModel ? (
                  <article className="model-focus-card">
                    <div className="surface-bar model-focus-head">
                      <div className="surface-title">
                        <span className="surface-title-label">{selectedModel.name}</span>
                        <span className="model-focus-subtitle">
                          {selectedModel.provider} · {selectedModel.architecture}
                        </span>
                      </div>
                      <div className="header-actions">
                        <StatusChip
                          label={
                            selectedModel.active
                              ? "Active"
                              : selectedModel.selectable
                                ? "Ready"
                                : selectedModel.state === "downloadable"
                                  ? "Downloadable"
                                  : "Planned"
                          }
                          tone={
                            selectedModel.active
                              ? "success"
                              : selectedModel.selectable
                                ? "success"
                                : selectedModel.state === "downloadable"
                                  ? "accent"
                                  : "warning"
                          }
                        />
                        {selectedModelFit ? (
                          <StatusChip
                            label={selectedModelFit.label}
                            tone={selectedModelFit.tone}
                          />
                        ) : null}
                      </div>
                    </div>

                    <div className="model-focus-layout">
                      <div className="model-focus-copy">
                        <div className="detail-copy model-focus-copy-block">
                          <p>{selectedModel.summary}</p>
                          <p>{selectedModel.note}</p>
                          {selectedModelFit ? <p>{selectedModelFit.detail}</p> : null}
                          {selectedModel.path ? (
                            <code className="path-chip">{selectedModel.path}</code>
                          ) : null}
                        </div>

                        <div className="inline-actions model-focus-actions">
                          {selectedModel.supportsDownload ? (
                            <ActionButton
                              className="secondary"
                              state={buttonFeedback[`model-download:${selectedModel.id}`]}
                              idleLabel={
                                selectedModel.path
                                  ? "Re-download from Hugging Face"
                                  : "Download from Hugging Face"
                              }
                              workingLabel="Downloading"
                              doneLabel="Downloaded"
                              idleIcon={<DownloadIcon className="small-icon" />}
                              workingIcon={<DownloadIcon className="small-icon" />}
                              doneIcon={<CheckIcon className="small-icon" />}
                              onClick={() => downloadCatalogModel(selectedModel)}
                            />
                          ) : null}
                          {selectedModel.supportsInstall ? (
                            <ActionButton
                              className="secondary"
                              state={buttonFeedback[`model-link:${selectedModel.id}`]}
                              idleLabel={
                                selectedModel.path ? "Use another file" : "Use existing file"
                              }
                              workingLabel="Linking"
                              doneLabel="Linked"
                              idleIcon={<FolderIcon className="small-icon" />}
                              doneIcon={<CheckIcon className="small-icon" />}
                              onClick={() => linkCatalogModel(selectedModel)}
                            />
                          ) : null}
                          {selectedModel.managed ? (
                            <ActionButton
                              className="secondary"
                              state={buttonFeedback[`model-remove:${selectedModel.id}`]}
                              idleLabel="Delete downloaded model"
                              workingLabel="Deleting"
                              doneLabel="Deleted"
                              idleIcon={<TrashIcon className="small-icon" />}
                              doneIcon={<CheckIcon className="small-icon" />}
                              onClick={() => removeCatalogModel(selectedModel)}
                            />
                          ) : null}
                          {selectedModel.selectable && !selectedModel.active ? (
                            <button
                              className="secondary"
                              onClick={() => {
                                void activateModel(selectedModel);
                              }}
                            >
                              Use model
                            </button>
                          ) : null}
                          {selectedModel.artifactUrl ? (
                            <button
                              className="secondary"
                              onClick={() => {
                                void openModelArtifact(selectedModel);
                              }}
                            >
                              Open compatible file
                            </button>
                          ) : null}
                          {selectedModel.hfUrl ? (
                            <button
                              className="secondary"
                              onClick={() => {
                                void openModelReference(selectedModel);
                              }}
                            >
                              Open Hugging Face
                            </button>
                          ) : null}
                        </div>

                        <ul className="detail-list model-focus-highlights">
                          {selectedModel.highlights.map((highlight) => (
                            <li key={highlight}>{highlight}</li>
                          ))}
                        </ul>
                      </div>

                      <div className="model-focus-metrics">
                        <div className="model-focus-meta-list">
                          {selectedModelMeta.map(([label, value]) => (
                            <div className="model-focus-meta-row" key={label}>
                              <span>{label}</span>
                              <strong>{value}</strong>
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  </article>
                ) : null}
              </section>
            </>
          ) : null}

          {activeSection === "keybindings" ? (
            <section className="compact-grid-two">
              <article className="surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Global shortcuts</span>
                  </div>
                </div>
                <div className="field-grid">
                  <ShortcutField
                    label="Hold"
                    value={draft.holdShortcut}
                    armed={capturing === "holdShortcut"}
                    onArm={() => setCapturing("holdShortcut")}
                    onCapture={(value) => {
                      void applySettings({ holdShortcut: value });
                      setCapturing(null);
                    }}
                    onCancel={() => setCapturing(null)}
                  />
                  <ShortcutField
                    label="Toggle"
                    value={draft.toggleShortcut}
                    armed={capturing === "toggleShortcut"}
                    onArm={() => setCapturing("toggleShortcut")}
                    onCapture={(value) => {
                      void applySettings({ toggleShortcut: value });
                      setCapturing(null);
                    }}
                    onCancel={() => setCapturing(null)}
                  />
                </div>
                <div className="mini-meta-row">
                  <span>{snapshot.shortcutMessage}</span>
                  <span>{snapshot.shortcutsActive ? "Ready globally" : "Unavailable globally"}</span>
                </div>
                <div className="detail-copy">
                  <p>Press Esc while recording or transcribing to cancel the current dictation.</p>
                </div>
              </article>

              <article className="surface preference-note-surface">
                <div className="surface-title">
                  <span className="surface-title-label">Recording</span>
                </div>
                <label className="toggle-row toggle-row-card">
                  <input
                    type="checkbox"
                    checked={draft.autoPaste}
                    onChange={(event) =>
                      void applySettings({
                        autoPaste: event.currentTarget.checked,
                      })
                    }
                  />
                  <div>
                    <strong>Auto paste</strong>
                    <span>Paste after the final transcript is ready.</span>
                  </div>
                </label>
              </article>
            </section>
          ) : null}

          {activeSection === "interface" ? (
            <section className="compact-grid-two">
              <article className="surface preference-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Indicator</span>
                  </div>
                </div>

                <div className="setting-list">
                  <div className="setting-row">
                    <div className="setting-copy">
                      <strong>HUD position</strong>
                      <span>Where the pill sits while dictating.</span>
                    </div>
                    <div className="setting-control">
                      <ChoiceDropdown
                        label="Position"
                        value={draft.overlayPosition}
                        options={overlayPositionOptions}
                        renderPreview={(value) => (
                          <OverlayPositionPreview
                            position={value as EditableOverlayPosition}
                          />
                        )}
                        onChange={(value) =>
                          void applySettings({
                            overlayPosition: value as EditableOverlayPosition,
                          })
                        }
                      />
                    </div>
                  </div>

                  <div className="setting-row">
                    <div className="setting-copy">
                      <strong>Animation</strong>
                      <span>Choose a simpler live meter style.</span>
                    </div>
                    <div className="setting-control">
                      <ChoiceDropdown
                        label="Style"
                        value={draft.overlayAnimationStyle}
                        options={overlayAnimationOptions}
                        renderPreview={(value) => (
                          <AnimationOptionPreview
                            style={value as OverlayAnimationStyle}
                          />
                        )}
                        onChange={(value) =>
                          void applySettings({
                            overlayAnimationStyle: value as OverlayAnimationStyle,
                          })
                        }
                      />
                    </div>
                  </div>

                  <label className="toggle-row toggle-row-card setting-toggle">
                    <input
                      type="checkbox"
                      checked={draft.showLiveTranscription}
                      onChange={(event) =>
                        void applySettings({
                          showLiveTranscription: event.currentTarget.checked,
                        })
                      }
                    />
                    <div>
                      <strong>Show live transcription</strong>
                      <span>Expand the pill with draft text while speaking.</span>
                    </div>
                  </label>
                </div>

                <div className="mini-meta-row">
                  <span>{formatOverlayPosition(draft.overlayPosition)}</span>
                  <span>{formatOverlayAnimationStyle(draft.overlayAnimationStyle)}</span>
                  <span>{draft.showLiveTranscription ? "Expanded HUD" : "Compact HUD"}</span>
                </div>
              </article>

              <article className="surface preview-surface interface-preview-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Preview</span>
                  </div>
                </div>

                <InterfacePreviewCard
                  overlayPosition={draft.overlayPosition}
                  animationStyle={draft.overlayAnimationStyle}
                  showLiveTranscription={draft.showLiveTranscription}
                />

                <div className="interface-preview-grid">
                  <div className="interface-preview-mini">
                    <span>Placement</span>
                    <OverlayPositionPreview
                      position={draft.overlayPosition}
                      large
                    />
                  </div>
                  <div className="interface-preview-mini">
                    <span>Meter</span>
                    <AnimationOptionPreview
                      style={draft.overlayAnimationStyle}
                      large
                    />
                  </div>
                </div>
              </article>
            </section>
          ) : null}

          {activeSection === "cleanup" ? (
            <section className="compact-grid-two">
              <article className="surface preference-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Transcript cleanup</span>
                  </div>
                </div>

                <div className="setting-list">
                  <label className="toggle-row toggle-row-card setting-toggle">
                    <input
                      type="checkbox"
                      checked={draft.cleanupEnabled}
                      onChange={(event) =>
                        void applySettings({
                          cleanupEnabled: event.currentTarget.checked,
                        })
                      }
                    />
                    <div>
                      <strong>Remove filler words</strong>
                      <span>Clean transcripts before paste and history save.</span>
                    </div>
                  </label>

                  <div className="field">
                    <span>Word or phrase</span>
                    <div className="inline-actions cleanup-add-row">
                      <input
                        value={cleanupInput}
                        onChange={(event) => setCleanupInput(event.currentTarget.value)}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            void addCleanupTerm();
                          }
                        }}
                        placeholder="e.g. um, uh, you know"
                      />
                      <ActionButton
                        state={buttonFeedback["cleanup-add"]}
                        idleLabel="Add"
                        workingLabel="Adding"
                        doneLabel="Added"
                        doneIcon={<CheckIcon className="small-icon" />}
                        onClick={() => addCleanupTerm()}
                      />
                    </div>
                  </div>

                  {availableCleanupSuggestions.length > 0 ? (
                    <div className="cleanup-suggestions">
                      {availableCleanupSuggestions.map((term) => (
                        <button
                          key={term}
                          type="button"
                          className="secondary cleanup-suggestion"
                          onClick={() => {
                            void addCleanupTerm(term);
                          }}
                        >
                          {term}
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>

                <div className="mini-meta-row">
                  <span>{draft.cleanupEnabled ? "Cleanup on" : "Cleanup off"}</span>
                  <span>{cleanupTerms.length} terms</span>
                </div>
              </article>

              <article className="surface preference-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Managed terms</span>
                  </div>
                  <ActionButton
                    className="secondary small"
                    state={buttonFeedback["cleanup-restore"]}
                    idleLabel="Defaults"
                    workingLabel="Restoring"
                    doneLabel="Restored"
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={restoreCleanupDefaults}
                  />
                </div>

                {cleanupTerms.length === 0 ? (
                  <div className="empty-state cleanup-empty-state">
                    No cleanup terms yet.
                  </div>
                ) : (
                  <div className="cleanup-list">
                    {cleanupTerms.map((term) => (
                      <div className="cleanup-chip" key={term}>
                        <span>{term}</span>
                        <ActionButton
                          className="secondary small cleanup-chip-remove"
                          state={buttonFeedback[`cleanup-remove:${term}`]}
                          idleLabel="Remove"
                          doneLabel="Removed"
                          doneIcon={<CheckIcon className="small-icon" />}
                          onClick={() => removeCleanupTerm(term)}
                        />
                      </div>
                    ))}
                  </div>
                )}
              </article>
            </section>
          ) : null}

          {activeSection === "inputs" ? (
            <>
              <section className="compact-grid-two">
                <article className="surface">
                  <ChoiceDropdown
                    label="Source"
                    value={draft.selectedSourceId}
                    options={sourceOptions}
                    placeholder="No source"
                    onChange={(value) =>
                      void applySettings({
                        selectedSourceId: value,
                      })
                    }
                  />

                  <div className="mini-meta-row">
                    <span>{activeSource?.sampleRate ?? 0} Hz</span>
                    <span>{activeSource?.channels ?? 0} ch</span>
                    <span>{activeSource?.isDefault ? "Default" : "Manual"}</span>
                  </div>

                  <div className="inline-actions">
                    <ActionButton
                      className="secondary"
                      state={buttonFeedback["refresh-inputs"]}
                      idleLabel="Refresh"
                      workingLabel="Refreshing"
                      doneLabel="Updated"
                      idleIcon={<RefreshIcon className="small-icon" />}
                      workingIcon={<RefreshIcon className="small-icon" />}
                      doneIcon={<CheckIcon className="small-icon" />}
                      onClick={refreshDevices}
                    />
                    {snapshot.phase === "recording" ? (
                      <button className="secondary" onClick={stopRecording}>
                        Stop
                      </button>
                    ) : (
                      <>
                        <button
                          className="secondary"
                          onClick={() => startRecording("hold")}
                        >
                          Hold
                        </button>
                        <button
                          className="secondary"
                          onClick={() => startRecording("toggle")}
                        >
                          Toggle
                        </button>
                      </>
                    )}
                  </div>
                </article>

                <article className="surface preview-surface">
                  <div className="surface-bar">
                    <div className="surface-title">
                      <span className="surface-title-label">Preview</span>
                    </div>
                  </div>
                  <div className="pill-stage pill-stage-wide">
                    <TranscriptionPill
                      phase={snapshot.phase}
                      title={snapshot.overlay.title || previewTitle}
                      detail={previewDetail}
                      levels={snapshot.overlay.levels}
                      animationStyle={draft.overlayAnimationStyle}
                      showLiveTranscription={draft.showLiveTranscription}
                      onCancel={() => {
                        void cancelCurrentOperation();
                      }}
                    />
                  </div>
                </article>
              </section>
            </>
          ) : null}

          {activeSection === "history" ? (
            <>
              <section className="compact-grid-two">
                <article className="surface preference-surface">
                  <div className="surface-bar">
                    <div className="surface-title">
                      <span className="surface-title-label">Search</span>
                    </div>
                  </div>

                  <label className="search-field">
                    <SearchIcon className="search-icon" />
                    <input
                      type="search"
                      value={historyQuery}
                      onChange={(event) => setHistoryQuery(event.currentTarget.value)}
                      placeholder="Search transcripts"
                    />
                  </label>

                  <div className="mini-meta-row">
                    <span>{snapshot.history.length} saved</span>
                    <span>{filteredHistory.length} visible</span>
                  </div>
                </article>

                <article className="surface preference-surface">
                  <div className="surface-bar">
                    <div className="surface-title">
                      <span className="surface-title-label">Audio clips</span>
                    </div>
                  </div>

                  <div className="setting-list">
                    <div className="field">
                      <span>Keep original captured audio</span>
                      <div className="segmented">
                        {audioRetentionOptions.map((option) => (
                          <button
                            key={option.id}
                            type="button"
                            className={`segment ${draft.audioRetentionPolicy === option.id ? "segment-active" : ""}`}
                            onClick={() =>
                              void applySettings({
                                audioRetentionPolicy: option.id,
                              })
                            }
                          >
                            {option.label}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>

                  <div className="mini-meta-row">
                    <span>{formatAudioRetentionPolicy(draft.audioRetentionPolicy)}</span>
                    <span>Transcript text stays until you remove it</span>
                  </div>
                </article>
              </section>

              {filteredHistory.length === 0 ? (
                <section className="surface empty-state">
                  {snapshot.history.length === 0
                    ? "No transcripts yet."
                    : "No matches."}
                </section>
              ) : (
                <section className="history-list">
                  {filteredHistory.map((item) => (
                    <article className="surface history-row" key={item.id}>
                      <div className="history-row-head">
                        <div className="history-meta">
                          <span>{new Date(item.createdAt).toLocaleString()}</span>
                          <span>{item.sourceName}</span>
                          <span>{item.mode}</span>
                          <span>{formatDuration(item.durationMs)}</span>
                          {item.audioPath ? <span>Audio saved</span> : null}
                        </div>
                        <div className="history-actions">
                          {item.audioPath ? (
                            <button
                              className="secondary small icon-only-button"
                              onClick={() => {
                                void openHistoryAudio(item);
                              }}
                              aria-label="Open audio"
                              title="Open audio"
                            >
                              <ExternalIcon className="small-icon" />
                            </button>
                          ) : null}
                          <ActionButton
                            className="secondary small"
                            state={buttonFeedback[`copy:${item.id}`]}
                            idleLabel="Copy"
                            doneLabel="Copied"
                            idleIcon={<CopyIcon className="small-icon" />}
                            doneIcon={<CheckIcon className="small-icon" />}
                            onClick={() => copyHistory(item.id, item.text)}
                            iconOnly
                          />
                          <ActionButton
                            className="secondary small"
                            state={buttonFeedback[`history-remove:${item.id}`]}
                            idleLabel="Remove"
                            workingLabel="Removing"
                            doneLabel="Removed"
                            idleIcon={<TrashIcon className="small-icon" />}
                            doneIcon={<CheckIcon className="small-icon" />}
                            onClick={() => removeHistoryItem(item.id)}
                            iconOnly
                          />
                        </div>
                      </div>
                      <p>{item.text}</p>
                    </article>
                  ))}
                </section>
              )}
            </>
          ) : null}

          {activeSection === "about" ? (
            <section className="compact-grid-three">
              <article className="surface info-tile">
                <CheckIcon className="tile-icon-svg" />
                <strong>Local only</strong>
                <span>Audio, paste, and history stay on-device.</span>
              </article>
              <article className="surface info-tile">
                <FolderIcon className="tile-icon-svg" />
                <strong>Tray-first</strong>
                <span>Close hides the window and keeps hotkeys alive.</span>
              </article>
              <article className="surface info-tile">
                <ExternalIcon className="tile-icon-svg" />
                <strong>Next</strong>
                <span>Better live preview, more Parakeet variants, vector search.</span>
              </article>
            </section>
          ) : null}
        </div>
      </section>
    </main>
  );
}

export default function App() {
  const [snapshot, setSnapshot] = useSnapshotState();

  useEffect(() => {
    document.documentElement.classList.toggle("indicator-window", isIndicatorWindow);
    document.body.classList.toggle("indicator-window", isIndicatorWindow);

    return () => {
      document.documentElement.classList.remove("indicator-window");
      document.body.classList.remove("indicator-window");
    };
  }, []);

  return isIndicatorWindow ? (
    <IndicatorApp snapshot={snapshot} />
  ) : (
    <ControlApp snapshot={snapshot} setSnapshot={setSnapshot} />
  );
}

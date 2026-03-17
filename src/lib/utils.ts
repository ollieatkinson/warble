import type { KeyboardEvent } from "react";

import type {
  AccelerationProvider,
  AppPhase,
  AudioRetentionPolicy,
  ColorTheme,
  EditableOverlayPosition,
  HistoryItem,
  InferenceProvider,
  LivePreviewModel,
  LiveTranscriptLines,
  LiveTranscriptWidth,
  OverlayAnimationStyle,
  OverlayPosition,
  Snapshot,
  SettingsDraft,
  StatusTone,
  SystemProfile,
} from "../types";

export function formatDuration(durationMs: number) {
  const seconds = durationMs / 1000;
  return `${seconds.toFixed(seconds > 10 ? 0 : 1)}s`;
}

export function formatElapsedClock(durationMs: number) {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }

  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function formatPhaseLabel(phase: AppPhase) {
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

export function formatOverlayPosition(position: OverlayPosition) {
  switch (position) {
    case "dynamic-island":
      return "Dynamic Island";
    case "top-left":
      return "Top left";
    case "top-right":
      return "Top right";
    case "top-center":
      return "Top center";
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

export function formatOverlayAnimationStyle(style: OverlayAnimationStyle) {
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

export function formatLivePreviewModel(model: LivePreviewModel) {
  switch (model) {
    case "nemotron-streaming":
      return "Nemotron";
    case "parakeet-eou":
      return "Realtime EOU";
    case "auto":
    default:
      return "Auto";
  }
}

export function formatLiveTranscriptWidth(width: LiveTranscriptWidth) {
  switch (width) {
    case "compact":
      return "Compact";
    case "wide":
      return "Wide";
    case "balanced":
    default:
      return "Balanced";
  }
}

export function formatLiveTranscriptLines(lines: LiveTranscriptLines) {
  switch (lines) {
    case "one":
      return "1 line";
    case "three":
      return "3 lines";
    case "two":
    default:
      return "2 lines";
  }
}

export function formatColorTheme(theme: ColorTheme) {
  switch (theme) {
    case "light":
      return "Light";
    case "dark":
      return "Dark";
    case "system":
    default:
      return "System";
  }
}

export function formatAudioRetentionPolicy(policy: AudioRetentionPolicy) {
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

export function formatBytes(bytes?: number | null) {
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

export function formatEta(seconds?: number | null) {
  if (seconds == null) {
    return null;
  }

  if (seconds <= 0) {
    return "Almost done";
  }

  return `${formatElapsedClock(seconds * 1000)} left`;
}

export function formatSystemProfile(profile: SystemProfile) {
  const memoryLabel =
    profile.totalMemoryBytes > 0
      ? `${formatBytes(profile.totalMemoryBytes)} RAM`
      : "RAM unknown";
  const coreLabel = `${profile.logicalCores} threads`;
  const parts = [memoryLabel, coreLabel];

  if (profile.gpuName) {
    parts.push(profile.gpuName);
  }
  if (profile.gpuMemoryBytes > 0) {
    parts.push(`${formatBytes(profile.gpuMemoryBytes)} VRAM`);
  }
  if (profile.supportedAccelerationProviders.length > 0) {
    parts.push(formatAccelerationProviders(profile.supportedAccelerationProviders));
  }

  return parts.join(" · ");
}

export function formatAccelerationProvider(provider: AccelerationProvider) {
  switch (provider) {
    case "directml":
      return "DirectML";
    case "webgpu":
      return "WebGPU";
    default:
      return provider;
  }
}

export function formatAccelerationProviders(
  providers: AccelerationProvider[],
  fallback = "CPU only",
) {
  if (providers.length === 0) {
    return fallback;
  }

  return providers.map(formatAccelerationProvider).join(" + ");
}

export function formatInferenceProvider(provider: InferenceProvider) {
  switch (provider) {
    case "directml":
      return "DirectML";
    case "webgpu":
      return "WebGPU";
    case "cpu":
    default:
      return "CPU";
  }
}

export function formatCaptureInput(sampleRate: number, channels: number) {
  const parts: string[] = [];
  if (sampleRate > 0) {
    parts.push(`${Math.round(sampleRate / 100) / 10} kHz`);
  }
  if (channels > 0) {
    parts.push(`${channels} ch`);
  }

  return parts.join(" · ") || "Unknown input";
}

export function buildSupportReport(snapshot: Snapshot) {
  const selectedSource =
    snapshot.sources.find((source) => source.id === snapshot.settings.selectedSourceId) ?? null;
  const previewEvents =
    snapshot.previewDiagnostics.recentEvents.length > 0
      ? snapshot.previewDiagnostics.recentEvents.map((event) => `- ${event}`).join("\n")
      : "- No live preview events recorded";
  const captureEvents =
    snapshot.captureDiagnostics.recentEvents.length > 0
      ? snapshot.captureDiagnostics.recentEvents.map((event) => `- ${event}`).join("\n")
      : "- No capture events recorded";

  return [
    "Warble support report",
    `Generated: ${new Date().toISOString()}`,
    "",
    `Platform: ${snapshot.platform}`,
    `Phase: ${snapshot.phase}`,
    `Status: ${snapshot.statusMessage}`,
    `Error: ${snapshot.errorMessage ?? "None"}`,
    "",
    "Selection",
    `Source: ${
      selectedSource
        ? `${selectedSource.name} (${formatCaptureInput(
            selectedSource.sampleRate,
            selectedSource.channels,
          )})`
        : snapshot.settings.selectedSourceId ?? "None"
    }`,
    `Model: ${snapshot.settings.selectedModelId} (${snapshot.settings.selectedModelKind})`,
    `Live preview model: ${snapshot.settings.livePreviewModel}`,
    "",
    "Capture diagnostics",
    `Status: ${snapshot.captureDiagnostics.status}`,
    `Detail: ${snapshot.captureDiagnostics.detail || "None"}`,
    `Source: ${snapshot.captureDiagnostics.sourceName || "Unknown"}`,
    `Input: ${formatCaptureInput(
      snapshot.captureDiagnostics.sampleRate,
      snapshot.captureDiagnostics.channels,
    )}`,
    `Buffered samples: ${snapshot.captureDiagnostics.lastBufferedSamples || 0}`,
    `Log: ${snapshot.captureDiagnostics.logPath ?? "Unavailable"}`,
    captureEvents,
    "",
    "Live preview diagnostics",
    `Backend: ${snapshot.previewDiagnostics.backend || "Unknown"}`,
    `Status: ${snapshot.previewDiagnostics.status}`,
    `Detail: ${snapshot.previewDiagnostics.detail || "None"}`,
    `Log: ${snapshot.previewDiagnostics.logPath ?? "Unavailable"}`,
    previewEvents,
  ].join("\n");
}

export function toneForPhase(phase: AppPhase): StatusTone {
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

export function normalizeEditableOverlayPosition(
  position: OverlayPosition,
): EditableOverlayPosition {
  switch (position) {
    case "dynamic-island":
    case "top-center":
    case "top-left":
    case "top-right":
    case "bottom-left":
    case "bottom-right":
    case "bottom-center":
      return position;
    case "caret":
    default:
      return "bottom-center";
  }
}

export function formatInvokeError(error: unknown) {
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "Something went wrong.";
}

export function parseReplacementVariantsInput(value: string) {
  return value
    .split(/[\n,]+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

export function formatShortcutKey(key: string) {
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

type ShortcutCaptureEvent = Pick<
  KeyboardEvent<HTMLButtonElement>,
  "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey"
>;

export function captureShortcut(event: ShortcutCaptureEvent) {
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

const MODIFIER_CODE_MAP: Record<string, string> = {
  MetaRight: "RightMeta",
  MetaLeft: "LeftMeta",
  ControlRight: "RightControl",
  ControlLeft: "LeftControl",
  ShiftRight: "RightShift",
  ShiftLeft: "LeftShift",
  AltRight: "RightAlt",
  AltLeft: "LeftAlt",
};

export function captureModifierShortcut(code: string): string | null {
  return MODIFIER_CODE_MAP[code] ?? null;
}

export function isModifierOnlyShortcut(shortcut: string): boolean {
  return shortcut in Object.fromEntries(
    Object.values(MODIFIER_CODE_MAP).map((v) => [v, true]),
  );
}

export function formatModifierShortcutLabel(shortcut: string): string {
  switch (shortcut) {
    case "RightMeta": return "Right \u2318";
    case "LeftMeta": return "Left \u2318";
    case "RightControl": return "Right Ctrl";
    case "LeftControl": return "Left Ctrl";
    case "RightShift": return "Right Shift";
    case "LeftShift": return "Left Shift";
    case "RightAlt": return "Right Alt";
    case "LeftAlt": return "Left Alt";
    default: return shortcut;
  }
}

export function matchesHistory(item: HistoryItem, query: string) {
  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  if (terms.length === 0) {
    return true;
  }

  const haystack = [item.text, item.sourceName, item.mode, item.pasted ? "pasted" : "saved"]
    .concat([
      item.capture.modelName,
      formatInferenceProvider(item.capture.inferenceProvider),
      formatCaptureInput(item.capture.inputSampleRate, item.capture.inputChannels),
    ])
    .join(" ")
    .toLowerCase();

  return terms.every((term) => haystack.includes(term));
}

export function buildSettingsUpdate(draft: SettingsDraft) {
  return {
    holdShortcut: draft.holdShortcut,
    toggleShortcut: draft.toggleShortcut,
    pasteLastShortcut: draft.pasteLastShortcut,
    selectedSourceId: draft.selectedSourceId || undefined,
    autoPaste: draft.autoPaste,
    cleanupEnabled: draft.cleanupEnabled,
    audioRetentionPolicy: draft.audioRetentionPolicy,
    overlayPosition: draft.overlayPosition,
    overlayAnimationStyle: draft.overlayAnimationStyle,
    livePreviewModel: draft.livePreviewModel,
    liveTranscriptWidth: draft.liveTranscriptWidth,
    liveTranscriptLines: draft.liveTranscriptLines,
    showRecordingTimer: draft.showRecordingTimer,
    showLiveTranscription: draft.showLiveTranscription,
    colorTheme: draft.colorTheme,
  };
}

export function resampleLevels(sourceLevels: number[], count: number) {
  return Array.from({ length: count }, (_, index) => {
    if (count <= 1 || sourceLevels.length === 1) {
      return sourceLevels[0] ?? 0.14;
    }

    const position = (index / (count - 1)) * (sourceLevels.length - 1);
    const nearestIndex = Math.round(position);
    return sourceLevels[nearestIndex] ?? sourceLevels[0] ?? 0.14;
  });
}

export function smoothLevels(levels: number[]) {
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

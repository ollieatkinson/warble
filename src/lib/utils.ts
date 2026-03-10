import type { KeyboardEvent } from "react";

import type {
  AppPhase,
  AudioRetentionPolicy,
  EditableOverlayPosition,
  HistoryItem,
  InferenceProvider,
  LivePreviewModel,
  OverlayAnimationStyle,
  OverlayPosition,
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
  if (profile.directmlAvailable) {
    parts.push("DirectML ready");
  }

  return parts.join(" · ");
}

export function formatInferenceProvider(provider: InferenceProvider) {
  switch (provider) {
    case "directml":
      return "DirectML";
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
    case "bottom-left":
    case "bottom-right":
      return position;
    case "caret":
    case "bottom-center":
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

export function captureShortcut(event: KeyboardEvent<HTMLButtonElement>) {
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
    selectedSourceId: draft.selectedSourceId || undefined,
    autoPaste: draft.autoPaste,
    cleanupEnabled: draft.cleanupEnabled,
    audioRetentionPolicy: draft.audioRetentionPolicy,
    overlayPosition: draft.overlayPosition,
    overlayAnimationStyle: draft.overlayAnimationStyle,
    livePreviewModel: draft.livePreviewModel,
    showRecordingTimer: draft.showRecordingTimer,
    showLiveTranscription: draft.showLiveTranscription,
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

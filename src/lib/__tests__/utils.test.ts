import type { KeyboardEvent } from "react";

import {
  buildSupportReport,
  formatDuration,
  formatElapsedClock,
  formatPhaseLabel,
  formatOverlayPosition,
  formatOverlayAnimationStyle,
  formatLivePreviewModel,
  formatLiveTranscriptWidth,
  formatLiveTranscriptLines,
  formatColorTheme,
  formatAudioRetentionPolicy,
  formatBytes,
  formatEta,
  formatSystemProfile,
  formatAccelerationProvider,
  formatAccelerationProviders,
  formatInferenceProvider,
  formatCaptureInput,
  toneForPhase,
  normalizeEditableOverlayPosition,
  formatInvokeError,
  formatShortcutKey,
  captureShortcut,
  matchesHistory,
  buildSettingsUpdate,
  resampleLevels,
  smoothLevels,
} from "../utils";

import {
  createHistoryItem,
  createSettings,
  createSnapshot,
  createSettingsDraft,
  createSystemProfile,
} from "../../test/fixtures";

describe("formatDuration", () => {
  it("shows one decimal for short durations", () => {
    expect(formatDuration(2500)).toBe("2.5s");
    expect(formatDuration(500)).toBe("0.5s");
  });

  it("rounds to whole seconds above 10s", () => {
    expect(formatDuration(15000)).toBe("15s");
    expect(formatDuration(60500)).toBe("61s");
  });

  it("handles zero", () => {
    expect(formatDuration(0)).toBe("0.0s");
  });
});

describe("formatElapsedClock", () => {
  it("formats minutes and seconds", () => {
    expect(formatElapsedClock(65000)).toBe("1:05");
    expect(formatElapsedClock(0)).toBe("0:00");
  });

  it("includes hours when >= 3600s", () => {
    expect(formatElapsedClock(3661000)).toBe("1:01:01");
  });

  it("clamps negative values to 0:00", () => {
    expect(formatElapsedClock(-5000)).toBe("0:00");
  });
});

describe("formatPhaseLabel", () => {
  it.each([
    ["recording", "Recording"],
    ["transcribing", "Transcribing"],
    ["error", "Attention"],
    ["idle", "Idle"],
  ] as const)("maps %s to %s", (phase, label) => {
    expect(formatPhaseLabel(phase)).toBe(label);
  });
});

describe("formatOverlayPosition", () => {
  it.each([
    ["dynamic-island", "Dynamic Island"],
    ["top-left", "Top left"],
    ["top-right", "Top right"],
    ["top-center", "Top center"],
    ["bottom-left", "Bottom left"],
    ["bottom-right", "Bottom right"],
    ["caret", "Near caret"],
    ["bottom-center", "Bottom center"],
  ] as const)("maps %s to %s", (pos, label) => {
    expect(formatOverlayPosition(pos)).toBe(label);
  });
});

describe("formatOverlayAnimationStyle", () => {
  it.each([
    ["radial", "Radial"],
    ["waveform", "Wave"],
    ["spectrum", "Bars"],
  ] as const)("maps %s to %s", (style, label) => {
    expect(formatOverlayAnimationStyle(style)).toBe(label);
  });
});

describe("formatLivePreviewModel", () => {
  it.each([
    ["nemotron-streaming", "Nemotron"],
    ["parakeet-eou", "Realtime EOU"],
    ["auto", "Auto"],
  ] as const)("maps %s to %s", (model, label) => {
    expect(formatLivePreviewModel(model)).toBe(label);
  });
});

describe("formatLiveTranscriptWidth", () => {
  it.each([
    ["compact", "Compact"],
    ["wide", "Wide"],
    ["balanced", "Balanced"],
  ] as const)("maps %s to %s", (width, label) => {
    expect(formatLiveTranscriptWidth(width)).toBe(label);
  });
});

describe("formatLiveTranscriptLines", () => {
  it.each([
    ["one", "1 line"],
    ["three", "3 lines"],
    ["two", "2 lines"],
  ] as const)("maps %s to %s", (lines, label) => {
    expect(formatLiveTranscriptLines(lines)).toBe(label);
  });
});

describe("formatColorTheme", () => {
  it.each([
    ["light", "Light"],
    ["dark", "Dark"],
    ["system", "System"],
  ] as const)("maps %s to %s", (theme, label) => {
    expect(formatColorTheme(theme)).toBe(label);
  });
});

describe("formatAudioRetentionPolicy", () => {
  it.each([
    ["seven-days", "7 days"],
    ["thirty-days", "30 days"],
    ["one-day", "24 hours"],
  ] as const)("maps %s to %s", (policy, label) => {
    expect(formatAudioRetentionPolicy(policy)).toBe(label);
  });
});

describe("formatBytes", () => {
  it("returns 'Not installed' for null, zero, and negative", () => {
    expect(formatBytes(null)).toBe("Not installed");
    expect(formatBytes(0)).toBe("Not installed");
    expect(formatBytes(-1)).toBe("Not installed");
    expect(formatBytes(undefined)).toBe("Not installed");
  });

  it("formats bytes", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("formats kilobytes", () => {
    expect(formatBytes(2048)).toBe("2.0 KB");
  });

  it("formats megabytes and gigabytes", () => {
    expect(formatBytes(1024 * 1024 * 500)).toBe("500 MB");
    expect(formatBytes(1024 ** 3 * 2)).toBe("2.0 GB");
  });

  it("drops decimals for values >= 100", () => {
    expect(formatBytes(1024 * 200)).toBe("200 KB");
  });
});

describe("formatEta", () => {
  it("returns null for null/undefined", () => {
    expect(formatEta(null)).toBeNull();
    expect(formatEta(undefined)).toBeNull();
  });

  it("returns 'Almost done' for zero and negative", () => {
    expect(formatEta(0)).toBe("Almost done");
    expect(formatEta(-5)).toBe("Almost done");
  });

  it("formats positive seconds with clock and suffix", () => {
    expect(formatEta(90)).toBe("1:30 left");
  });
});

describe("formatSystemProfile", () => {
  it("joins all profile parts", () => {
    const profile = createSystemProfile();
    const result = formatSystemProfile(profile);
    expect(result).toContain("RAM");
    expect(result).toContain("8 threads");
    expect(result).toContain("NVIDIA RTX 3060");
    expect(result).toContain("VRAM");
    expect(result).toContain("DirectML");
    expect(result.split(" \u00B7 ").length).toBe(5);
  });

  it("omits gpu fields when absent", () => {
    const profile = createSystemProfile({
      gpuName: "",
      gpuMemoryBytes: 0,
      supportedAccelerationProviders: [],
    });
    const result = formatSystemProfile(profile);
    expect(result.split(" \u00B7 ").length).toBe(2);
  });
});

describe("formatAccelerationProvider", () => {
  it("maps known providers", () => {
    expect(formatAccelerationProvider("directml")).toBe("DirectML");
    expect(formatAccelerationProvider("webgpu")).toBe("WebGPU");
  });

  it("passes unknown providers through", () => {
    expect(formatAccelerationProvider("cuda" as never)).toBe("cuda");
  });
});

describe("formatAccelerationProviders", () => {
  it("returns fallback for empty array", () => {
    expect(formatAccelerationProviders([])).toBe("CPU only");
    expect(formatAccelerationProviders([], "None")).toBe("None");
  });

  it("joins multiple providers", () => {
    expect(formatAccelerationProviders(["directml", "webgpu"])).toBe(
      "DirectML + WebGPU",
    );
  });
});

describe("formatInferenceProvider", () => {
  it.each([
    ["directml", "DirectML"],
    ["webgpu", "WebGPU"],
    ["cpu", "CPU"],
  ] as const)("maps %s to %s", (p, label) => {
    expect(formatInferenceProvider(p)).toBe(label);
  });
});

describe("formatCaptureInput", () => {
  it("formats sample rate and channels", () => {
    expect(formatCaptureInput(48000, 2)).toBe("48 kHz \u00B7 2 ch");
  });

  it("returns 'Unknown input' when both are zero", () => {
    expect(formatCaptureInput(0, 0)).toBe("Unknown input");
  });

  it("shows only available parts", () => {
    expect(formatCaptureInput(16000, 0)).toBe("16 kHz");
    expect(formatCaptureInput(0, 1)).toBe("1 ch");
  });
});

describe("buildSupportReport", () => {
  it("includes source selection and diagnostic events", () => {
    const snapshot = createSnapshot({
      phase: "transcribing",
      statusMessage: "Working",
      errorMessage: "Disk busy",
      previewDiagnostics: {
        backend: "nemotron",
        status: "warming",
        detail: "Loading preview model",
        recentEvents: ["Preview booted", "Streaming chunk 1"],
        logPath: "/tmp/preview.log",
      },
      captureDiagnostics: {
        status: "listening",
        detail: "Microphone active",
        recentEvents: ["Mic selected", "Samples flowing"],
        logPath: "/tmp/capture.log",
        sourceName: "Built-in Microphone",
        sampleRate: 48000,
        channels: 2,
        lastBufferedSamples: 2048,
      },
    });

    const report = buildSupportReport(snapshot);

    expect(report).toContain("Warble support report");
    expect(report).toContain("Platform: windows");
    expect(report).toContain("Phase: transcribing");
    expect(report).toContain("Status: Working");
    expect(report).toContain("Error: Disk busy");
    expect(report).toContain("Source: Built-in Microphone (48 kHz \u00B7 2 ch)");
    expect(report).toContain("Model: parakeet (parakeet)");
    expect(report).toContain("Buffered samples: 2048");
    expect(report).toContain("Backend: nemotron");
    expect(report).toContain("- Mic selected");
    expect(report).toContain("- Streaming chunk 1");
  });

  it("falls back when selection and events are unavailable", () => {
    const snapshot = createSnapshot({
      settings: createSettings({ selectedSourceId: null }),
      sources: [],
      errorMessage: null,
      captureDiagnostics: {
        status: "idle",
        detail: "",
        recentEvents: [],
        logPath: null,
        sourceName: "",
        sampleRate: 0,
        channels: 0,
        lastBufferedSamples: 0,
      },
      previewDiagnostics: {
        backend: "",
        status: "idle",
        detail: "",
        recentEvents: [],
        logPath: null,
      },
    });

    const report = buildSupportReport(snapshot);

    expect(report).toContain("Error: None");
    expect(report).toContain("Source: None");
    expect(report).toContain("Detail: None");
    expect(report).toContain("Input: Unknown input");
    expect(report).toContain("Source: Unknown");
    expect(report).toContain("Backend: Unknown");
    expect(report).toContain("Log: Unavailable");
    expect(report).toContain("- No capture events recorded");
    expect(report).toContain("- No live preview events recorded");
  });
});

describe("toneForPhase", () => {
  it.each([
    ["recording", "danger"],
    ["transcribing", "warning"],
    ["error", "danger"],
    ["idle", "success"],
  ] as const)("maps %s to %s", (phase, tone) => {
    expect(toneForPhase(phase)).toBe(tone);
  });
});

describe("normalizeEditableOverlayPosition", () => {
  it.each([
    "dynamic-island",
    "top-center",
    "top-left",
    "top-right",
    "bottom-center",
    "bottom-left",
    "bottom-right",
  ] as const)("keeps %s editable", (position) => {
    expect(normalizeEditableOverlayPosition(position)).toBe(position);
  });

  it("normalizes caret to bottom-center", () => {
    expect(normalizeEditableOverlayPosition("caret")).toBe("bottom-center");
  });
});

describe("formatInvokeError", () => {
  it("returns string errors directly", () => {
    expect(formatInvokeError("Boom")).toBe("Boom");
  });

  it("extracts message from Error instances", () => {
    expect(formatInvokeError(new Error("Oops"))).toBe("Oops");
  });

  it("returns fallback for other types", () => {
    expect(formatInvokeError(42)).toBe("Something went wrong.");
    expect(formatInvokeError(null)).toBe("Something went wrong.");
  });
});

describe("formatShortcutKey", () => {
  it("maps space to 'Space'", () => {
    expect(formatShortcutKey(" ")).toBe("Space");
  });

  it("uppercases F-keys", () => {
    expect(formatShortcutKey("f1")).toBe("F1");
    expect(formatShortcutKey("F12")).toBe("F12");
  });

  it("maps arrow keys", () => {
    expect(formatShortcutKey("ArrowUp")).toBe("Up");
    expect(formatShortcutKey("ArrowDown")).toBe("Down");
  });

  it("uppercases single characters", () => {
    expect(formatShortcutKey("a")).toBe("A");
  });

  it("passes through known named keys", () => {
    expect(formatShortcutKey("Escape")).toBe("Escape");
    expect(formatShortcutKey("Enter")).toBe("Enter");
  });
});

describe("captureShortcut", () => {
  function fakeEvent(
    overrides: Partial<KeyboardEvent<HTMLButtonElement>>,
  ): KeyboardEvent<HTMLButtonElement> {
    return {
      key: "",
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: false,
      ...overrides,
    } as KeyboardEvent<HTMLButtonElement>;
  }

  it("returns null for bare modifier keys", () => {
    expect(captureShortcut(fakeEvent({ key: "Control" }))).toBeNull();
    expect(captureShortcut(fakeEvent({ key: "Shift" }))).toBeNull();
  });

  it("builds combo string with modifiers", () => {
    expect(
      captureShortcut(
        fakeEvent({ key: "a", ctrlKey: true, shiftKey: true }),
      ),
    ).toBe("Ctrl+Shift+A");
  });

  it("handles plain key without modifiers", () => {
    expect(captureShortcut(fakeEvent({ key: " " }))).toBe("Space");
  });
});

describe("matchesHistory", () => {
  it("matches everything on empty query", () => {
    expect(matchesHistory(createHistoryItem(), "")).toBe(true);
    expect(matchesHistory(createHistoryItem(), "   ")).toBe(true);
  });

  it("matches against item text", () => {
    expect(matchesHistory(createHistoryItem({ text: "foo bar" }), "foo")).toBe(
      true,
    );
  });

  it("requires all terms to match (AND logic)", () => {
    const item = createHistoryItem({ text: "hello world" });
    expect(matchesHistory(item, "hello world")).toBe(true);
    expect(matchesHistory(item, "hello missing")).toBe(false);
  });

  it("matches against capture metadata", () => {
    const item = createHistoryItem();
    expect(matchesHistory(item, "Parakeet")).toBe(true);
    expect(matchesHistory(item, "CPU")).toBe(true);
  });
});

describe("buildSettingsUpdate", () => {
  it("maps draft fields to update object", () => {
    const draft = createSettingsDraft();
    const update = buildSettingsUpdate(draft);
    expect(update.holdShortcut).toBe(draft.holdShortcut);
    expect(update.autoPaste).toBe(draft.autoPaste);
    expect(update.colorTheme).toBe(draft.colorTheme);
  });

  it("converts empty selectedSourceId to undefined", () => {
    const draft = createSettingsDraft({ selectedSourceId: "" });
    expect(buildSettingsUpdate(draft).selectedSourceId).toBeUndefined();
  });

  it("preserves non-empty selectedSourceId", () => {
    const draft = createSettingsDraft({ selectedSourceId: "mic-1" });
    expect(buildSettingsUpdate(draft).selectedSourceId).toBe("mic-1");
  });
});

describe("resampleLevels", () => {
  it("resamples to requested count", () => {
    expect(resampleLevels([0, 0.5, 1], 5)).toHaveLength(5);
  });

  it("defaults to 0.14 for empty source", () => {
    expect(resampleLevels([], 3)).toEqual([0.14, 0.14, 0.14]);
  });

  it("returns single source value when count is 1", () => {
    expect(resampleLevels([0.8, 0.2], 1)).toEqual([0.8]);
  });
});

describe("smoothLevels", () => {
  it("preserves length", () => {
    expect(smoothLevels([1, 2, 3, 4, 5])).toHaveLength(5);
  });

  it("applies weighted average", () => {
    const result = smoothLevels([0, 0, 1, 0, 0]);
    // Center element: 0*0.1 + 0*0.2 + 1*0.4 + 0*0.2 + 0*0.1 = 0.4
    expect(result[2]).toBeCloseTo(0.4);
    // Neighbor: 0*0.1 + 0*0.2 + 0*0.4 + 1*0.2 + 0*0.1 = 0.2
    expect(result[1]).toBeCloseTo(0.2);
  });

  it("handles single element", () => {
    // All fallbacks equal the element itself: v*0.1 + v*0.2 + v*0.4 + v*0.2 + v*0.1 = v
    expect(smoothLevels([0.5])[0]).toBeCloseTo(0.5);
  });
});

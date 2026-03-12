import { renderHook, waitFor } from "@testing-library/react";
import { vi } from "vitest";

import { createSettingsDraft, createSnapshot } from "../../test/fixtures";

const tauriApiMocks = vi.hoisted(() => ({
  primeAutoPasteAccess: vi.fn(() => Promise.resolve()),
  primeMicrophoneAccess: vi.fn(() => Promise.resolve()),
  refreshDevices: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../lib/tauriApi", () => ({
  primeAutoPasteAccess: tauriApiMocks.primeAutoPasteAccess,
  primeMicrophoneAccess: tauriApiMocks.primeMicrophoneAccess,
  refreshDevices: tauriApiMocks.refreshDevices,
}));

vi.mock("../useButtonFeedback", () => ({
  useButtonFeedback: () => ({
    buttonFeedback: {},
    clearButtonFeedback: vi.fn(),
    finishButtonFeedback: vi.fn(),
    setButtonFeedbackState: vi.fn(),
  }),
}));

vi.mock("../useCleanupActions", () => ({
  useCleanupActions: () => ({
    cleanupInput: "",
    setCleanupInput: vi.fn(),
    addCleanupTerm: vi.fn(),
    removeCleanupTerm: vi.fn(),
    restoreCleanupDefaults: vi.fn(),
  }),
}));

vi.mock("../useHistoryActions", () => ({
  useHistoryActions: () => ({
    historyQuery: "",
    setHistoryQuery: vi.fn(),
    copyHistory: vi.fn(),
    openHistoryAudio: vi.fn(),
    removeHistoryItem: vi.fn(),
    clearHistory: vi.fn(),
  }),
}));

vi.mock("../useModelActions", () => ({
  useModelActions: () => ({
    activateModel: vi.fn(),
    downloadCatalogModel: vi.fn(),
    removeCatalogModel: vi.fn(),
    openModelReference: vi.fn(),
  }),
}));

vi.mock("../useRecordingActions", () => ({
  useRecordingActions: () => ({
    startRecording: vi.fn(),
    stopRecording: vi.fn(),
    cancelCurrentOperation: vi.fn(),
    transcribeFile: vi.fn(),
  }),
}));

vi.mock("../useSettingsSync", () => ({
  useSettingsSync: ({
    snapshot,
  }: {
    snapshot: ReturnType<typeof createSnapshot> | null;
  }) => {
    const draft = createSettingsDraft({
      autoPaste: snapshot?.settings.autoPaste ?? true,
      selectedSourceId: snapshot?.settings.selectedSourceId ?? "default-mic",
    });
    return {
      draft,
      applySettings: vi.fn(),
      dismissSnapshotError: vi.fn(),
      refreshSnapshot: vi.fn(() => Promise.resolve()),
      sendSettingsUpdate: vi.fn(),
      draftRef: { current: draft },
    };
  },
}));

vi.mock("../useTheme", () => ({
  useTheme: vi.fn(),
}));

import { useControlApp } from "../useControlApp";

describe("useControlApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("primes macOS microphone access even when sources are already listed", async () => {
    const snapshot = createSnapshot({
      platform: "macos",
      settings: { ...createSnapshot().settings, autoPaste: false },
    });

    renderHook(() =>
      useControlApp({
        snapshot,
        setSnapshot: vi.fn(),
      }),
    );

    await waitFor(() => {
      expect(tauriApiMocks.primeMicrophoneAccess).toHaveBeenCalledTimes(1);
    });
    expect(tauriApiMocks.primeAutoPasteAccess).not.toHaveBeenCalled();
    expect(tauriApiMocks.refreshDevices).not.toHaveBeenCalled();
  });

  it("primes macOS auto-paste access when auto-paste is enabled", async () => {
    const snapshot = createSnapshot({
      platform: "macos",
      settings: { ...createSnapshot().settings, autoPaste: true },
    });

    renderHook(() =>
      useControlApp({
        snapshot,
        setSnapshot: vi.fn(),
      }),
    );

    await waitFor(() => {
      expect(tauriApiMocks.primeMicrophoneAccess).toHaveBeenCalledTimes(1);
      expect(tauriApiMocks.primeAutoPasteAccess).toHaveBeenCalledTimes(1);
    });
  });
});

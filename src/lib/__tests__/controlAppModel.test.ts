import {
  buildDraftFromSnapshot,
  deriveSourceState,
  deriveModelState,
  deriveHistoryState,
  derivePreviewState,
  findModelById,
  mapInstalledStreamingModels,
} from "../controlAppModel";
import {
  createSnapshot,
  createSettings,
  createSettingsDraft,
  createHistoryItem,
  createSourceInfo,
  createModelRow,
} from "../../test/fixtures";

describe("buildDraftFromSnapshot", () => {
  it("maps all settings fields from snapshot to draft", () => {
    const snapshot = createSnapshot({
      settings: createSettings({
        holdShortcut: "Ctrl+H",
        toggleShortcut: "Ctrl+T",
        selectedSourceId: "mic-2",
        autoPaste: false,
        cleanupEnabled: false,
        audioRetentionPolicy: "forever",
        overlayPosition: "bottom-left",
        overlayAnimationStyle: "waveform",
        livePreviewModel: "parakeet-eou",
        liveTranscriptWidth: "wide",
        liveTranscriptLines: "four",
        showRecordingTimer: false,
        showLiveTranscription: false,
        colorTheme: "dark",
      }),
    });

    const draft = buildDraftFromSnapshot(snapshot);

    expect(draft.holdShortcut).toBe("Ctrl+H");
    expect(draft.toggleShortcut).toBe("Ctrl+T");
    expect(draft.selectedSourceId).toBe("mic-2");
    expect(draft.autoPaste).toBe(false);
    expect(draft.cleanupEnabled).toBe(false);
    expect(draft.audioRetentionPolicy).toBe("forever");
    expect(draft.overlayPosition).toBe("bottom-left");
    expect(draft.overlayAnimationStyle).toBe("waveform");
    expect(draft.livePreviewModel).toBe("parakeet-eou");
    expect(draft.liveTranscriptWidth).toBe("wide");
    expect(draft.liveTranscriptLines).toBe("four");
    expect(draft.showRecordingTimer).toBe(false);
    expect(draft.showLiveTranscription).toBe(false);
    expect(draft.colorTheme).toBe("dark");
  });

  it("falls back to first source id when selectedSourceId is null", () => {
    const snapshot = createSnapshot({
      settings: createSettings({ selectedSourceId: null as unknown as string }),
      sources: [
        createSourceInfo({ id: "fallback-mic", name: "Fallback Mic" }),
      ],
    });

    const draft = buildDraftFromSnapshot(snapshot);

    expect(draft.selectedSourceId).toBe("fallback-mic");
  });

  it("falls back to empty string when selectedSourceId is null and no sources", () => {
    const snapshot = createSnapshot({
      settings: createSettings({ selectedSourceId: null as unknown as string }),
      sources: [],
    });

    const draft = buildDraftFromSnapshot(snapshot);

    expect(draft.selectedSourceId).toBe("");
  });

  it("normalizes caret overlay position to bottom-center", () => {
    const snapshot = createSnapshot({
      settings: createSettings({ overlayPosition: "caret" }),
    });

    const draft = buildDraftFromSnapshot(snapshot);

    expect(draft.overlayPosition).toBe("bottom-center");
  });
});

describe("deriveSourceState", () => {
  it("finds the selected source by id", () => {
    const mic1 = createSourceInfo({ id: "mic-1", name: "Mic One", isDefault: false });
    const mic2 = createSourceInfo({ id: "mic-2", name: "Mic Two", isDefault: false });
    const snapshot = createSnapshot({ sources: [mic1, mic2] });

    const { activeSource, sourceOptions } = deriveSourceState(snapshot, "mic-2");

    expect(activeSource).toEqual(mic2);
    expect(sourceOptions).toHaveLength(2);
    expect(sourceOptions[0].id).toBe("mic-1");
    expect(sourceOptions[1].id).toBe("mic-2");
  });

  it("falls back to the default source when selected id is missing", () => {
    const mic1 = createSourceInfo({ id: "mic-1", name: "Mic One", isDefault: false });
    const mic2 = createSourceInfo({ id: "mic-2", name: "Mic Two", isDefault: true });
    const snapshot = createSnapshot({ sources: [mic1, mic2] });

    const { activeSource } = deriveSourceState(snapshot, "nonexistent");

    expect(activeSource).toEqual(mic2);
  });

  it("falls back to first source when no default and id is missing", () => {
    const mic1 = createSourceInfo({ id: "mic-1", name: "Mic One", isDefault: false });
    const mic2 = createSourceInfo({ id: "mic-2", name: "Mic Two", isDefault: false });
    const snapshot = createSnapshot({ sources: [mic1, mic2] });

    const { activeSource } = deriveSourceState(snapshot, "nonexistent");

    expect(activeSource).toEqual(mic1);
  });

  it("returns null active source when sources are empty", () => {
    const snapshot = createSnapshot({ sources: [] });

    const { activeSource, sourceOptions } = deriveSourceState(snapshot, "any");

    expect(activeSource).toBeNull();
    expect(sourceOptions).toEqual([]);
  });

  it("formats source option descriptions with sample rate, channels, and default label", () => {
    const mic = createSourceInfo({
      id: "mic-1",
      name: "Test Mic",
      sampleRate: 44100,
      channels: 1,
      isDefault: true,
    });
    const snapshot = createSnapshot({ sources: [mic] });

    const { sourceOptions } = deriveSourceState(snapshot, "mic-1");

    expect(sourceOptions[0].label).toBe("Test Mic");
    expect(sourceOptions[0].description).toBe("44100 Hz \u00b7 1 ch \u00b7 default");
  });
});

describe("deriveModelState", () => {
  it("returns model state with parakeet ready by default", () => {
    const snapshot = createSnapshot();
    const draft = createSettingsDraft();

    const state = deriveModelState(snapshot, draft);

    expect(state.modelRows.length).toBeGreaterThan(0);
    expect(state.activeModel).not.toBeNull();
    expect(state.activeModel?.id).toBe("parakeet");
    expect(state.readyModelOptions.length).toBeGreaterThanOrEqual(1);
  });

  it("separates batch and streaming model rows", () => {
    const snapshot = createSnapshot();
    const draft = createSettingsDraft();

    const state = deriveModelState(snapshot, draft);

    const allIds = state.modelRows.map((r) => r.id);
    const batchIds = state.batchModelRows.map((r) => r.id);
    const streamingIds = state.streamingModelRows.map((r) => r.id);

    expect(batchIds.every((id) => allIds.includes(id))).toBe(true);
    expect(streamingIds.every((id) => allIds.includes(id))).toBe(true);
    expect(streamingIds.every((id) => !batchIds.includes(id))).toBe(true);
  });

  it("resolves auto live preview to nemotron-streaming when installed", () => {
    const snapshot = createSnapshot({
      settings: createSettings({
        installedModelPaths: {
          "nemotron-streaming": "/models/nemotron",
          "parakeet-eou": "/models/eou",
        },
      }),
    });
    const draft = createSettingsDraft({ livePreviewModel: "auto" });

    const state = deriveModelState(snapshot, draft);

    expect(state.resolvedLivePreviewModel).toBe("auto");
    expect(state.effectiveLivePreviewModelId).toBe("nemotron-streaming");
  });

  it("resolves auto live preview to parakeet-eou when nemotron is not installed", () => {
    const snapshot = createSnapshot({
      settings: createSettings({
        installedModelPaths: {
          "parakeet-eou": "/models/eou",
        },
      }),
    });
    const draft = createSettingsDraft({ livePreviewModel: "auto" });

    const state = deriveModelState(snapshot, draft);

    expect(state.effectiveLivePreviewModelId).toBe("parakeet-eou");
  });

  it("resolves auto live preview to null when no streaming models installed", () => {
    const snapshot = createSnapshot();
    const draft = createSettingsDraft({ livePreviewModel: "auto" });

    const state = deriveModelState(snapshot, draft);

    expect(state.effectiveLivePreviewModelId).toBeNull();
  });

  it("falls back to auto when draft specifies uninstalled streaming model", () => {
    const snapshot = createSnapshot();
    const draft = createSettingsDraft({ livePreviewModel: "nemotron-streaming" });

    const state = deriveModelState(snapshot, draft);

    expect(state.resolvedLivePreviewModel).toBe("auto");
  });
});

describe("deriveHistoryState", () => {
  it("finds the first file transcript in history", () => {
    const micItem = createHistoryItem({
      id: "h1",
      text: "mic recording",
      capture: {
        sourceKind: "microphone",
        modelId: "parakeet",
        modelName: "Parakeet TDT",
        inferenceProvider: "cpu",
        inputSampleRate: 48000,
        inputChannels: 2,
        transcriptionSampleRate: 16000,
      },
    });
    const fileItem = createHistoryItem({
      id: "h2",
      text: "file transcript",
      capture: {
        sourceKind: "file",
        modelId: "parakeet",
        modelName: "Parakeet TDT",
        inferenceProvider: "cpu",
        inputSampleRate: 48000,
        inputChannels: 2,
        transcriptionSampleRate: 16000,
      },
    });
    const snapshot = createSnapshot({ history: [micItem, fileItem] });

    const { recentFileTranscript } = deriveHistoryState(snapshot, "");

    expect(recentFileTranscript).toEqual(fileItem);
  });

  it("returns null when no file transcript exists", () => {
    const micItem = createHistoryItem({ id: "h1" });
    const snapshot = createSnapshot({ history: [micItem] });

    const { recentFileTranscript } = deriveHistoryState(snapshot, "");

    expect(recentFileTranscript).toBeNull();
  });

  it("filters history items by search query", () => {
    const item1 = createHistoryItem({ id: "h1", text: "meeting notes about budget" });
    const item2 = createHistoryItem({ id: "h2", text: "quick reminder to call" });
    const snapshot = createSnapshot({ history: [item1, item2] });

    const { filteredHistory } = deriveHistoryState(snapshot, "budget");

    expect(filteredHistory).toHaveLength(1);
    expect(filteredHistory[0].id).toBe("h1");
  });

  it("returns all items when query is empty", () => {
    const item1 = createHistoryItem({ id: "h1", text: "first" });
    const item2 = createHistoryItem({ id: "h2", text: "second" });
    const snapshot = createSnapshot({ history: [item1, item2] });

    const { filteredHistory } = deriveHistoryState(snapshot, "");

    expect(filteredHistory).toHaveLength(2);
  });
});

describe("derivePreviewState", () => {
  it("returns Listening title when recording", () => {
    const snapshot = createSnapshot({ phase: "recording" });

    const { previewTitle } = derivePreviewState(snapshot);

    expect(previewTitle).toBe("Listening");
  });

  it("returns Transcribing title when transcribing", () => {
    const snapshot = createSnapshot({ phase: "transcribing" });

    const { previewTitle } = derivePreviewState(snapshot);

    expect(previewTitle).toBe("Transcribing");
  });

  it("returns Ready title when idle", () => {
    const snapshot = createSnapshot({ phase: "idle" });

    const { previewTitle, previewDetail } = derivePreviewState(snapshot);

    expect(previewTitle).toBe("Ready");
    expect(previewDetail).toBe("Ready");
  });

  it("uses overlay detail when present", () => {
    const snapshot = createSnapshot({
      phase: "recording",
      overlay: {
        visible: true,
        title: "",
        detail: "Processing audio...",
        levels: Array(12).fill(0),
        elapsedMs: 1000,
        limitMs: null,
      },
    });

    const { previewDetail } = derivePreviewState(snapshot);

    expect(previewDetail).toBe("Processing audio...");
  });
});

describe("findModelById", () => {
  it("returns the matching model row", () => {
    const rows = [
      createModelRow({ id: "model-a", name: "Model A" }),
      createModelRow({ id: "model-b", name: "Model B" }),
    ];

    const result = findModelById(rows, "model-b");

    expect(result).not.toBeNull();
    expect(result?.name).toBe("Model B");
  });

  it("returns null when no model matches", () => {
    const rows = [createModelRow({ id: "model-a" })];

    const result = findModelById(rows, "nonexistent");

    expect(result).toBeNull();
  });
});

describe("mapInstalledStreamingModels", () => {
  it("maps model rows to streaming model info with defaults", () => {
    const rows = [
      createModelRow({
        id: "stream-1",
        name: "Streamer One",
        unlockedFeatures: ["live-preview"],
        supportedAccelerationProviders: ["directml"],
      }),
    ];

    const result = mapInstalledStreamingModels(rows);

    expect(result).toEqual([
      {
        id: "stream-1",
        name: "Streamer One",
        unlockedFeatures: ["live-preview"],
        supportedAccelerationProviders: ["directml"],
      },
    ]);
  });

  it("defaults unlockedFeatures and supportedAccelerationProviders to empty arrays", () => {
    const rows = [
      createModelRow({
        id: "stream-2",
        name: "Streamer Two",
        unlockedFeatures: undefined,
        supportedAccelerationProviders: undefined,
      }),
    ];

    const result = mapInstalledStreamingModels(rows);

    expect(result[0].unlockedFeatures).toEqual([]);
    expect(result[0].supportedAccelerationProviders).toEqual([]);
  });
});

import { overlayPositionOptions } from "../constants";
import { formatModelSizeLabel, buildModelRows } from "./modelCatalog";
import { matchesHistory, normalizeEditableOverlayPosition } from "./utils";
import type {
  ChoiceOption,
  LivePreviewModel,
  ModelRow,
  SettingsDraft,
  Snapshot,
} from "../types";

export function supportsDynamicIsland(snapshot: Snapshot) {
  return snapshot.platform === "macos" && snapshot.dynamicIslandAvailable;
}

export function deriveOverlayPositionOptions(snapshot: Snapshot) {
  const dynamicIslandAvailable = supportsDynamicIsland(snapshot);

  return overlayPositionOptions.filter(
    (option) => option.id !== "dynamic-island" || dynamicIslandAvailable,
  );
}

export function buildDraftFromSnapshot(snapshot: Snapshot): SettingsDraft {
  const dynamicIslandAvailable = supportsDynamicIsland(snapshot);

  return {
    holdShortcut: snapshot.settings.holdShortcut,
    toggleShortcut: snapshot.settings.toggleShortcut,
    pasteLastShortcut: snapshot.settings.pasteLastShortcut,
    selectedSourceId: snapshot.settings.selectedSourceId ?? snapshot.sources[0]?.id ?? "",
    autoPaste: snapshot.settings.autoPaste,
    cleanupEnabled: snapshot.settings.cleanupEnabled,
    audioRetentionPolicy: snapshot.settings.audioRetentionPolicy,
    overlayPosition: normalizeEditableOverlayPosition(
      snapshot.settings.overlayPosition,
      dynamicIslandAvailable,
    ),
    overlayAnimationStyle: snapshot.settings.overlayAnimationStyle,
    livePreviewModel: snapshot.settings.livePreviewModel,
    liveTranscriptWidth: snapshot.settings.liveTranscriptWidth,
    liveTranscriptLines: snapshot.settings.liveTranscriptLines,
    showRecordingTimer: snapshot.settings.showRecordingTimer,
    showLiveTranscription: snapshot.settings.showLiveTranscription,
    colorTheme: snapshot.settings.colorTheme,
  };
}

export function deriveSourceState(snapshot: Snapshot, selectedSourceId: string) {
  const activeSource =
    snapshot.sources.find((source) => source.id === selectedSourceId) ??
    snapshot.sources.find((source) => source.isDefault) ??
    snapshot.sources[0] ??
    null;

  const sourceOptions: ChoiceOption[] = snapshot.sources.map((source) => ({
    id: source.id,
    label: source.name,
    description: `${source.sampleRate} Hz · ${source.channels} ch${
      source.isDefault ? " · default" : ""
    }`,
  }));

  return {
    activeSource,
    sourceOptions,
  };
}

function buildReadyModelOptions(rows: ModelRow[]): ChoiceOption[] {
  return rows
    .filter((row) => row.selectable)
    .map((row) => ({
      id: row.id,
      label: row.name,
      description: `${row.footprint} · ${formatModelSizeLabel(row)}`,
    }));
}

export function deriveModelState(snapshot: Snapshot, draft: SettingsDraft) {
  const modelRows = buildModelRows(snapshot);
  const activeModelId = snapshot.settings.selectedModelId ?? "parakeet";
  const activeModel =
    modelRows.find((row) => row.active) ??
    modelRows.find((row) => row.id === activeModelId) ??
    null;
  const batchModelRows = modelRows.filter((row) => !row.tags.includes("streaming"));
  const streamingModelRows = modelRows.filter((row) => row.tags.includes("streaming"));
  const readyModelOptions = buildReadyModelOptions(batchModelRows);
  const activeReadyModelId =
    activeModel?.selectable && !activeModel.tags.includes("streaming")
      ? activeModel.id
      : (readyModelOptions[0]?.id ?? "");
  const installedStreamingModels = streamingModelRows.filter(
    (row) => row.state === "ready" && (row.unlockedFeatures?.length ?? 0) > 0,
  );
  const livePreviewOptions: ChoiceOption[] = [
    {
      id: "auto",
      label: "Auto",
      description: "Prefer Nemotron, then Realtime EOU.",
    },
    ...installedStreamingModels.map((row) => ({
      id: row.id,
      label: row.name,
      description: `${row.footprint} · ${row.quality}`,
    })),
  ];
  const resolvedLivePreviewModel: LivePreviewModel =
    draft.livePreviewModel === "auto" ||
    installedStreamingModels.some((row) => row.id === draft.livePreviewModel)
      ? draft.livePreviewModel
      : "auto";
  const effectiveLivePreviewModelId =
    resolvedLivePreviewModel !== "auto"
      ? resolvedLivePreviewModel
      : installedStreamingModels.some((row) => row.id === "nemotron-streaming")
        ? "nemotron-streaming"
        : installedStreamingModels.some((row) => row.id === "parakeet-eou")
          ? "parakeet-eou"
          : null;

  return {
    modelRows,
    activeModel,
    batchModelRows,
    streamingModelRows,
    readyModelOptions,
    activeReadyModelId,
    installedStreamingModels,
    livePreviewOptions,
    resolvedLivePreviewModel,
    effectiveLivePreviewModelId,
  };
}

export function deriveHistoryState(snapshot: Snapshot, historyQuery: string) {
  const recentFileTranscript =
    snapshot.history.find((item) => item.capture.sourceKind === "file") ?? null;
  const filteredHistory = snapshot.history.filter((item) =>
    matchesHistory(item, historyQuery),
  );

  return {
    recentFileTranscript,
    filteredHistory,
  };
}

export function derivePreviewState(snapshot: Snapshot) {
  const previewTitle =
    snapshot.phase === "recording"
      ? "Listening"
      : snapshot.phase === "transcribing"
        ? "Transcribing"
        : "Ready";

  return {
    previewTitle,
    previewDetail: snapshot.overlay.detail || previewTitle,
  };
}

export function findModelById(rows: ModelRow[], modelId: string) {
  return rows.find((row) => row.id === modelId) ?? null;
}

export function mapInstalledStreamingModels(rows: ModelRow[]) {
  return rows.map((row) => ({
    id: row.id,
    name: row.name,
    unlockedFeatures: row.unlockedFeatures ?? [],
    supportedAccelerationProviders: row.supportedAccelerationProviders ?? [],
  }));
}

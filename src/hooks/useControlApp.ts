import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";

import {
  MEDIA_FILE_EXTENSIONS,
  SIDEBAR_COLLAPSED_KEY,
} from "../constants";
import {
  buildDraftFromSnapshot,
  deriveHistoryState,
  deriveModelState,
  derivePreviewState,
  deriveSourceState,
  findModelById,
  mapInstalledStreamingModels,
} from "../lib/controlAppModel";
import { buildSettingsUpdate, formatInvokeError } from "../lib/utils";
import {
  addCleanupTerm as addCleanupTermCommand,
  cancelCurrentOperation as cancelCurrentOperationCommand,
  clearErrorMessage,
  clearHistory as clearHistoryCommand,
  downloadCatalogModel as downloadCatalogModelCommand,
  getSnapshot,
  refreshDevices as refreshDevicesCommand,
  removeCatalogModel as removeCatalogModelCommand,
  removeCleanupTerm as removeCleanupTermCommand,
  removeHistoryItem as removeHistoryItemCommand,
  restoreDefaultCleanupTerms as restoreDefaultCleanupTermsCommand,
  startManualRecording,
  stopManualRecording,
  transcribeMediaFile,
  updateSettings,
} from "../lib/tauriApi";
import type {
  FlashMessage,
  SectionId,
  SettingsDraft,
  ShortcutFieldName,
  Snapshot,
} from "../types";
import { useButtonFeedback } from "./useButtonFeedback";

const TRANSCRIBE_FILE_FEEDBACK_MS = 900;
const COPY_FEEDBACK_MS = 1000;
const HISTORY_REMOVE_FEEDBACK_MS = 900;
const HISTORY_CLEAR_FEEDBACK_MS = 1000;
const MODEL_DOWNLOAD_FEEDBACK_MS = 1500;
const MODEL_REMOVE_FEEDBACK_MS = 1200;
const CLEANUP_REMOVE_FEEDBACK_MS = 900;

function loadSidebarCollapsedPreference() {
  try {
    return window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "1";
  } catch {
    return false;
  }
}

export function useControlApp({
  snapshot,
  setSnapshot,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
}) {
  const [activeSection, setActiveSection] = useState<SectionId>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(
    loadSidebarCollapsedPreference,
  );
  const [message, setMessage] = useState<FlashMessage>(null);
  const [capturing, setCapturing] = useState<ShortcutFieldName | null>(null);
  const [historyQuery, setHistoryQuery] = useState("");
  const [cleanupInput, setCleanupInput] = useState("");
  const [draft, setDraft] = useState<SettingsDraft>({
    holdShortcut: "",
    toggleShortcut: "",
    selectedSourceId: "",
    autoPaste: true,
    cleanupEnabled: true,
    audioRetentionPolicy: "one-day",
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    livePreviewModel: "auto",
    liveTranscriptWidth: "balanced",
    liveTranscriptLines: "one",
    showRecordingTimer: false,
    showLiveTranscription: false,
  });
  const draftRef = useRef(draft);
  const previousSnapshotRef = useRef<Snapshot | null>(null);
  const {
    buttonFeedback,
    clearButtonFeedback,
    finishButtonFeedback,
    setButtonFeedbackState,
  } = useButtonFeedback();

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
    if (!snapshot) {
      return;
    }

    setDraft(buildDraftFromSnapshot(snapshot));
  }, [snapshot]);

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    const previous = previousSnapshotRef.current;
    if (
      previous &&
      previous.phase === "transcribing" &&
      snapshot.phase === "idle" &&
      snapshot.history.length > previous.history.length &&
      snapshot.history[0]?.capture.sourceKind === "file"
    ) {
      setHistoryQuery("");
    }

    previousSnapshotRef.current = snapshot;
  }, [snapshot]);

  useEffect(() => {
    function handleCancelEscape(event: globalThis.KeyboardEvent) {
      if (event.key !== "Escape") {
        return;
      }

      if (
        !snapshot ||
        (snapshot.phase !== "recording" && snapshot.phase !== "transcribing")
      ) {
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

  async function refreshSnapshot() {
    const current = await getSnapshot();
    setSnapshot(current);
  }

  function showError(error: unknown) {
    setMessage({
      kind: "error",
      text: formatInvokeError(error),
    });
  }

  function findCurrentModelRow(modelId: string) {
    if (!snapshot) {
      return null;
    }

    return findModelById(
      deriveModelState(snapshot, draftRef.current).modelRows,
      modelId,
    );
  }

  async function sendSettingsUpdate(update: Record<string, unknown>) {
    setMessage(null);

    try {
      await updateSettings(update);
    } catch (error) {
      await refreshSnapshot();
      throw error;
    }
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
      showError(error);
    }
  }

  async function dismissSnapshotError() {
    try {
      await clearErrorMessage();
    } catch (error) {
      showError(error);
    }
  }

  async function refreshDevices() {
    setMessage(null);
    setButtonFeedbackState("refresh-inputs", "working");

    try {
      await refreshDevicesCommand();
      await refreshSnapshot();
      finishButtonFeedback("refresh-inputs");
    } catch (error) {
      clearButtonFeedback("refresh-inputs");
      showError(error);
    }
  }

  async function startRecording(mode: "hold" | "toggle") {
    try {
      setMessage(null);
      await startManualRecording(mode);
    } catch (error) {
      showError(error);
    }
  }

  async function stopRecording() {
    try {
      setMessage(null);
      await stopManualRecording();
    } catch (error) {
      showError(error);
    }
  }

  async function cancelCurrentOperation() {
    try {
      setMessage(null);
      await cancelCurrentOperationCommand();
    } catch (error) {
      showError(error);
    }
  }

  async function transcribeFile() {
    const actionId = "transcribe-file";
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      const selected = await openDialog({
        directory: false,
        multiple: false,
        filters: [
          {
            name: "Audio or video",
            extensions: [...MEDIA_FILE_EXTENSIONS],
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        clearButtonFeedback(actionId);
        return;
      }

      await transcribeMediaFile(selected);
      finishButtonFeedback(actionId, TRANSCRIBE_FILE_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function copyHistory(id: string, text: string) {
    const actionId = `copy:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await navigator.clipboard.writeText(text);
      finishButtonFeedback(actionId, COPY_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function openHistoryAudio(audioPath: string | null) {
    if (!audioPath) {
      return;
    }

    try {
      setMessage(null);
      await openPath(audioPath);
    } catch (error) {
      showError(error);
    }
  }

  async function removeHistoryItem(id: string) {
    const actionId = `history-remove:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await removeHistoryItemCommand(id);
      finishButtonFeedback(actionId, HISTORY_REMOVE_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function clearHistory() {
    const actionId = "history-clear-all";
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await clearHistoryCommand();
      setHistoryQuery("");
      finishButtonFeedback(actionId, HISTORY_CLEAR_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function activateModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = findCurrentModelRow(modelId);
    if (!row || !row.selectable || row.active) {
      return;
    }

    setMessage(null);
    try {
      await sendSettingsUpdate({
        selectedModelId: row.id,
        selectedModelKind: row.modelKind,
        selectedModelPath: row.source === "built-in" ? null : (row.path ?? null),
      });
    } catch (error) {
      showError(error);
    }
  }

  async function downloadCatalogModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = findCurrentModelRow(modelId);
    if (!row?.supportsDownload) {
      return;
    }

    setMessage(null);

    try {
      await downloadCatalogModelCommand(row.id);
      finishButtonFeedback(`model-download:${row.id}`, MODEL_DOWNLOAD_FEEDBACK_MS);
    } catch (error) {
      showError(error);
    }
  }

  async function removeCatalogModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = findCurrentModelRow(modelId);
    if (!row?.managed) {
      return;
    }

    const actionId = `model-remove:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await removeCatalogModelCommand(row.id);
      finishButtonFeedback(actionId, MODEL_REMOVE_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function openModelReference(url: string | undefined) {
    if (!url) {
      return;
    }

    try {
      await openUrl(url);
    } catch (error) {
      showError(error);
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
      await addCleanupTermCommand(trimmed);
      setCleanupInput("");
      finishButtonFeedback("cleanup-add");
    } catch (error) {
      clearButtonFeedback("cleanup-add");
      showError(error);
    }
  }

  async function removeCleanupTerm(term: string) {
    const actionId = `cleanup-remove:${term}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await removeCleanupTermCommand(term);
      finishButtonFeedback(actionId, CLEANUP_REMOVE_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  async function restoreCleanupDefaults() {
    setMessage(null);
    setButtonFeedbackState("cleanup-restore", "working");

    try {
      await restoreDefaultCleanupTermsCommand();
      finishButtonFeedback("cleanup-restore");
    } catch (error) {
      clearButtonFeedback("cleanup-restore");
      showError(error);
    }
  }

  if (!snapshot) {
    return {
      snapshot,
      ready: false as const,
    };
  }

  const sourceState = deriveSourceState(snapshot, draft.selectedSourceId);
  const modelState = deriveModelState(snapshot, draft);
  const historyState = deriveHistoryState(snapshot, historyQuery);
  const previewState = derivePreviewState(snapshot);

  return {
    snapshot,
    ready: true as const,
    activeSection,
    setActiveSection,
    sidebarCollapsed,
    setSidebarCollapsed,
    message,
    setMessage,
    capturing,
    setCapturing,
    historyQuery,
    setHistoryQuery,
    cleanupInput,
    setCleanupInput,
    buttonFeedback,
    draft,
    dismissSnapshotError,
    applySettings,
    refreshDevices,
    startRecording,
    stopRecording,
    cancelCurrentOperation,
    transcribeFile,
    copyHistory,
    openHistoryAudio,
    removeHistoryItem,
    clearHistory,
    chooseDefaultModel: activateModel,
    chooseLivePreviewModel: (value: SettingsDraft["livePreviewModel"]) =>
      applySettings({ livePreviewModel: value }),
    activateModel: (modelId: string) => activateModel(modelId),
    downloadCatalogModel: (modelId: string) => downloadCatalogModel(modelId),
    removeCatalogModel: (modelId: string) => removeCatalogModel(modelId),
    openModelReference,
    addCleanupTerm,
    removeCleanupTerm,
    restoreCleanupDefaults,
    activeSource: sourceState.activeSource,
    sourceOptions: sourceState.sourceOptions,
    activeModel: modelState.activeModel,
    batchModelRows: modelState.batchModelRows,
    streamingModelRows: modelState.streamingModelRows,
    readyModelOptions: modelState.readyModelOptions,
    activeReadyModelId: modelState.activeReadyModelId,
    installedStreamingModels: mapInstalledStreamingModels(
      modelState.installedStreamingModels,
    ),
    livePreviewOptions: modelState.livePreviewOptions,
    resolvedLivePreviewModel: modelState.resolvedLivePreviewModel,
    effectiveLivePreviewModelId: modelState.effectiveLivePreviewModelId,
    recentFileTranscript: historyState.recentFileTranscript,
    filteredHistory: historyState.filteredHistory,
    previewTitle: previewState.previewTitle,
    previewDetail: previewState.previewDetail,
    cleanupTerms: snapshot.settings.cleanupTerms,
  };
}

import { useEffect, useState } from "react";

import { SIDEBAR_COLLAPSED_KEY } from "../constants";
import {
  deriveHistoryState,
  deriveModelState,
  derivePreviewState,
  deriveSourceState,
  mapInstalledStreamingModels,
} from "../lib/controlAppModel";
import { refreshDevices as refreshDevicesCommand } from "../lib/tauriApi";
import { formatInvokeError } from "../lib/utils";
import type {
  FlashMessage,
  SectionId,
  SettingsDraft,
  ShortcutFieldName,
  Snapshot,
} from "../types";
import { useButtonFeedback } from "./useButtonFeedback";
import { useCleanupActions } from "./useCleanupActions";
import { useHistoryActions } from "./useHistoryActions";
import { useModelActions } from "./useModelActions";
import { useRecordingActions } from "./useRecordingActions";
import { useSettingsSync } from "./useSettingsSync";

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
  const {
    buttonFeedback,
    clearButtonFeedback,
    finishButtonFeedback,
    setButtonFeedbackState,
  } = useButtonFeedback();
  const feedbackControls = {
    clearButtonFeedback,
    finishButtonFeedback,
    setButtonFeedbackState,
  };

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

  function showError(error: unknown) {
    setMessage({
      kind: "error",
      text: formatInvokeError(error),
    });
  }

  function clearMessage() {
    setMessage(null);
  }

  const sharedContext = {
    snapshot,
    buttonFeedback,
    showError,
    clearMessage,
  };
  const actionContext = {
    ...sharedContext,
    ...feedbackControls,
  };

  const {
    draft,
    applySettings,
    dismissSnapshotError,
    refreshSnapshot,
    sendSettingsUpdate,
    draftRef,
  } = useSettingsSync({
    snapshot,
    setSnapshot,
    showError,
    clearMessage,
  });

  const {
    startRecording,
    stopRecording,
    cancelCurrentOperation,
    transcribeFile,
  } = useRecordingActions({
    ...actionContext,
  });

  const {
    historyQuery,
    setHistoryQuery,
    copyHistory,
    openHistoryAudio,
    removeHistoryItem,
    clearHistory,
  } = useHistoryActions({
    ...actionContext,
  });

  const {
    activateModel,
    downloadCatalogModel,
    removeCatalogModel,
    openModelReference,
  } = useModelActions({
    ...actionContext,
    draftRef,
    sendSettingsUpdate,
  });

  const {
    cleanupInput,
    setCleanupInput,
    addCleanupTerm,
    removeCleanupTerm,
    restoreCleanupDefaults,
  } = useCleanupActions({
    ...actionContext,
  });

  async function refreshDevices() {
    clearMessage();
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
  const shellState = {
    activeSection,
    setActiveSection,
    sidebarCollapsed,
    setSidebarCollapsed,
    message,
    setMessage,
    capturing,
    setCapturing,
    buttonFeedback,
  };
  const queryState = {
    historyQuery,
    setHistoryQuery,
    cleanupInput,
    setCleanupInput,
  };
  const actionState = {
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
    activateModel,
    downloadCatalogModel,
    removeCatalogModel,
    openModelReference,
    addCleanupTerm,
    removeCleanupTerm,
    restoreCleanupDefaults,
  };
  const derivedState = {
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

  return {
    snapshot,
    ready: true as const,
    draft,
    ...shellState,
    ...queryState,
    ...actionState,
    ...derivedState,
  };
}

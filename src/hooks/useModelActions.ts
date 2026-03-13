import { openUrl } from "@tauri-apps/plugin-opener";

import { deriveModelState, findModelById } from "../lib/controlAppModel";
import {
  downloadCatalogModel as downloadCatalogModelCommand,
  removeCatalogModel as removeCatalogModelCommand,
} from "../lib/tauriApi";
import type { MacosModelRuntimePreference } from "../types";
import type {
  DraftRef,
  SettingsUpdateSender,
  SharedActionContext,
  SharedFeedbackControls,
} from "./useControlAppShared";

const MODEL_DOWNLOAD_FEEDBACK_MS = 1500;
const MODEL_REMOVE_FEEDBACK_MS = 1200;

type UseModelActionsArgs = SharedActionContext &
  SharedFeedbackControls & {
    draftRef: DraftRef;
    sendSettingsUpdate: SettingsUpdateSender;
  };

export function useModelActions({
  snapshot,
  showError,
  clearMessage,
  clearButtonFeedback,
  finishButtonFeedback,
  setButtonFeedbackState,
  draftRef,
  sendSettingsUpdate,
}: UseModelActionsArgs) {
  function findCurrentModelRow(modelId: string) {
    if (!snapshot) {
      return null;
    }

    return findModelById(
      deriveModelState(snapshot, draftRef.current).modelRows,
      modelId,
    );
  }

  async function activateModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = findCurrentModelRow(modelId);
    if (!row || !row.selectable || row.active) {
      return;
    }

    clearMessage();
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

    const actionId = `model-download:${row.id}`;
    clearMessage();
    setButtonFeedbackState(actionId, "working");

    try {
      await downloadCatalogModelCommand(row.id);
      finishButtonFeedback(actionId, MODEL_DOWNLOAD_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
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
    clearMessage();
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

  async function chooseMacosModelRuntime(
    modelId: string,
    runtime: MacosModelRuntimePreference,
  ) {
    if (!snapshot) {
      return;
    }

    const nextPreferences = { ...snapshot.settings.macosModelRuntimePreferences };
    if (runtime === "cpu") {
      delete nextPreferences[modelId];
    } else {
      nextPreferences[modelId] = runtime;
    }

    clearMessage();
    try {
      await sendSettingsUpdate({
        macosModelRuntimePreferences: nextPreferences,
      });
    } catch (error) {
      showError(error);
    }
  }

  return {
    activateModel,
    downloadCatalogModel,
    removeCatalogModel,
    openModelReference,
    chooseMacosModelRuntime,
  };
}

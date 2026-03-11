import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";

import { MEDIA_FILE_EXTENSIONS } from "../constants";
import {
  cancelCurrentOperation as cancelCurrentOperationCommand,
  startManualRecording,
  stopManualRecording,
  transcribeMediaFile,
} from "../lib/tauriApi";
import type { RecordingMode } from "../types";
import type {
  SharedActionContext,
  SharedFeedbackControls,
} from "./useControlAppShared";

const TRANSCRIBE_FILE_FEEDBACK_MS = 900;

type UseRecordingActionsArgs = SharedActionContext & SharedFeedbackControls;

export function useRecordingActions({
  snapshot,
  showError,
  clearMessage,
  clearButtonFeedback,
  finishButtonFeedback,
  setButtonFeedbackState,
}: UseRecordingActionsArgs) {
  async function startRecording(mode: RecordingMode) {
    try {
      clearMessage();
      await startManualRecording(mode);
    } catch (error) {
      showError(error);
    }
  }

  async function stopRecording() {
    try {
      clearMessage();
      await stopManualRecording();
    } catch (error) {
      showError(error);
    }
  }

  async function cancelCurrentOperation() {
    try {
      clearMessage();
      await cancelCurrentOperationCommand();
    } catch (error) {
      showError(error);
    }
  }

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

  async function transcribeFile() {
    const actionId = "transcribe-file";
    clearMessage();
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

  return {
    startRecording,
    stopRecording,
    cancelCurrentOperation,
    transcribeFile,
  };
}

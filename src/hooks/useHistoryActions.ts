import { openPath } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";

import {
  clearHistory as clearHistoryCommand,
  removeHistoryItem as removeHistoryItemCommand,
} from "../lib/tauriApi";
import type { Snapshot } from "../types";
import type {
  SharedActionContext,
  SharedFeedbackControls,
} from "./useControlAppShared";

const COPY_FEEDBACK_MS = 1000;
const HISTORY_REMOVE_FEEDBACK_MS = 900;
const HISTORY_CLEAR_FEEDBACK_MS = 1000;

type UseHistoryActionsArgs = SharedActionContext & SharedFeedbackControls;

export function useHistoryActions({
  snapshot,
  showError,
  clearMessage,
  clearButtonFeedback,
  finishButtonFeedback,
  setButtonFeedbackState,
}: UseHistoryActionsArgs) {
  const [historyQuery, setHistoryQuery] = useState("");
  const previousSnapshotRef = useRef<Snapshot | null>(null);

  useEffect(() => {
    const previous = previousSnapshotRef.current;
    if (
      previous &&
      snapshot &&
      previous.phase === "transcribing" &&
      snapshot.phase === "idle" &&
      snapshot.history.length > previous.history.length &&
      snapshot.history[0]?.capture.sourceKind === "file"
    ) {
      setHistoryQuery("");
    }

    previousSnapshotRef.current = snapshot;
  }, [snapshot]);

  async function copyHistory(id: string, text: string) {
    const actionId = `copy:${id}`;
    clearMessage();
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
      clearMessage();
      await openPath(audioPath);
    } catch (error) {
      showError(error);
    }
  }

  async function removeHistoryItem(id: string) {
    const actionId = `history-remove:${id}`;
    clearMessage();
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
    clearMessage();
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

  return {
    historyQuery,
    setHistoryQuery,
    copyHistory,
    openHistoryAudio,
    removeHistoryItem,
    clearHistory,
  };
}

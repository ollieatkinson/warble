import { useState } from "react";

import {
  addCleanupTerm as addCleanupTermCommand,
  removeCleanupTerm as removeCleanupTermCommand,
  restoreDefaultCleanupTerms as restoreDefaultCleanupTermsCommand,
} from "../lib/tauriApi";
import type {
  SharedActionContext,
  SharedFeedbackControls,
} from "./useControlAppShared";

const CLEANUP_REMOVE_FEEDBACK_MS = 900;

type UseCleanupActionsArgs = SharedActionContext & SharedFeedbackControls;

export function useCleanupActions({
  showError,
  clearMessage,
  clearButtonFeedback,
  finishButtonFeedback,
  setButtonFeedbackState,
}: UseCleanupActionsArgs) {
  const [cleanupInput, setCleanupInput] = useState("");

  async function addCleanupTerm(term = cleanupInput) {
    const trimmed = term.trim();
    if (!trimmed) {
      showError("Enter a filler word or phrase to remove.");
      return;
    }

    clearMessage();
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
    clearMessage();
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
    clearMessage();
    setButtonFeedbackState("cleanup-restore", "working");

    try {
      await restoreDefaultCleanupTermsCommand();
      finishButtonFeedback("cleanup-restore");
    } catch (error) {
      clearButtonFeedback("cleanup-restore");
      showError(error);
    }
  }

  return {
    cleanupInput,
    setCleanupInput,
    addCleanupTerm,
    removeCleanupTerm,
    restoreCleanupDefaults,
  };
}

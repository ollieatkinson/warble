import { useState } from "react";

import {
  addReplacementRule as addReplacementRuleCommand,
  removeReplacementRule as removeReplacementRuleCommand,
} from "../lib/tauriApi";
import { parseReplacementVariantsInput } from "../lib/utils";
import type {
  SharedActionContext,
  SharedFeedbackControls,
} from "./useControlAppShared";

const REPLACEMENT_REMOVE_FEEDBACK_MS = 900;

type UseReplacementActionsArgs = SharedActionContext & SharedFeedbackControls;

export function useReplacementActions({
  showError,
  clearMessage,
  clearButtonFeedback,
  finishButtonFeedback,
  setButtonFeedbackState,
}: UseReplacementActionsArgs) {
  const [replacementVariantsInput, setReplacementVariantsInput] = useState("");
  const [replacementValueInput, setReplacementValueInput] = useState("");

  async function addReplacementRule(
    variantsInput = replacementVariantsInput,
    replacementInput = replacementValueInput,
  ) {
    const variants = parseReplacementVariantsInput(variantsInput);
    const replacement = replacementInput.trim();

    if (variants.length === 0) {
      showError("Enter one or more spoken variants.");
      return;
    }
    if (!replacement) {
      showError("Enter the text Warble should insert.");
      return;
    }

    clearMessage();
    setButtonFeedbackState("replacement-add", "working");

    try {
      await addReplacementRuleCommand(variants, replacement);
      setReplacementVariantsInput("");
      setReplacementValueInput("");
      finishButtonFeedback("replacement-add");
    } catch (error) {
      clearButtonFeedback("replacement-add");
      showError(error);
    }
  }

  async function removeReplacementRule(id: string) {
    const actionId = `replacement-remove:${id}`;
    clearMessage();
    setButtonFeedbackState(actionId, "working");

    try {
      await removeReplacementRuleCommand(id);
      finishButtonFeedback(actionId, REPLACEMENT_REMOVE_FEEDBACK_MS);
    } catch (error) {
      clearButtonFeedback(actionId);
      showError(error);
    }
  }

  return {
    replacementVariantsInput,
    setReplacementVariantsInput,
    replacementValueInput,
    setReplacementValueInput,
    addReplacementRule,
    removeReplacementRule,
  };
}

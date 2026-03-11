import { useEffect, useRef, useState } from "react";

import type { ButtonFeedbackState } from "../types";

export function useButtonFeedback() {
  const [buttonFeedback, setButtonFeedback] = useState<
    Record<string, ButtonFeedbackState>
  >({});
  const timersRef = useRef<Record<string, number>>({});

  useEffect(() => {
    return () => {
      Object.values(timersRef.current).forEach((timer) => {
        window.clearTimeout(timer);
      });
    };
  }, []);

  function clearButtonFeedback(actionId: string) {
    const timer = timersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete timersRef.current[actionId];
    }

    setButtonFeedback((current) => {
      if (!(actionId in current)) {
        return current;
      }

      const next = { ...current };
      delete next[actionId];
      return next;
    });
  }

  function setButtonFeedbackState(actionId: string, state: ButtonFeedbackState) {
    const timer = timersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete timersRef.current[actionId];
    }

    setButtonFeedback((current) => ({
      ...current,
      [actionId]: state,
    }));
  }

  function finishButtonFeedback(actionId: string, holdMs = 1200) {
    setButtonFeedbackState(actionId, "done");
    timersRef.current[actionId] = window.setTimeout(() => {
      clearButtonFeedback(actionId);
    }, holdMs);
  }

  return {
    buttonFeedback,
    clearButtonFeedback,
    setButtonFeedbackState,
    finishButtonFeedback,
  };
}

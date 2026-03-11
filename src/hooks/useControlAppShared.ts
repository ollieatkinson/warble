import type { MutableRefObject } from "react";

import type { ButtonFeedbackState, SettingsDraft, Snapshot } from "../types";

export type ControlButtonFeedback = Record<string, ButtonFeedbackState>;

export type SharedActionContext = {
  snapshot: Snapshot | null;
  buttonFeedback: ControlButtonFeedback;
  showError: (error: unknown) => void;
  clearMessage: () => void;
};

export type SharedFeedbackControls = {
  clearButtonFeedback: (actionId: string) => void;
  finishButtonFeedback: (actionId: string, holdMs?: number) => void;
  setButtonFeedbackState: (actionId: string, state: ButtonFeedbackState) => void;
};

export type DraftRef = MutableRefObject<SettingsDraft>;
export type SettingsUpdateSender = (update: Record<string, unknown>) => Promise<void>;

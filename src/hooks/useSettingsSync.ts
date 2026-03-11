import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useEffect, useRef, useState } from "react";

import { buildDraftFromSnapshot } from "../lib/controlAppModel";
import { clearErrorMessage, getSnapshot, updateSettings } from "../lib/tauriApi";
import { buildSettingsUpdate } from "../lib/utils";
import type { SettingsDraft, Snapshot } from "../types";

const DEFAULT_DRAFT: SettingsDraft = {
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
};

type UseSettingsSyncArgs = {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
  showError: (error: unknown) => void;
  clearMessage: () => void;
};

type UseSettingsSyncResult = {
  draft: SettingsDraft;
  setDraft: Dispatch<SetStateAction<SettingsDraft>>;
  draftRef: MutableRefObject<SettingsDraft>;
  applySettings: (update: Partial<SettingsDraft>) => Promise<void>;
  dismissSnapshotError: () => Promise<void>;
  refreshSnapshot: () => Promise<void>;
  sendSettingsUpdate: (update: Record<string, unknown>) => Promise<void>;
};

export function useSettingsSync({
  snapshot,
  setSnapshot,
  showError,
  clearMessage,
}: UseSettingsSyncArgs): UseSettingsSyncResult {
  const [draft, setDraft] = useState<SettingsDraft>(DEFAULT_DRAFT);
  const draftRef = useRef(draft);

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    const nextDraft = buildDraftFromSnapshot(snapshot);
    draftRef.current = nextDraft;
    setDraft(nextDraft);
  }, [snapshot]);

  async function refreshSnapshot() {
    const current = await getSnapshot();
    setSnapshot(current);
  }

  async function sendSettingsUpdate(update: Record<string, unknown>) {
    clearMessage();

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
    draftRef.current = nextDraft;
    setDraft(nextDraft);

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

  return {
    draft,
    setDraft,
    draftRef,
    applySettings,
    dismissSnapshotError,
    refreshSnapshot,
    sendSettingsUpdate,
  };
}

import { ShortcutField } from "../components/common";
import type { SettingsDraft, ShortcutFieldName, Snapshot } from "../types";

export function KeybindingsSection({
  snapshot,
  draft,
  capturing,
  onSetCapturing,
  onApplySettings,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  capturing: ShortcutFieldName | null;
  onSetCapturing: (value: ShortcutFieldName | null) => void;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
}) {
  const autoPasteDetail =
    snapshot.autoPasteSupport === "active-app"
      ? snapshot.platform === "macos"
        ? "Transcribed will use Cmd+V after final transcription. macOS may ask for Accessibility permission."
        : snapshot.platform === "linux"
          ? "Transcribed will paste into the active app when an X11 display is available."
          : "Transcribed will paste into the active app after the final transcript is ready."
      : "This session will copy the transcript to the clipboard instead of pasting into the active app.";

  return (
    <section className="compact-grid-two">
      <article className="surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Global shortcuts</span>
          </div>
        </div>
        <div className="field-grid">
          <ShortcutField
            label="Hold"
            value={draft.holdShortcut}
            armed={capturing === "holdShortcut"}
            onArm={() => onSetCapturing("holdShortcut")}
            onCapture={(value) => {
              void onApplySettings({ holdShortcut: value });
              onSetCapturing(null);
            }}
            onCancel={() => onSetCapturing(null)}
          />
          <ShortcutField
            label="Toggle"
            value={draft.toggleShortcut}
            armed={capturing === "toggleShortcut"}
            onArm={() => onSetCapturing("toggleShortcut")}
            onCapture={(value) => {
              void onApplySettings({ toggleShortcut: value });
              onSetCapturing(null);
            }}
            onCancel={() => onSetCapturing(null)}
          />
        </div>
        <div className="mini-meta-row">
          <span>{snapshot.shortcutMessage}</span>
          <span>{snapshot.shortcutsActive ? "Ready globally" : "Unavailable globally"}</span>
        </div>
        <div className="detail-copy">
          <p>Press Esc while recording or transcribing to cancel the current dictation.</p>
        </div>
      </article>

      <article className="surface preference-note-surface">
        <div className="surface-title">
          <span className="surface-title-label">Recording</span>
        </div>
        <label className="toggle-row toggle-row-card">
          <input
            type="checkbox"
            checked={draft.autoPaste}
            onChange={(event) =>
              void onApplySettings({
                autoPaste: event.currentTarget.checked,
              })
            }
          />
          <div>
            <strong>Auto paste</strong>
            <span>{autoPasteDetail}</span>
          </div>
        </label>
      </article>
    </section>
  );
}

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
  return (
    <section>
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
          <ShortcutField
            label="Paste last"
            value={draft.pasteLastShortcut}
            armed={capturing === "pasteLastShortcut"}
            onArm={() => onSetCapturing("pasteLastShortcut")}
            onCapture={(value) => {
              void onApplySettings({ pasteLastShortcut: value });
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
          <p>
            Use Hold for push-to-talk behavior, Toggle for one-tap start and stop, and Paste
            last to resend your latest successful transcript.
          </p>
        </div>
      </article>
    </section>
  );
}

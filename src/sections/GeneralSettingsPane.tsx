import { audioRetentionOptions } from "../constants";
import { formatAudioRetentionPolicy } from "../lib/utils";
import type { SettingsDraft, Snapshot } from "../types";

export function GeneralSettingsPane({
  snapshot,
  draft,
  onApplySettings,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
}) {
  const autoPasteDetail =
    snapshot.autoPasteSupport === "active-app"
      ? snapshot.platform === "macos"
        ? "Paste the finished transcript with Cmd+V into the active app. macOS may ask for Accessibility permission."
        : snapshot.platform === "linux"
          ? "Paste into the active app when an X11 display is available."
          : "Paste the finished transcript into the active app."
      : "Active-app paste is unavailable in this session, so Warble will copy to the clipboard instead.";

  return (
    <section className="compact-grid-two">
      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Output</span>
          </div>
        </div>

        <div className="setting-list">
          <label className="toggle-row toggle-row-card setting-toggle">
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
              <strong>Auto paste finished dictation</strong>
              <span>{autoPasteDetail}</span>
            </div>
          </label>
        </div>

        <div className="mini-meta-row">
          <span>{draft.autoPaste ? "Auto paste on" : "Copy only"}</span>
          <span>Finished transcripts are always saved to History</span>
        </div>
      </article>

      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Storage</span>
          </div>
        </div>

        <div className="setting-list">
          <div className="field">
            <span>Keep original captured audio</span>
            <div className="segmented">
              {audioRetentionOptions.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={`segment ${draft.audioRetentionPolicy === option.id ? "segment-active" : ""}`}
                  onClick={() =>
                    void onApplySettings({
                      audioRetentionPolicy: option.id,
                    })
                  }
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="mini-meta-row">
          <span>{formatAudioRetentionPolicy(draft.audioRetentionPolicy)}</span>
          <span>Closing the window keeps the app in the tray with hotkeys alive</span>
        </div>
      </article>
    </section>
  );
}

import { settingsPanes } from "../constants";
import { ShellDialog } from "../components/ShellDialog";
import { GeneralSettingsPane } from "./GeneralSettingsPane";
import { InterfaceSection } from "./InterfaceSection";
import { KeybindingsSection } from "./KeybindingsSection";
import type {
  SettingsDraft,
  SettingsPaneId,
  ShortcutFieldName,
  Snapshot,
} from "../types";

export function SettingsSheet({
  snapshot,
  draft,
  activePane,
  capturing,
  onClose,
  onSetActivePane,
  onSetCapturing,
  onApplySettings,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  activePane: SettingsPaneId;
  capturing: ShortcutFieldName | null;
  onClose: () => void;
  onSetActivePane: (pane: SettingsPaneId) => void;
  onSetCapturing: (value: ShortcutFieldName | null) => void;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
}) {
  const activePaneMeta =
    settingsPanes.find((pane) => pane.id === activePane) ?? settingsPanes[0];

  return (
    <ShellDialog
      title="Settings"
      description="Preferences live in one place so the main window stays focused on capture, models, and history."
      size="wide"
      fullHeight
      onClose={onClose}
    >
      <div className="settings-shell">
        <aside className="settings-nav">
          {settingsPanes.map((pane) => (
            <button
              key={pane.id}
              type="button"
              className={`settings-nav-button ${pane.id === activePane ? "settings-nav-button-active" : ""}`}
              onClick={() => onSetActivePane(pane.id)}
            >
              <strong>{pane.label}</strong>
              <span>{pane.description}</span>
            </button>
          ))}
        </aside>

        <div className="settings-pane">
          <div className="settings-pane-intro">
            <strong>{activePaneMeta.label}</strong>
            <span>{activePaneMeta.description}</span>
          </div>

          {activePane === "general" ? (
            <GeneralSettingsPane
              snapshot={snapshot}
              draft={draft}
              onApplySettings={onApplySettings}
            />
          ) : null}

          {activePane === "shortcuts" ? (
            <KeybindingsSection
              snapshot={snapshot}
              draft={draft}
              capturing={capturing}
              onSetCapturing={onSetCapturing}
              onApplySettings={onApplySettings}
            />
          ) : null}

          {activePane === "appearance" ? (
            <InterfaceSection draft={draft} onApplySettings={onApplySettings} />
          ) : null}
        </div>
      </div>
    </ShellDialog>
  );
}

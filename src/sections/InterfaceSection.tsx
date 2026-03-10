import { overlayAnimationOptions, overlayPositionOptions } from "../constants";
import { ChoiceDropdown } from "../components/common";
import {
  AnimationOptionPreview,
  InterfacePreviewCard,
  OverlayPositionPreview,
} from "../components/TranscriptionPill";
import { formatOverlayAnimationStyle, formatOverlayPosition } from "../lib/utils";
import type { SettingsDraft } from "../types";

export function InterfaceSection({
  draft,
  onApplySettings,
  installedStreamingModels,
  directmlAvailable,
  previewDiagnostics,
}: {
  draft: SettingsDraft;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  installedStreamingModels: Array<{
    id: string;
    name: string;
    unlockedFeatures: string[];
    directmlCapable: boolean;
  }>;
  directmlAvailable: boolean;
  previewDiagnostics: {
    backend: string;
    status: string;
    detail: string;
    recentEvents: string[];
    logPath: string | null;
  };
}) {
  const unlockedFeatures = Array.from(
    new Set(installedStreamingModels.flatMap((model) => model.unlockedFeatures)),
  );
  const installedModelLabel = installedStreamingModels.map((model) => model.name).join(" · ");
  return (
    <section className="compact-grid-two">
      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Indicator</span>
          </div>
        </div>

        <div className="setting-list">
          <div className="setting-row">
            <div className="setting-copy">
              <strong>HUD position</strong>
              <span>Where the pill sits while dictating.</span>
            </div>
            <div className="setting-control">
              <ChoiceDropdown
                label="Position"
                value={draft.overlayPosition}
                options={overlayPositionOptions}
                renderPreview={(value) => (
                  <OverlayPositionPreview position={value as typeof draft.overlayPosition} />
                )}
                onChange={(value) =>
                  void onApplySettings({
                    overlayPosition: value as typeof draft.overlayPosition,
                  })
                }
              />
            </div>
          </div>

          <div className="setting-row">
            <div className="setting-copy">
              <strong>Animation</strong>
              <span>Choose a simpler live meter style.</span>
            </div>
            <div className="setting-control">
              <ChoiceDropdown
                label="Style"
                value={draft.overlayAnimationStyle}
                options={overlayAnimationOptions}
                renderPreview={(value) => (
                  <AnimationOptionPreview style={value as typeof draft.overlayAnimationStyle} />
                )}
                onChange={(value) =>
                  void onApplySettings({
                    overlayAnimationStyle: value as typeof draft.overlayAnimationStyle,
                  })
                }
              />
            </div>
          </div>

          <label className="toggle-row toggle-row-card setting-toggle">
            <input
              type="checkbox"
              checked={draft.showRecordingTimer}
              onChange={(event) =>
                void onApplySettings({
                  showRecordingTimer: event.currentTarget.checked,
                })
              }
            />
            <div>
              <strong>Show recording timer</strong>
              <span>Shows elapsed capture time and warns as batch models near their limit.</span>
            </div>
          </label>

          <label className="toggle-row toggle-row-card setting-toggle">
            <input
              type="checkbox"
              checked={draft.showLiveTranscription}
              onChange={(event) =>
                void onApplySettings({
                  showLiveTranscription: event.currentTarget.checked,
                })
              }
            />
            <div>
              <strong>Show live transcription</strong>
              <span>Expand the pill with draft text while speaking.</span>
            </div>
          </label>
        </div>

        <div className="mini-meta-row">
          <span>{formatOverlayPosition(draft.overlayPosition)}</span>
          <span>{formatOverlayAnimationStyle(draft.overlayAnimationStyle)}</span>
          <span>{draft.showRecordingTimer ? "Timer on" : "Timer off"}</span>
          <span>{draft.showLiveTranscription ? "Expanded HUD" : "Compact HUD"}</span>
        </div>

        <div className="feature-unlock-panel">
          <div className="feature-unlock-head">
            <strong>Streaming features</strong>
            <span>
              {installedStreamingModels.length > 0
                ? `${installedStreamingModels.length} add-on${installedStreamingModels.length > 1 ? "s" : ""} installed`
                : "Install Realtime EOU or Nemotron in Models"}
            </span>
          </div>
          <div className="feature-unlock-list">
            <div className="feature-unlock-row">
              <strong>Live transcript</strong>
              <span>
                {unlockedFeatures.includes("Live transcript")
                  ? `Streaming preview is unlocked. Installed: ${installedModelLabel}`
                  : "Locked until a streaming model is installed"}
              </span>
            </div>
            <div className="feature-unlock-row">
              <strong>Final dictation</strong>
              <span>
                Streaming add-ons only affect the live preview. Final pasted text still comes from the selected batch model.
              </span>
            </div>
            <div className="feature-unlock-row">
              <strong>GPU acceleration</strong>
              <span>
                {installedStreamingModels.some((model) => model.directmlCapable)
                  ? directmlAvailable
                    ? "DirectML ready on this Windows machine"
                    : "Model supports DirectML, but this PC is not reporting DirectML-ready"
                  : "No DirectML-capable streaming model installed"}
              </span>
            </div>
          </div>
        </div>

        <div className="feature-unlock-panel">
          <div className="feature-unlock-head">
            <strong>Live preview diagnostics</strong>
            <span>{previewDiagnostics.backend || "No session yet"}</span>
          </div>
          <div className="feature-unlock-list">
            <div className="feature-unlock-row">
              <strong>Status</strong>
              <span>{previewDiagnostics.status}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Detail</strong>
              <span>{previewDiagnostics.detail || "No detail yet"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Log</strong>
              <span>{previewDiagnostics.logPath ?? "Unavailable"}</span>
            </div>
            <div className="feature-unlock-row feature-unlock-row-stack">
              <strong>Recent</strong>
              <span className="feature-unlock-events">
                {previewDiagnostics.recentEvents.length > 0
                  ? previewDiagnostics.recentEvents.join("\n")
                  : "No live preview events yet"}
              </span>
            </div>
          </div>
        </div>
      </article>

      <article className="surface preview-surface interface-preview-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Preview</span>
          </div>
        </div>

        <InterfacePreviewCard
          overlayPosition={draft.overlayPosition}
          animationStyle={draft.overlayAnimationStyle}
          showRecordingTimer={draft.showRecordingTimer}
          showLiveTranscription={draft.showLiveTranscription}
        />

        <div className="interface-preview-grid">
          <div className="interface-preview-mini">
            <span>Placement</span>
            <OverlayPositionPreview position={draft.overlayPosition} large />
          </div>
          <div className="interface-preview-mini">
            <span>Meter</span>
            <AnimationOptionPreview style={draft.overlayAnimationStyle} large />
          </div>
        </div>
      </article>
    </section>
  );
}

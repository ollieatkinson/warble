import {
  colorThemeOptions,
  liveTranscriptLineOptions,
  liveTranscriptWidthOptions,
  overlayAnimationOptions,
  overlayPositionOptions,
} from "../constants";
import { ChoiceDropdown } from "../components/common";
import {
  AnimationOptionPreview,
  InterfacePreviewCard,
  OverlayPositionPreview,
} from "../components/TranscriptionPill";
import {
  formatLiveTranscriptLines,
  formatLiveTranscriptWidth,
  formatOverlayAnimationStyle,
  formatOverlayPosition,
} from "../lib/utils";
import type { SettingsDraft } from "../types";

export function InterfaceSection({
  draft,
  onApplySettings,
}: {
  draft: SettingsDraft;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
}) {
  return (
    <>
      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Appearance</span>
          </div>
        </div>

        <div className="setting-list">
          <div className="setting-row">
            <div className="setting-copy">
              <strong>Theme</strong>
              <span>Choose light, dark, or follow your system setting.</span>
            </div>
            <div className="setting-control">
              <ChoiceDropdown
                label="Theme"
                value={draft.colorTheme}
                options={colorThemeOptions}
                onChange={(value) =>
                  void onApplySettings({
                    colorTheme: value as typeof draft.colorTheme,
                  })
                }
              />
            </div>
          </div>
        </div>
      </article>

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

            {draft.showLiveTranscription ? (
              <>
                <div className="setting-row">
                  <div className="setting-copy">
                    <strong>Transcript width</strong>
                    <span>Reserve more room for the newest words in the pill.</span>
                  </div>
                  <div className="setting-control">
                    <ChoiceDropdown
                      label="Width"
                      value={draft.liveTranscriptWidth}
                      options={liveTranscriptWidthOptions}
                      onChange={(value) =>
                        void onApplySettings({
                          liveTranscriptWidth: value as typeof draft.liveTranscriptWidth,
                        })
                      }
                    />
                  </div>
                </div>

                <div className="setting-row">
                  <div className="setting-copy">
                    <strong>Transcript lines</strong>
                    <span>Choose how many lines of live text the HUD can show.</span>
                  </div>
                  <div className="setting-control">
                    <ChoiceDropdown
                      label="Lines"
                      value={draft.liveTranscriptLines}
                      options={liveTranscriptLineOptions}
                      onChange={(value) =>
                        void onApplySettings({
                          liveTranscriptLines: value as typeof draft.liveTranscriptLines,
                        })
                      }
                    />
                  </div>
                </div>
              </>
            ) : null}
          </div>

          <div className="mini-meta-row">
            <span>{formatOverlayPosition(draft.overlayPosition)}</span>
            <span>{formatOverlayAnimationStyle(draft.overlayAnimationStyle)}</span>
            <span>{draft.showRecordingTimer ? "Timer on" : "Timer off"}</span>
            <span>{draft.showLiveTranscription ? "Expanded HUD" : "Compact HUD"}</span>
            {draft.showLiveTranscription ? (
              <>
                <span>{formatLiveTranscriptWidth(draft.liveTranscriptWidth)}</span>
                <span>{formatLiveTranscriptLines(draft.liveTranscriptLines)}</span>
              </>
            ) : null}
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
            liveTranscriptWidth={draft.liveTranscriptWidth}
            liveTranscriptLines={draft.liveTranscriptLines}
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
    </>
  );
}

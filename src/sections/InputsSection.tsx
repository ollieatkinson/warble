import { ActionButton, ChoiceDropdown } from "../components/common";
import { TranscriptionPill } from "../components/TranscriptionPill";
import { CheckIcon, RefreshIcon } from "../components/icons";
import type {
  ButtonFeedbackState,
  ChoiceOption,
  LiveTranscriptLines,
  LiveTranscriptWidth,
  SettingsDraft,
  Snapshot,
  SourceInfo,
} from "../types";

export function InputsSection({
  snapshot,
  draft,
  sourceOptions,
  activeSource,
  previewTitle,
  previewDetail,
  buttonFeedback,
  elapsedMs,
  limitMs,
  liveTranscriptWidth,
  liveTranscriptLines,
  onApplySettings,
  onRefreshDevices,
  onStartRecording,
  onStopRecording,
  onCancel,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  sourceOptions: ChoiceOption[];
  activeSource: SourceInfo | null;
  previewTitle: string;
  previewDetail: string;
  buttonFeedback: Record<string, ButtonFeedbackState>;
  elapsedMs: number;
  limitMs: number | null;
  liveTranscriptWidth: LiveTranscriptWidth;
  liveTranscriptLines: LiveTranscriptLines;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  onRefreshDevices: () => void | Promise<void>;
  onStartRecording: (mode: "hold" | "toggle") => void;
  onStopRecording: () => void;
  onCancel: () => void;
}) {
  const previewPhase = snapshot.phase === "idle" ? "recording" : snapshot.phase;
  const previewElapsedMs = snapshot.phase === "idle" ? 17_000 : elapsedMs;
  const previewLimitMs = snapshot.phase === "idle" ? 300_000 : limitMs;
  const previewHeadline =
    snapshot.phase === "idle" ? "Listening" : snapshot.overlay.title || previewTitle;
  const previewCopy =
    snapshot.phase === "idle" ? "Speak to test this input" : previewDetail;

  return (
    <section className="compact-grid-two">
      <article className="surface">
        <ChoiceDropdown
          label="Source"
          value={draft.selectedSourceId}
          options={sourceOptions}
          placeholder="No source"
          onChange={(value) =>
            void onApplySettings({
              selectedSourceId: value,
            })
          }
        />

        <div className="mini-meta-row">
          <span>{activeSource?.sampleRate ?? 0} Hz</span>
          <span>{activeSource?.channels ?? 0} ch</span>
          <span>{activeSource?.isDefault ? "Default" : "Manual"}</span>
        </div>

        <div className="inline-actions">
          <ActionButton
            className="secondary"
            state={buttonFeedback["refresh-inputs"]}
            idleLabel="Refresh"
            workingLabel="Refreshing"
            doneLabel="Updated"
            idleIcon={<RefreshIcon className="small-icon" />}
            workingIcon={<RefreshIcon className="small-icon" />}
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={onRefreshDevices}
          />
          {snapshot.phase === "recording" ? (
            <button className="secondary" onClick={onStopRecording}>
              Stop
            </button>
          ) : snapshot.phase === "transcribing" ? (
            <button className="secondary" onClick={onCancel}>
              Cancel
            </button>
          ) : (
            <>
              <button className="secondary" onClick={() => onStartRecording("hold")}>
                Hold
              </button>
              <button className="secondary" onClick={() => onStartRecording("toggle")}>
                Toggle
              </button>
            </>
          )}
        </div>
      </article>

      <article className="surface preview-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Preview</span>
          </div>
        </div>
        <div className="pill-stage pill-stage-wide">
          <TranscriptionPill
            phase={previewPhase}
            title={previewHeadline}
            detail={previewCopy}
            levels={snapshot.overlay.levels}
            animationStyle={draft.overlayAnimationStyle}
            showRecordingTimer={draft.showRecordingTimer}
            showLiveTranscription={draft.showLiveTranscription}
            liveTranscriptWidth={liveTranscriptWidth}
            liveTranscriptLines={liveTranscriptLines}
            elapsedMs={previewElapsedMs}
            limitMs={previewLimitMs}
            animatedDemo={snapshot.phase === "idle"}
            onCancel={onCancel}
          />
        </div>
      </article>
    </section>
  );
}

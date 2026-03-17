import { ActionButton, ChoiceDropdown, StatTile, StatusChip } from "../components/common";
import { TranscriptionPill } from "../components/TranscriptionPill";
import {
  CheckIcon,
  DownloadIcon,
  HistoryIcon,
  InputIcon,
  KeysIcon,
  RefreshIcon,
} from "../components/icons";
import { toneForPhase } from "../lib/utils";
import type {
  ButtonFeedbackState,
  ChoiceOption,
  HistoryItem,
  LiveTranscriptLines,
  LiveTranscriptWidth,
  ModelRow,
  OverlayAnimationStyle,
  SettingsDraft,
  Snapshot,
  SourceInfo,
} from "../types";

function outputSummary(snapshot: Snapshot, draft: SettingsDraft) {
  if (!draft.autoPaste) {
    return "Copy to clipboard";
  }

  return snapshot.autoPasteSupport === "active-app"
    ? "Paste into active app"
    : "Copy to clipboard when finished";
}

export function CaptureSection({
  snapshot,
  draft,
  sourceOptions,
  activeSource,
  activeModel,
  previewTitle,
  previewDetail,
  recentFileTranscript,
  historyCount,
  cleanupTermsCount,
  fileActionState,
  refreshActionState,
  onApplySettings,
  onRefreshDevices,
  onStartRecording,
  onStopRecording,
  onCancel,
  onTranscribeFile,
  onNavigateToModels,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  sourceOptions: ChoiceOption[];
  activeSource: SourceInfo | null;
  activeModel: ModelRow | null;
  previewTitle: string;
  previewDetail: string;
  recentFileTranscript: HistoryItem | null;
  historyCount: number;
  cleanupTermsCount: number;
  fileActionState?: ButtonFeedbackState;
  refreshActionState?: ButtonFeedbackState;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  onRefreshDevices: () => void | Promise<void>;
  onStartRecording: (mode: "hold" | "toggle") => void;
  onStopRecording: () => void;
  onCancel: () => void;
  onTranscribeFile: () => void | Promise<void>;
  onNavigateToModels?: () => void;
}) {
  const previewPhase = snapshot.phase === "idle" ? "recording" : snapshot.phase;
  const previewElapsedMs = snapshot.phase === "idle" ? 17_000 : snapshot.overlay.elapsedMs;
  const previewLimitMs = snapshot.phase === "idle" ? 300_000 : snapshot.overlay.limitMs;
  const previewHeadline =
    snapshot.phase === "idle" ? "Listening" : snapshot.overlay.title || previewTitle;
  const previewCopy =
    snapshot.phase === "idle" ? "Speak to test this setup" : previewDetail;

  return (
    <>
      {snapshot.modelStatus === "missing" && onNavigateToModels ? (
        <section className="surface model-missing-banner">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">No transcription model installed</span>
            </div>
          </div>
          <p className="model-missing-copy">
            Warble needs a speech model to transcribe audio. Browse the model catalog to download one.
          </p>
          <button className="secondary" onClick={onNavigateToModels}>
            Browse models
          </button>
        </section>
      ) : null}

      <section className="tile-grid">
        <StatTile
          icon={<CheckIcon className="tile-icon-svg" />}
          label="Model"
          value={activeModel?.name ?? "No model"}
          tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
        />
        <StatTile
          icon={<InputIcon className="tile-icon-svg" />}
          label="Input"
          value={activeSource?.name ?? "No source"}
          tone="accent"
        />
        <StatTile
          icon={<HistoryIcon className="tile-icon-svg" />}
          label="History"
          value={`${historyCount} saved`}
          tone="muted"
        />
        <StatTile
          icon={<KeysIcon className="tile-icon-svg" />}
          label="Shortcuts"
          value={snapshot.shortcutsActive ? "Ready" : "Unavailable"}
          tone={snapshot.shortcutsActive ? "accent" : "warning"}
        />
      </section>

      <section className="compact-grid-two capture-main-grid">
        <article className="surface capture-control-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Capture</span>
              <StatusChip label={previewTitle} tone={toneForPhase(snapshot.phase)} />
            </div>
            <ActionButton
              className="secondary small"
              state={refreshActionState}
              idleLabel="Refresh"
              workingLabel="Refreshing"
              doneLabel="Updated"
              idleIcon={<RefreshIcon className="small-icon" />}
              workingIcon={<RefreshIcon className="small-icon" />}
              doneIcon={<CheckIcon className="small-icon" />}
              onClick={onRefreshDevices}
            />
          </div>

          <div className="field capture-source-field">
            <ChoiceDropdown
              label="Microphone"
              value={draft.selectedSourceId}
              options={sourceOptions}
              placeholder="No source"
              onChange={(value) =>
                void onApplySettings({
                  selectedSourceId: value,
                })
              }
            />
          </div>

          <div className="mini-meta-row">
            <span>{activeSource?.sampleRate ?? 0} Hz</span>
            <span>{activeSource?.channels ?? 0} ch</span>
            <span>{activeSource?.isDefault ? "Default device" : "Manual selection"}</span>
          </div>

          <div className="capture-action-strip">
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
                <ActionButton
                  className="secondary"
                  state={fileActionState}
                  idleLabel="Transcribe File"
                  workingLabel="Opening"
                  doneLabel="Queued"
                  idleIcon={<DownloadIcon className="small-icon" />}
                  doneIcon={<CheckIcon className="small-icon" />}
                  onClick={onTranscribeFile}
                />
                <button className="secondary" onClick={() => onStartRecording("hold")}>
                  Hold to Dictate
                </button>
                <button className="secondary" onClick={() => onStartRecording("toggle")}>
                  Toggle Dictation
                </button>
              </>
            )}
          </div>

          <div className="feature-unlock-list capture-setup-list">
            <div className="feature-unlock-row">
              <strong>Final transcription</strong>
              <span>{activeModel?.name ?? "Choose a model in Models"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Output</strong>
              <span>{outputSummary(snapshot, draft)}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Cleanup</strong>
              <span>
                {draft.cleanupEnabled
                  ? `${cleanupTermsCount} filler term${cleanupTermsCount === 1 ? "" : "s"} removed`
                  : "Raw transcript kept as-is"}
              </span>
            </div>
            <div className="feature-unlock-row">
              <strong>Status</strong>
              <span>{snapshot.statusMessage}</span>
            </div>
          </div>
        </article>

        <article className="surface preview-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Live Preview</span>
            </div>
          </div>

          <div className="pill-stage pill-stage-wide">
            <TranscriptionPill
              phase={previewPhase}
              title={previewHeadline}
              detail={previewCopy}
              levels={snapshot.overlay.levels}
              overlayPosition={draft.overlayPosition}
              animationStyle={draft.overlayAnimationStyle as OverlayAnimationStyle}
              showRecordingTimer={draft.showRecordingTimer}
              showLiveTranscription={draft.showLiveTranscription}
              liveTranscriptWidth={draft.liveTranscriptWidth as LiveTranscriptWidth}
              liveTranscriptLines={draft.liveTranscriptLines as LiveTranscriptLines}
              elapsedMs={previewElapsedMs}
              limitMs={previewLimitMs}
              animatedDemo={snapshot.phase === "idle"}
              onCancel={onCancel}
            />
          </div>

          <div className="mini-meta-row">
            <span>{draft.showRecordingTimer ? "Timer visible" : "Timer hidden"}</span>
            <span>{draft.showLiveTranscription ? "Live text on" : "Live text off"}</span>
            <span>{draft.colorTheme === "system" ? "Matches system" : `${draft.colorTheme} theme`}</span>
          </div>
        </article>
      </section>

      {recentFileTranscript ? (
        <section className="surface overview-transcript-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Recent File Transcript</span>
            </div>
          </div>

          <div className="mini-meta-row">
            <span>{new Date(recentFileTranscript.createdAt).toLocaleString()}</span>
            <span>{recentFileTranscript.sourceName}</span>
          </div>

          <div className="detail-copy overview-transcript-copy">
            <p>{recentFileTranscript.text}</p>
          </div>
        </section>
      ) : null}
    </>
  );
}

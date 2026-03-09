import { StatTile, StatusChip } from "../components/common";
import { TranscriptionPill } from "../components/TranscriptionPill";
import { CheckIcon, HistoryIcon, InputIcon, KeysIcon } from "../components/icons";
import { toneForPhase } from "../lib/utils";
import type {
  AppPhase,
  ModelRow,
  OverlayAnimationStyle,
  SourceInfo,
} from "../types";

export function OverviewSection({
  activeModel,
  activeSource,
  animationStyle,
  showLiveTranscription,
  phase,
  historyCount,
  overlayTitle,
  previewTitle,
  previewDetail,
  levels,
  recentTranscript,
  shortcutsActive,
  onStartRecording,
  onStopRecording,
  onCancel,
}: {
  activeModel: ModelRow | null;
  activeSource: SourceInfo | null;
  animationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
  phase: AppPhase;
  historyCount: number;
  overlayTitle: string;
  previewTitle: string;
  previewDetail: string;
  levels: number[];
  recentTranscript: boolean;
  shortcutsActive: boolean;
  onStartRecording: (mode: "hold" | "toggle") => void;
  onStopRecording: () => void;
  onCancel: () => void;
}) {
  return (
    <>
      <section className="tile-grid">
        <StatTile
          icon={<CheckIcon className="tile-icon-svg" />}
          label="Engine"
          value={activeModel ? `${activeModel.name}` : "No model"}
          tone={activeModel?.active ? "success" : "warning"}
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
          label="Keys"
          value={shortcutsActive ? "Active" : "Unavailable"}
          tone={shortcutsActive ? "accent" : "warning"}
        />
      </section>

      <section className="surface preview-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Live</span>
            <StatusChip label={previewTitle} tone={toneForPhase(phase)} />
          </div>
          <div className="inline-actions">
            {phase === "recording" ? (
              <button className="secondary" onClick={onStopRecording}>
                Stop
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
        </div>

        <div className="pill-stage">
          <TranscriptionPill
            phase={phase}
            title={overlayTitle || previewTitle}
            detail={previewDetail}
            levels={levels}
            animationStyle={animationStyle}
            showLiveTranscription={showLiveTranscription}
            onCancel={onCancel}
          />
        </div>

        <div className="mini-meta-row">
          <span>{activeSource?.sampleRate ?? 0} Hz</span>
          <span>{activeSource?.channels ?? 0} ch</span>
          <span>{recentTranscript ? "Last saved locally" : "No transcript yet"}</span>
        </div>
      </section>
    </>
  );
}

import { invoke } from "@tauri-apps/api/core";

import { DEMO_LEVELS } from "../constants";
import { formatElapsedClock, resampleLevels, smoothLevels } from "../lib/utils";
import type {
  AppPhase,
  EditableOverlayPosition,
  OverlayAnimationStyle,
  Snapshot,
} from "../types";

function normalizeIndicatorCopy(value: string): string {
  return value
    .replace(/[_▁Ġ]+/g, " ")
    .split(/\n+/)
    .map((line) => line.replace(/\s+/g, " ").trim())
    .filter(Boolean)
    .join("\n");
}

function latestIndicatorCopy(value: string): string {
  const normalized = normalizeIndicatorCopy(value);
  if (!normalized) {
    return "";
  }

  const lines = normalized
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);
  const latestLine = lines.length > 0 ? lines[lines.length - 1] : "";
  const words = latestLine.split(" ").filter(Boolean);

  if (words.length <= 10) {
    return latestLine;
  }

  return `…${words.slice(-10).join(" ")}`;
}

export function SignalBars({
  phase,
  levels,
  count = 24,
  compact = false,
  animationStyle = "spectrum",
}: {
  phase: AppPhase;
  levels?: number[];
  count?: number;
  compact?: boolean;
  animationStyle?: OverlayAnimationStyle;
}) {
  const tone =
    phase === "transcribing"
      ? "warm"
      : phase === "recording"
        ? "live"
        : "idle";
  const sourceLevels = levels && levels.length > 0 ? levels : [0];

  if (animationStyle === "spectrum") {
    const width = compact ? 60 : 94;
    const height = compact ? 18 : 26;
    const baselineY = height - (compact ? 1.5 : 2);
    const barCount = compact ? 12 : 16;
    const inset = compact ? 3.5 : 0;
    const gap = compact ? 1.8 : 2.2;
    const innerWidth = width - inset * 2;
    const barWidth = (innerWidth - gap * (barCount - 1)) / barCount;
    const minHeight = compact ? 2.2 : 3;
    const maxHeight = compact ? 14 : 21;
    const sampled = resampleLevels(sourceLevels, barCount);

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className={`signal-bars signal-bars-${tone} ${compact ? "signal-bars-compact" : ""}`}
        aria-hidden="true"
      >
        <path
          d={`M ${inset} ${baselineY} L ${width - inset} ${baselineY}`}
          className="signal-bars-base"
        />
        {sampled.map((level, index) => {
          const normalized = Math.max(0, level);
          const barHeight = minHeight + normalized * (maxHeight - minHeight);
          const x = inset + index * (barWidth + gap);
          const y = baselineY - barHeight;

          return (
            <rect
              key={`bar-${index}`}
              x={x}
              y={y}
              width={barWidth}
              height={barHeight}
              rx={barWidth / 2}
              className="signal-bars-bar"
            />
          );
        })}
      </svg>
    );
  }

  if (animationStyle === "radial") {
    const width = compact ? 30 : 40;
    const height = compact ? 30 : 40;
    const cx = width / 2;
    const cy = height / 2;
    const pointCount = compact ? 18 : 24;
    const baseRadius = compact ? 8.6 : 11.8;
    const maxExtension = compact ? 3.1 : 4.9;
    const sampled = resampleLevels(sourceLevels, pointCount);

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className={`signal-radial signal-radial-${tone} ${compact ? "signal-radial-compact" : ""}`}
        aria-hidden="true"
      >
        <circle cx={cx} cy={cy} r={baseRadius} className="signal-radial-ring" />
        {sampled.map((level, index) => {
          const angle = (index / pointCount) * Math.PI * 2 - Math.PI / 2;
          const normalized = Math.max(0, level);
          const innerRadius = baseRadius - 0.2;
          const outerRadius = baseRadius + 0.7 + normalized * maxExtension;
          const x1 = cx + Math.cos(angle) * innerRadius;
          const y1 = cy + Math.sin(angle) * innerRadius;
          const x2 = cx + Math.cos(angle) * outerRadius;
          const y2 = cy + Math.sin(angle) * outerRadius;

          return (
            <line
              key={`spoke-${index}`}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              className="signal-radial-spoke"
            />
          );
        })}
        <circle
          cx={cx}
          cy={cy}
          r={compact ? 6.2 : 7.4}
          className="signal-radial-core-halo"
        />
        <circle
          cx={cx}
          cy={cy}
          r={compact ? 4.2 : 5.2}
          className="signal-radial-core"
        />
      </svg>
    );
  }

  const smoothedLevels = smoothLevels(
    resampleLevels(sourceLevels, compact ? 24 : count),
  );
  const width = compact ? 62 : 108;
  const height = compact ? 20 : 34;
  const inset = compact ? 3 : 0;
  const centerY = height / 2;
  const amplitude = compact ? 5.3 : 8.6;
  const drawableWidth = width - inset * 2;
  const step =
    smoothedLevels.length > 1
      ? drawableWidth / (smoothedLevels.length - 1)
      : drawableWidth;
  const topPoints = smoothedLevels.map((level, index) => {
    const x = inset + index * step;
    const normalized = Math.max(0, level);
    const offset = (compact ? 1.3 : 2) + normalized * amplitude;
    return { x, y: centerY - offset };
  });
  const bottomPoints = smoothedLevels.map((level, index) => {
    const x = inset + index * step;
    const normalized = Math.max(0, level);
    const offset = (compact ? 1.3 : 2) + normalized * amplitude;
    return { x, y: centerY + offset };
  });
  const areaPath = [
    `M 0 ${centerY}`,
    ...topPoints.map((point) => `L ${point.x} ${point.y}`),
    `L ${width} ${centerY}`,
    ...bottomPoints
      .slice()
      .reverse()
      .map((point) => `L ${point.x} ${point.y}`),
    "Z",
  ].join(" ");
  const topPath = topPoints.length
    ? `M ${topPoints.map((point) => `${point.x} ${point.y}`).join(" L ")}`
    : "";
  const bottomPath = bottomPoints.length
    ? `M ${bottomPoints.map((point) => `${point.x} ${point.y}`).join(" L ")}`
    : "";

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className={`signal-wave signal-wave-${tone} ${compact ? "signal-wave-compact" : ""}`}
      aria-hidden="true"
    >
      <path d={areaPath} className="signal-wave-fill" />
      <path d={topPath} className="signal-wave-line" />
      <path d={bottomPath} className="signal-wave-line" />
      <path
        d={`M 0 ${centerY} L ${width} ${centerY}`}
        className="signal-wave-center"
      />
    </svg>
  );
}

export function TranscriptionPill({
  phase,
  title,
  detail,
  levels,
  animationStyle,
  showRecordingTimer,
  showLiveTranscription,
  elapsedMs = 0,
  limitMs = null,
  onCancel,
}: {
  phase: AppPhase;
  title: string;
  detail: string;
  levels: number[];
  animationStyle: OverlayAnimationStyle;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
  elapsedMs?: number;
  limitMs?: number | null;
  onCancel?: (() => void) | null;
}) {
  const copy = showLiveTranscription
    ? latestIndicatorCopy(detail.trim() || title)
    : normalizeIndicatorCopy(detail.trim() || title);
  const usesRadialCore = animationStyle === "radial";
  const canCancel = phase === "recording" || phase === "transcribing";
  const hasTimer = showRecordingTimer && canCancel;
  const timerText = hasTimer
    ? limitMs && limitMs > 0
      ? `${formatElapsedClock(elapsedMs)} / ${formatElapsedClock(limitMs)}`
      : formatElapsedClock(elapsedMs)
    : "";
  const timerTone =
    limitMs && limitMs > 0
      ? elapsedMs >= limitMs
        ? "danger"
        : elapsedMs >= limitMs * 0.8
          ? "warning"
          : "muted"
      : "muted";

  return (
    <div
      className={[
        "indicator-shell",
        `indicator-shell-${phase}`,
        "indicator-shell-inline",
        showLiveTranscription ? "indicator-shell-detail" : "indicator-shell-compact",
        usesRadialCore ? "indicator-shell-radial" : "",
      ].join(" ")}
    >
      {showLiveTranscription ? (
        <div className="indicator-copy indicator-copy-floating indicator-copy-live">
          <span>{copy}</span>
        </div>
      ) : null}
      <div className={["indicator-meter-row", showLiveTranscription ? "indicator-meter-row-detail" : ""].filter(Boolean).join(" ")}>
        <div className={["indicator-mark", usesRadialCore ? "indicator-mark-radial" : ""].filter(Boolean).join(" ")}>
          {usesRadialCore ? null : (
            <button
              type="button"
              className={[
                "indicator-status-button",
                "indicator-status-button-inline",
                canCancel && onCancel ? "indicator-status-button-cancelable" : "",
                `indicator-status-button-${phase}`,
              ]
                .filter(Boolean)
                .join(" ")}
              onClick={() => onCancel?.()}
              disabled={!canCancel || !onCancel}
              aria-label={canCancel ? "Cancel current dictation" : "Dictation status"}
              title={canCancel ? "Cancel current dictation" : "Dictation status"}
            >
              <span className="indicator-dot" />
            </button>
          )}
          <div
            className={[
              "indicator-signal",
              usesRadialCore ? "" : "indicator-signal-linear",
              usesRadialCore ? "indicator-signal-radial" : "",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            <SignalBars
              phase={phase}
              levels={levels}
              compact
              animationStyle={animationStyle}
            />
            {usesRadialCore ? (
              <button
                type="button"
                className={[
                  "indicator-status-button",
                  "indicator-status-button-radial",
                  canCancel && onCancel ? "indicator-status-button-cancelable" : "",
                  `indicator-status-button-${phase}`,
                ]
                  .filter(Boolean)
                  .join(" ")}
                onClick={() => onCancel?.()}
                disabled={!canCancel || !onCancel}
                aria-label={canCancel ? "Cancel current dictation" : "Dictation status"}
                title={canCancel ? "Cancel current dictation" : "Dictation status"}
              >
                <span className="indicator-dot" />
              </button>
            ) : null}
          </div>
          {hasTimer ? (
            <span
              className={[
                "indicator-timer",
                `indicator-timer-${timerTone}`,
              ].join(" ")}
            >
              {timerText}
            </span>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function IndicatorApp({ snapshot }: { snapshot: Snapshot | null }) {
  async function cancelFromOverlay() {
    try {
      await invoke("cancel_current_operation_command");
    } catch {
      // Keep the overlay interaction quiet if cancel fails.
    }
  }

  if (!snapshot || !snapshot.overlay.visible) {
    return <div className="indicator-root indicator-root-hidden" />;
  }

  return (
    <main className="indicator-root">
      <TranscriptionPill
        phase={snapshot.phase}
        title={snapshot.overlay.title}
        detail={snapshot.overlay.detail}
        levels={snapshot.overlay.levels}
        animationStyle={snapshot.settings.overlayAnimationStyle}
        showRecordingTimer={snapshot.settings.showRecordingTimer}
        showLiveTranscription={snapshot.settings.showLiveTranscription}
        elapsedMs={snapshot.overlay.elapsedMs}
        limitMs={snapshot.overlay.limitMs}
        onCancel={cancelFromOverlay}
      />
    </main>
  );
}

export function OverlayPositionPreview({
  position,
  large = false,
}: {
  position: EditableOverlayPosition;
  large?: boolean;
}) {
  return (
    <div
      className={[
        "position-preview",
        large ? "position-preview-large" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className={`position-preview-screen position-preview-screen-${position}`}>
        <span className="position-preview-pill" />
      </div>
    </div>
  );
}

export function AnimationOptionPreview({
  style,
  large = false,
}: {
  style: OverlayAnimationStyle;
  large?: boolean;
}) {
  const showsStandaloneDot = style !== "radial";

  return (
    <div
      className={[
        "animation-choice-preview",
        large ? "animation-choice-preview-large" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className={["animation-choice-pill", !showsStandaloneDot ? "animation-choice-pill-radial" : ""].filter(Boolean).join(" ")}>
        {showsStandaloneDot ? <span className="animation-choice-dot" /> : null}
        <SignalBars
          phase="recording"
          levels={DEMO_LEVELS}
          compact
          animationStyle={style}
        />
      </div>
    </div>
  );
}

export function InterfacePreviewCard({
  overlayPosition,
  animationStyle,
  showRecordingTimer,
  showLiveTranscription,
}: {
  overlayPosition: EditableOverlayPosition;
  animationStyle: OverlayAnimationStyle;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
}) {
  return (
    <div className="interface-demo-frame">
      <div className={`interface-demo-screen interface-demo-screen-${overlayPosition}`}>
        <div className="interface-demo-pill">
          <TranscriptionPill
            phase="recording"
            title="Listening"
            detail={"Capturing live transcript\nwith the latest lines visible"}
            levels={DEMO_LEVELS}
            animationStyle={animationStyle}
            showRecordingTimer={showRecordingTimer}
            showLiveTranscription={showLiveTranscription}
            elapsedMs={134_000}
            limitMs={300_000}
          />
        </div>
      </div>
    </div>
  );
}

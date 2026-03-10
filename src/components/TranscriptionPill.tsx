import { invoke } from "@tauri-apps/api/core";
import { useLayoutEffect, useRef, useState } from "react";

import { DEMO_LEVELS } from "../constants";
import { formatElapsedClock, resampleLevels, smoothLevels } from "../lib/utils";
import type {
  AppPhase,
  EditableOverlayPosition,
  LiveTranscriptLines,
  LiveTranscriptWidth,
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

function normalizeLiveIndicatorCopy(value: string): string {
  return normalizeIndicatorCopy(value).replace(/\s+/g, " ").trim();
}

function LiveTranscriptText({
  text,
  lines,
}: {
  text: string;
  lines: LiveTranscriptLines;
}) {
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLSpanElement | null>(null);
  const [offset, setOffset] = useState(0);
  const multiline = lines !== "one";

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    const content = contentRef.current;
    if (!viewport || !content) {
      return;
    }

    let frameId = 0;

    const updateOffset = () => {
      const nextOffset = multiline
        ? Math.min(0, viewport.clientHeight - content.scrollHeight)
        : Math.min(0, viewport.clientWidth - content.scrollWidth);

      setOffset((current) => (current === nextOffset ? current : nextOffset));
    };

    const schedule = () => {
      window.cancelAnimationFrame(frameId);
      frameId = window.requestAnimationFrame(updateOffset);
    };

    schedule();

    const observer =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => {
            schedule();
          })
        : null;
    observer?.observe(viewport);
    observer?.observe(content);
    window.addEventListener("resize", schedule);

    return () => {
      window.cancelAnimationFrame(frameId);
      observer?.disconnect();
      window.removeEventListener("resize", schedule);
    };
  }, [text, multiline]);

  return (
    <div
      ref={viewportRef}
      className={[
        "indicator-live-viewport",
        `indicator-live-viewport-lines-${lines}`,
      ].join(" ")}
    >
      <span
        ref={contentRef}
        className={[
          "indicator-live-content",
          multiline ? "indicator-live-content-multiline" : "indicator-live-content-single",
        ].join(" ")}
        style={{
          transform: multiline
            ? `translateY(${offset}px)`
            : `translateX(${offset}px)`,
        }}
      >
        {text}
      </span>
    </div>
  );
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
  liveTranscriptWidth = "balanced",
  liveTranscriptLines = "one",
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
  liveTranscriptWidth?: LiveTranscriptWidth;
  liveTranscriptLines?: LiveTranscriptLines;
  elapsedMs?: number;
  limitMs?: number | null;
  onCancel?: (() => void) | null;
}) {
  const copy = showLiveTranscription
    ? normalizeLiveIndicatorCopy(detail.trim() || title)
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
  const rowClassName = [
    "indicator-row",
    showLiveTranscription ? "indicator-row-detail" : "indicator-row-compact",
    usesRadialCore ? "indicator-row-radial" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      className={[
        "indicator-shell",
        `indicator-shell-${phase}`,
        "indicator-shell-inline",
        showLiveTranscription ? "indicator-shell-detail" : "indicator-shell-compact",
        showLiveTranscription
          ? `indicator-shell-detail-width-${liveTranscriptWidth}`
          : "",
        showLiveTranscription
          ? `indicator-shell-detail-lines-${liveTranscriptLines}`
          : "",
        usesRadialCore ? "indicator-shell-radial" : "",
      ].join(" ")}
    >
      {showLiveTranscription ? (
        <div className="indicator-copy indicator-copy-floating indicator-copy-live">
          <LiveTranscriptText text={copy} lines={liveTranscriptLines} />
        </div>
      ) : null}
      <div className={rowClassName}>
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
  );
}

export function IndicatorApp({ snapshot }: { snapshot: Snapshot | null }) {
  const measureRef = useRef<HTMLDivElement | null>(null);
  const lastReportedSizeRef = useRef<string>("");

  async function cancelFromOverlay() {
    try {
      await invoke("cancel_current_operation_command");
    } catch {
      // Keep the overlay interaction quiet if cancel fails.
    }
  }

  useLayoutEffect(() => {
    if (!snapshot?.overlay.visible || !measureRef.current) {
      return;
    }

    const node = measureRef.current;
    const root = node.parentElement;
    let frameId = 0;

    const reportSize = () => {
      const rect = node.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) {
        return;
      }

      const styles = root ? window.getComputedStyle(root) : null;
      const horizontalPadding =
        (styles ? Number.parseFloat(styles.paddingLeft) : 0) +
        (styles ? Number.parseFloat(styles.paddingRight) : 0);
      const verticalPadding =
        (styles ? Number.parseFloat(styles.paddingTop) : 0) +
        (styles ? Number.parseFloat(styles.paddingBottom) : 0);
      const scale = window.devicePixelRatio || 1;
      const width = Math.ceil((rect.width + horizontalPadding) * scale);
      const height = Math.ceil((rect.height + verticalPadding) * scale);
      const sizeKey = `${width}x${height}`;

      if (lastReportedSizeRef.current === sizeKey) {
        return;
      }
      lastReportedSizeRef.current = sizeKey;

      void invoke("report_indicator_layout_command", {
        width,
        height,
      }).catch(() => {
        // Keep overlay measurement failures quiet.
      });
    };

    const scheduleReport = () => {
      window.cancelAnimationFrame(frameId);
      frameId = window.requestAnimationFrame(reportSize);
    };

    scheduleReport();

    const observer =
      typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => {
            scheduleReport();
          })
        : null;
    observer?.observe(node);
    if (root) {
      observer?.observe(root);
    }
    window.addEventListener("resize", scheduleReport);

    return () => {
      window.cancelAnimationFrame(frameId);
      observer?.disconnect();
      window.removeEventListener("resize", scheduleReport);
    };
  }, [
    snapshot?.overlay.visible,
    snapshot?.phase,
    snapshot?.overlay.title,
    snapshot?.overlay.detail,
    snapshot?.overlay.elapsedMs,
    snapshot?.overlay.limitMs,
    snapshot?.settings.overlayAnimationStyle,
    snapshot?.settings.showLiveTranscription,
    snapshot?.settings.liveTranscriptWidth,
    snapshot?.settings.liveTranscriptLines,
    snapshot?.settings.showRecordingTimer,
  ]);

  if (!snapshot || !snapshot.overlay.visible) {
    return <div className="indicator-root indicator-root-hidden" />;
  }

  return (
    <main className="indicator-root">
      <div ref={measureRef} className="indicator-measure">
        <TranscriptionPill
          phase={snapshot.phase}
          title={snapshot.overlay.title}
          detail={snapshot.overlay.detail}
          levels={snapshot.overlay.levels}
          animationStyle={snapshot.settings.overlayAnimationStyle}
          showRecordingTimer={snapshot.settings.showRecordingTimer}
          showLiveTranscription={snapshot.settings.showLiveTranscription}
          liveTranscriptWidth={snapshot.settings.liveTranscriptWidth}
          liveTranscriptLines={snapshot.settings.liveTranscriptLines}
          elapsedMs={snapshot.overlay.elapsedMs}
          limitMs={snapshot.overlay.limitMs}
          onCancel={cancelFromOverlay}
        />
      </div>
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
  liveTranscriptWidth,
  liveTranscriptLines,
}: {
  overlayPosition: EditableOverlayPosition;
  animationStyle: OverlayAnimationStyle;
  showRecordingTimer: boolean;
  showLiveTranscription: boolean;
  liveTranscriptWidth: LiveTranscriptWidth;
  liveTranscriptLines: LiveTranscriptLines;
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
            liveTranscriptWidth={liveTranscriptWidth}
            liveTranscriptLines={liveTranscriptLines}
            elapsedMs={134_000}
            limitMs={300_000}
          />
        </div>
      </div>
    </div>
  );
}

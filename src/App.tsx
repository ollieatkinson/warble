import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { KeyboardEvent, ReactNode, SVGProps } from "react";

type RecordingMode = "hold" | "toggle";
type AppPhase = "idle" | "recording" | "transcribing" | "error";
type ModelStatus = "ready" | "missing";
type OverlayPosition =
  | "bottom-center"
  | "bottom-left"
  | "bottom-right"
  | "caret";
type OverlayAnimationStyle = "spectrum" | "waveform";
type ShortcutFieldName = "holdShortcut" | "toggleShortcut";
type SectionId =
  | "overview"
  | "models"
  | "keybindings"
  | "inputs"
  | "history"
  | "about";
type ModelFilter = "all" | "ready" | "multilingual" | "local";
type EditableOverlayPosition = Exclude<OverlayPosition, "caret">;

type Settings = {
  holdShortcut: string;
  toggleShortcut: string;
  selectedSourceId: string | null;
  autoPaste: boolean;
  overlayPosition: OverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
};

type SourceInfo = {
  id: string;
  name: string;
  sampleRate: number;
  channels: number;
  isDefault: boolean;
};

type HistoryItem = {
  id: string;
  text: string;
  createdAt: string;
  sourceName: string;
  mode: RecordingMode;
  durationMs: number;
  pasted: boolean;
};

type OverlaySnapshot = {
  visible: boolean;
  title: string;
  detail: string;
  levels: number[];
};

type Snapshot = {
  phase: AppPhase;
  settings: Settings;
  sources: SourceInfo[];
  history: HistoryItem[];
  modelStatus: ModelStatus;
  shortcutsActive: boolean;
  shortcutMessage: string;
  statusMessage: string;
  errorMessage: string | null;
  overlay: OverlaySnapshot;
};

type SettingsDraft = {
  holdShortcut: string;
  toggleShortcut: string;
  selectedSourceId: string;
  autoPaste: boolean;
  overlayPosition: OverlayPosition;
  overlayAnimationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
};

type FlashMessage = {
  kind: "success" | "error";
  text: string;
} | null;

type StoredModelEntry = {
  id: string;
  name: string;
  path: string;
  compatible: boolean;
  ready: boolean;
};

type ModelPathInspection = {
  name: string;
  path: string;
  compatible: boolean;
  ready: boolean;
};

type ModelRow = {
  id: string;
  name: string;
  family: string;
  languages: string;
  speed: string;
  quality: string;
  footprint: string;
  state: "ready" | "planned" | "incomplete";
  source: "built-in" | "planned" | "local";
  active: boolean;
  summary: string;
  note: string;
  path?: string;
  tags: string[];
};

type IconProps = SVGProps<SVGSVGElement>;

const SNAPSHOT_EVENT = "transcribed://snapshot";
const MODEL_LIBRARY_STORAGE_KEY = "transcribed:model-library";
const SELECTED_MODEL_STORAGE_KEY = "transcribed:selected-model";
const isIndicatorWindow = new URLSearchParams(window.location.search).has(
  "indicator",
);

const sections: Array<{
  id: SectionId;
  label: string;
}> = [
  { id: "overview", label: "Overview" },
  { id: "models", label: "Models" },
  { id: "keybindings", label: "Keys" },
  { id: "inputs", label: "Input" },
  { id: "history", label: "History" },
  { id: "about", label: "About" },
];

const modelFilters: Array<{
  id: ModelFilter;
  label: string;
}> = [
  { id: "all", label: "All" },
  { id: "ready", label: "Ready" },
  { id: "multilingual", label: "Multilingual" },
  { id: "local", label: "Local" },
];

const overlayPositionOptions: Array<{
  id: EditableOverlayPosition;
  label: string;
}> = [
  { id: "bottom-center", label: "Center" },
  { id: "bottom-left", label: "Left" },
  { id: "bottom-right", label: "Right" },
];

const overlayAnimationOptions: Array<{
  id: OverlayAnimationStyle;
  label: string;
}> = [
  { id: "spectrum", label: "Spectrum" },
  { id: "waveform", label: "Waveform" },
];

async function getSnapshot() {
  return invoke<Snapshot>("get_snapshot");
}

async function inspectModelPath(path: string) {
  return invoke<ModelPathInspection>("inspect_model_path", { path });
}

function formatDuration(durationMs: number) {
  const seconds = durationMs / 1000;
  return `${seconds.toFixed(seconds > 10 ? 0 : 1)}s`;
}

function formatPhaseLabel(phase: AppPhase) {
  switch (phase) {
    case "recording":
      return "Recording";
    case "transcribing":
      return "Transcribing";
    case "error":
      return "Attention";
    case "idle":
    default:
      return "Idle";
  }
}

function formatOverlayPosition(position: OverlayPosition) {
  switch (position) {
    case "bottom-left":
      return "Bottom left";
    case "bottom-right":
      return "Bottom right";
    case "caret":
      return "Near caret";
    case "bottom-center":
    default:
      return "Bottom center";
  }
}

function formatOverlayAnimationStyle(style: OverlayAnimationStyle) {
  switch (style) {
    case "waveform":
      return "Waveform";
    case "spectrum":
    default:
      return "Spectrum";
  }
}

function normalizeEditableOverlayPosition(
  position: OverlayPosition,
): EditableOverlayPosition {
  switch (position) {
    case "bottom-left":
    case "bottom-right":
      return position;
    case "caret":
    case "bottom-center":
    default:
      return "bottom-center";
  }
}

function formatInvokeError(error: unknown) {
  return typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "Something went wrong.";
}

function formatShortcutKey(key: string) {
  if (key === " ") {
    return "Space";
  }

  if (/^f\d{1,2}$/i.test(key)) {
    return key.toUpperCase();
  }

  const keyMap: Record<string, string> = {
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
  };

  if (keyMap[key]) {
    return keyMap[key];
  }

  if (key.length === 1) {
    return key.toUpperCase();
  }

  return key;
}

function captureShortcut(event: KeyboardEvent<HTMLButtonElement>) {
  const ignored = new Set(["Control", "Shift", "Alt", "Meta"]);
  const mainKey = formatShortcutKey(event.key);

  if (ignored.has(event.key) || !mainKey) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }

  parts.push(mainKey);
  return parts.join("+");
}

function matchesHistory(item: HistoryItem, query: string) {
  const terms = query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);

  if (terms.length === 0) {
    return true;
  }

  const haystack = [
    item.text,
    item.sourceName,
    item.mode,
    item.pasted ? "pasted" : "saved",
  ]
    .join(" ")
    .toLowerCase();

  return terms.every((term) => haystack.includes(term));
}

function hasUnsavedChanges(snapshot: Snapshot, draft: SettingsDraft) {
  return (
    snapshot.settings.holdShortcut !== draft.holdShortcut ||
    snapshot.settings.toggleShortcut !== draft.toggleShortcut ||
    (snapshot.settings.selectedSourceId ?? "") !== draft.selectedSourceId ||
    snapshot.settings.autoPaste !== draft.autoPaste ||
    snapshot.settings.overlayPosition !== draft.overlayPosition ||
    snapshot.settings.overlayAnimationStyle !== draft.overlayAnimationStyle ||
    snapshot.settings.showLiveTranscription !== draft.showLiveTranscription
  );
}

function resampleLevels(sourceLevels: number[], count: number) {
  return Array.from({ length: count }, (_, index) => {
    if (count <= 1 || sourceLevels.length === 1) {
      return sourceLevels[0] ?? 0.14;
    }

    const position = (index / (count - 1)) * (sourceLevels.length - 1);
    const baseIndex = Math.floor(position);
    const nextIndex = Math.min(sourceLevels.length - 1, baseIndex + 1);
    const mix = position - baseIndex;
    const baseLevel = sourceLevels[baseIndex] ?? 0.14;
    const nextLevel = sourceLevels[nextIndex] ?? baseLevel;
    return baseLevel + (nextLevel - baseLevel) * mix;
  });
}

function smoothLevels(levels: number[]) {
  return levels.map((level, index, values) => {
    const previous = values[index - 1] ?? level;
    const next = values[index + 1] ?? level;
    const fartherPrevious = values[index - 2] ?? previous;
    const fartherNext = values[index + 2] ?? next;
    return (
      fartherPrevious * 0.1 +
      previous * 0.2 +
      level * 0.4 +
      next * 0.2 +
      fartherNext * 0.1
    );
  });
}

function makeCustomModelId(path: string) {
  return `custom:${path.toLowerCase().replace(/\\/g, "/")}`;
}

function loadStoredModels() {
  try {
    const raw = window.localStorage.getItem(MODEL_LIBRARY_STORAGE_KEY);
    if (!raw) {
      return [];
    }

    const parsed = JSON.parse(raw) as StoredModelEntry[];
    if (!Array.isArray(parsed)) {
      return [];
    }

    return parsed.filter(
      (entry) =>
        typeof entry.id === "string" &&
        typeof entry.name === "string" &&
        typeof entry.path === "string",
    );
  } catch {
    return [];
  }
}

function loadSelectedModelId() {
  try {
    return window.localStorage.getItem(SELECTED_MODEL_STORAGE_KEY) ?? "parakeet";
  } catch {
    return "parakeet";
  }
}

function buildModelRows(
  snapshot: Snapshot,
  customModels: StoredModelEntry[],
): ModelRow[] {
  const builtIns: ModelRow[] = [
    {
      id: "parakeet",
      name: "Parakeet TDT",
      family: "Parakeet",
      languages: "English-first",
      speed: "Fast",
      quality: "High",
      footprint: "0.6B int8",
      state: snapshot.modelStatus === "ready" ? "ready" : "incomplete",
      source: "built-in",
      active: snapshot.modelStatus === "ready",
      summary: "Current Rust-native runtime.",
      note:
        snapshot.modelStatus === "ready"
          ? "Live now."
          : "Built-in runtime missing model files.",
      tags: ["ready", "local", "english"],
    },
    {
      id: "whisper-small",
      name: "Whisper Small",
      family: "Whisper",
      languages: "Broad multilingual",
      speed: "Medium",
      quality: "Balanced",
      footprint: "244M",
      state: "planned",
      source: "planned",
      active: false,
      summary: "Planned multilingual fallback.",
      note: "Better fit for mixed-language dictation.",
      tags: ["planned", "multilingual"],
    },
    {
      id: "whisper-large",
      name: "Whisper Large",
      family: "Whisper",
      languages: "Broad multilingual",
      speed: "Slow",
      quality: "Best",
      footprint: "1.5B+",
      state: "planned",
      source: "planned",
      active: false,
      summary: "Highest-quality lane for later.",
      note: "Best for accuracy, worst for latency.",
      tags: ["planned", "multilingual"],
    },
  ];

  const locals = customModels.map<ModelRow>((entry) => ({
    id: entry.id,
    name: entry.name,
    family: "Local",
    languages: "Unknown",
    speed: "Unknown",
    quality: "Unknown",
    footprint: "Folder",
    state: entry.ready ? "ready" : "incomplete",
    source: "local",
    active: false,
    summary: entry.ready
      ? "Parakeet-compatible folder detected."
      : "Path saved locally.",
    note: entry.ready
      ? "Library only for now."
      : "Required model files are incomplete.",
    path: entry.path,
    tags: ["local", entry.ready ? "ready" : "incomplete"],
  }));

  return [...builtIns, ...locals];
}

function matchesModel(row: ModelRow, query: string, filter: ModelFilter) {
  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery) {
    const haystack = [
      row.name,
      row.family,
      row.languages,
      row.speed,
      row.quality,
      row.source,
      row.note,
    ]
      .join(" ")
      .toLowerCase();
    if (!haystack.includes(normalizedQuery)) {
      return false;
    }
  }

  switch (filter) {
    case "ready":
      return row.state === "ready";
    case "multilingual":
      return row.tags.includes("multilingual");
    case "local":
      return row.source === "local";
    case "all":
    default:
      return true;
  }
}

function useSnapshotState() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      const current = await getSnapshot();
      if (mounted) {
        setSnapshot(current);
      }

      unlisten = await listen<Snapshot>(SNAPSHOT_EVENT, (event) => {
        setSnapshot(event.payload);
      });
    })();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return [snapshot, setSnapshot] as const;
}

function GlyphBase({
  children,
  className,
  ...props
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

function OverviewIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="4.5" width="15" height="15" rx="3" />
      <path d="M8 12h8" />
      <path d="M12 8v8" />
    </GlyphBase>
  );
}

function ModelsIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M6 8.5 12 5l6 3.5-6 3.5L6 8.5Z" />
      <path d="M6 12.5 12 16l6-3.5" />
      <path d="M6 16.5 12 20l6-3.5" />
    </GlyphBase>
  );
}

function KeysIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="6.5" width="15" height="11" rx="3" />
      <path d="M8 10.5h2" />
      <path d="M12 10.5h4" />
      <path d="M8 14.5h8" />
    </GlyphBase>
  );
}

function InputIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5a3 3 0 0 1 3 3v4a3 3 0 0 1-6 0v-4a3 3 0 0 1 3-3Z" />
      <path d="M7.5 10.5a4.5 4.5 0 1 0 9 0" />
      <path d="M12 15v4.5" />
    </GlyphBase>
  );
}

function HistoryIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3" />
      <path d="M4.5 5.5v4h4" />
      <path d="M12 8.5v4l2.5 1.5" />
    </GlyphBase>
  );
}

function AboutIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 10v5" />
      <path d="M12 7.5h.01" />
    </GlyphBase>
  );
}

function SparkIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 3.5 13.7 8l4.8 1.7L13.7 11.4 12 16l-1.7-4.6L5.5 9.7 10.3 8 12 3.5Z" />
    </GlyphBase>
  );
}

function CheckIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m6.5 12 3.5 3.5 7-7" />
    </GlyphBase>
  );
}

function SearchIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="11" cy="11" r="5.5" />
      <path d="m16 16 3.5 3.5" />
    </GlyphBase>
  );
}

function FolderIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 8.5h5l1.5 2h8.5v7a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2Z" />
    </GlyphBase>
  );
}

function CopyIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="8" y="8" width="10" height="10" rx="2" />
      <path d="M6.5 15.5h-1A2.5 2.5 0 0 1 3 13V5.5A2.5 2.5 0 0 1 5.5 3H13a2.5 2.5 0 0 1 2.5 2.5v1" />
    </GlyphBase>
  );
}

function ExternalIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M14 5h5v5" />
      <path d="m10 14 9-9" />
      <path d="M19 13v4a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4" />
    </GlyphBase>
  );
}

function SectionIcon({
  section,
  className,
}: {
  section: SectionId;
  className?: string;
}) {
  switch (section) {
    case "models":
      return <ModelsIcon className={className} />;
    case "keybindings":
      return <KeysIcon className={className} />;
    case "inputs":
      return <InputIcon className={className} />;
    case "history":
      return <HistoryIcon className={className} />;
    case "about":
      return <AboutIcon className={className} />;
    case "overview":
    default:
      return <OverviewIcon className={className} />;
  }
}

function toneForPhase(phase: AppPhase) {
  switch (phase) {
    case "recording":
      return "danger";
    case "transcribing":
      return "warning";
    case "error":
      return "danger";
    case "idle":
    default:
      return "success";
  }
}

function toneForModelState(row: ModelRow) {
  switch (row.state) {
    case "ready":
      return "success";
    case "planned":
      return "warning";
    case "incomplete":
    default:
      return "muted";
  }
}

function SignalBars({
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
  const sourceLevels = levels && levels.length > 0 ? levels : [0.14];
  if (animationStyle === "spectrum") {
    const width = compact ? 56 : 96;
    const height = compact ? 20 : 36;
    const barCount = compact ? 11 : 16;
    const barWidth = compact ? 2.5 : 3.4;
    const barGap = compact ? 1.8 : 2.45;
    const innerWidth = barCount * barWidth + (barCount - 1) * barGap;
    const xOffset = (width - innerWidth) / 2;
    const centerY = height / 2;
    const amplitude = compact ? 7.2 : 13.5;
    const bars = smoothLevels(resampleLevels(sourceLevels, barCount)).map(
      (level, index) => {
        const eased = Math.pow(Math.max(0.08, level), 0.88);
        const barHeight = (compact ? 3 : 4) + eased * amplitude;
        return {
          x: xOffset + index * (barWidth + barGap),
          y: centerY - barHeight / 2,
          height: barHeight,
          opacity: 0.4 + eased * 0.6,
        };
      },
    );

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className={`signal-spectrum signal-spectrum-${tone} ${compact ? "signal-spectrum-compact" : ""}`}
        aria-hidden="true"
      >
        <line
          x1="0"
          y1={centerY}
          x2={width}
          y2={centerY}
          className="signal-spectrum-center"
        />
        {bars.map((bar, index) => (
          <rect
            key={`${index}-${bar.height.toFixed(2)}`}
            x={bar.x}
            y={bar.y}
            width={barWidth}
            height={bar.height}
            rx={barWidth / 2}
            className="signal-spectrum-bar"
            style={{ opacity: bar.opacity }}
          />
        ))}
      </svg>
    );
  }

  const smoothedLevels = smoothLevels(resampleLevels(sourceLevels, count));
  const width = compact ? 60 : 114;
  const height = compact ? 20 : 36;
  const centerY = height / 2;
  const amplitude = compact ? 5.8 : 10;
  const step = smoothedLevels.length > 1 ? width / (smoothedLevels.length - 1) : width;
  const topPoints = smoothedLevels.map((level, index) => {
    const x = index * step;
    const offset = (compact ? 1.25 : 1.8) + level * amplitude;
    return { x, y: centerY - offset };
  });
  const bottomPoints = smoothedLevels.map((level, index) => {
    const x = index * step;
    const offset = (compact ? 1.25 : 1.8) + level * amplitude;
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
    ? `M ${bottomPoints
        .map((point) => `${point.x} ${point.y}`)
        .join(" L ")}`
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

function TranscriptionPill({
  phase,
  title,
  detail,
  levels,
  animationStyle,
  showLiveTranscription,
}: {
  phase: AppPhase;
  title: string;
  detail: string;
  levels: number[];
  animationStyle: OverlayAnimationStyle;
  showLiveTranscription: boolean;
}) {
  const copy = detail.trim() || title;

  return (
    <div
      className={[
        "indicator-shell",
        `indicator-shell-${phase}`,
        "indicator-shell-inline",
        showLiveTranscription ? "indicator-shell-detail" : "indicator-shell-compact",
      ].join(" ")}
    >
      <div className="indicator-mark">
        <div className="indicator-dot" />
        <SignalBars
          phase={phase}
          levels={levels}
          compact
          animationStyle={animationStyle}
        />
      </div>
      {showLiveTranscription ? (
        <div className="indicator-copy">
          <span>{copy}</span>
        </div>
      ) : null}
    </div>
  );
}

function IndicatorApp({ snapshot }: { snapshot: Snapshot | null }) {
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
        showLiveTranscription={snapshot.settings.showLiveTranscription}
      />
    </main>
  );
}

function ShortcutField({
  label,
  value,
  armed,
  onArm,
  onCapture,
  onCancel,
}: {
  label: string;
  value: string;
  armed: boolean;
  onArm: () => void;
  onCapture: (value: string) => void;
  onCancel: () => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <button
        type="button"
        className={`shortcut-button ${armed ? "shortcut-button-armed" : ""}`}
        onClick={onArm}
        onKeyDown={(event) => {
          event.preventDefault();
          const captured = captureShortcut(event);
          if (captured) {
            onCapture(captured);
          }
        }}
        onBlur={onCancel}
      >
        {armed ? "Press shortcut..." : value}
      </button>
    </label>
  );
}

function SidebarButton({
  active,
  label,
  section,
  onClick,
}: {
  active: boolean;
  label: string;
  section: SectionId;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`sidebar-button ${active ? "sidebar-button-active" : ""}`}
      onClick={onClick}
    >
      <SectionIcon section={section} className="sidebar-icon" />
      <span>{label}</span>
    </button>
  );
}

function StatusChip({
  label,
  tone = "muted",
  icon,
}: {
  label: string;
  tone?: "success" | "warning" | "danger" | "muted" | "accent";
  icon?: ReactNode;
}) {
  return (
    <span className={`status-chip status-chip-${tone}`}>
      {icon ? <span className="status-chip-icon">{icon}</span> : null}
      {label}
    </span>
  );
}

function StatTile({
  icon,
  label,
  value,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  tone: "success" | "warning" | "danger" | "muted" | "accent";
}) {
  return (
    <article className={`tile tile-${tone}`}>
      <div className="tile-icon">{icon}</div>
      <div className="tile-copy">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </article>
  );
}

function ControlApp({
  snapshot,
  setSnapshot,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
}) {
  const [activeSection, setActiveSection] = useState<SectionId>("overview");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<FlashMessage>(null);
  const [capturing, setCapturing] = useState<ShortcutFieldName | null>(null);
  const [historyQuery, setHistoryQuery] = useState("");
  const [modelQuery, setModelQuery] = useState("");
  const [modelFilter, setModelFilter] = useState<ModelFilter>("all");
  const [customModelPath, setCustomModelPath] = useState("");
  const [customModels, setCustomModels] = useState<StoredModelEntry[]>(() =>
    loadStoredModels(),
  );
  const [selectedModelId, setSelectedModelId] = useState(() =>
    loadSelectedModelId(),
  );
  const [draft, setDraft] = useState<SettingsDraft>({
    holdShortcut: "",
    toggleShortcut: "",
    selectedSourceId: "",
    autoPaste: true,
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    showLiveTranscription: false,
  });

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    setDraft({
      holdShortcut: snapshot.settings.holdShortcut,
      toggleShortcut: snapshot.settings.toggleShortcut,
      selectedSourceId:
        snapshot.settings.selectedSourceId ?? snapshot.sources[0]?.id ?? "",
      autoPaste: snapshot.settings.autoPaste,
      overlayPosition: normalizeEditableOverlayPosition(
        snapshot.settings.overlayPosition,
      ),
      overlayAnimationStyle: snapshot.settings.overlayAnimationStyle,
      showLiveTranscription: snapshot.settings.showLiveTranscription,
    });
  }, [snapshot]);

  useEffect(() => {
    window.localStorage.setItem(
      MODEL_LIBRARY_STORAGE_KEY,
      JSON.stringify(customModels),
    );
  }, [customModels]);

  useEffect(() => {
    window.localStorage.setItem(SELECTED_MODEL_STORAGE_KEY, selectedModelId);
  }, [selectedModelId]);

  async function refreshSnapshot() {
    const current = await getSnapshot();
    setSnapshot(current);
  }

  async function saveSettings() {
    if (!snapshot) {
      return;
    }

    setSaving(true);
    setMessage(null);

    try {
      await invoke("update_settings_command", {
        update: {
          holdShortcut: draft.holdShortcut,
          toggleShortcut: draft.toggleShortcut,
          selectedSourceId: draft.selectedSourceId || undefined,
          autoPaste: draft.autoPaste,
          overlayPosition: draft.overlayPosition,
          overlayAnimationStyle: draft.overlayAnimationStyle,
          showLiveTranscription: draft.showLiveTranscription,
        },
      });
      await refreshSnapshot();
      setMessage({
        kind: "success",
        text: "Saved.",
      });
    } catch (error) {
      await refreshSnapshot();
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    } finally {
      setSaving(false);
    }
  }

  async function refreshDevices() {
    setMessage(null);

    try {
      await invoke("refresh_devices");
      await refreshSnapshot();
      setMessage({
        kind: "success",
        text: "Inputs refreshed.",
      });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function startRecording(mode: RecordingMode) {
    try {
      setMessage(null);
      await invoke("start_manual_recording", { mode });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function stopRecording() {
    try {
      setMessage(null);
      await invoke("stop_manual_recording");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function copyHistory(text: string) {
    await navigator.clipboard.writeText(text);
    setMessage({
      kind: "success",
      text: "Copied.",
    });
  }

  async function addCustomModel() {
    const trimmed = customModelPath.trim();
    if (!trimmed) {
      setMessage({
        kind: "error",
        text: "Enter a local model folder path.",
      });
      return;
    }

    try {
      const inspected = await inspectModelPath(trimmed);
      const entry: StoredModelEntry = {
        id: makeCustomModelId(inspected.path),
        name: inspected.name,
        path: inspected.path,
        compatible: inspected.compatible,
        ready: inspected.ready,
      };

      setCustomModels((current) => [
        entry,
        ...current.filter((model) => model.id !== entry.id),
      ]);
      setSelectedModelId(entry.id);
      setCustomModelPath("");
      setModelFilter("local");
      setMessage({
        kind: "success",
        text: entry.ready ? "Local model added." : "Path added to library.",
      });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  function removeCustomModel(id: string) {
    setCustomModels((current) => current.filter((model) => model.id !== id));
    setSelectedModelId("parakeet");
    setMessage({
      kind: "success",
      text: "Removed local model.",
    });
  }

  const modelRows = snapshot ? buildModelRows(snapshot, customModels) : [];
  const selectedRowExists = modelRows.some((row) => row.id === selectedModelId);
  const resolvedSelectedModelId = selectedRowExists
    ? selectedModelId
    : modelRows[0]?.id ?? "parakeet";
  const selectedModel =
    modelRows.find((row) => row.id === resolvedSelectedModelId) ?? modelRows[0];

  useEffect(() => {
    if (!selectedRowExists && modelRows[0]) {
      setSelectedModelId(modelRows[0].id);
    }
  }, [modelRows, selectedRowExists]);

  if (!snapshot) {
    return <main className="loading-shell">Loading...</main>;
  }

  const activeSource =
    snapshot.sources.find((source) => source.id === draft.selectedSourceId) ??
    snapshot.sources.find((source) => source.isDefault) ??
    snapshot.sources[0] ??
    null;
  const recentTranscript = snapshot.history[0] ?? null;
  const filteredHistory = snapshot.history.filter((item) =>
    matchesHistory(item, historyQuery),
  );
  const previewTitle =
    snapshot.phase === "recording"
      ? "Listening"
      : snapshot.phase === "transcribing"
        ? "Transcribing"
        : "Ready";
  const previewDetail = snapshot.overlay.detail || previewTitle;
  const unsavedChanges = hasUnsavedChanges(snapshot, draft);
  const filteredModels = modelRows.filter((row) =>
    matchesModel(row, modelQuery, modelFilter),
  );

  return (
    <main className="workspace-shell">
      <aside className="sidebar">
        <div className="sidebar-brand">
          <div className="sidebar-brand-mark">
            <SparkIcon className="brand-icon" />
          </div>
          <div className="sidebar-brand-copy">
            <strong>Transcribed</strong>
            <span>Local dictation</span>
          </div>
        </div>

        <nav className="sidebar-nav">
          {sections.map((section) => (
            <SidebarButton
              key={section.id}
              active={activeSection === section.id}
              label={section.label}
              section={section.id}
              onClick={() => setActiveSection(section.id)}
            />
          ))}
        </nav>

        <div className="sidebar-footer">
          <StatusChip
            label={formatPhaseLabel(snapshot.phase)}
            tone={toneForPhase(snapshot.phase)}
          />
          <StatusChip label="Local" tone="accent" />
        </div>
      </aside>

      <section className="workspace-main">
        <header className="workspace-header">
          <div className="workspace-title">
            <SectionIcon section={activeSection} className="workspace-title-icon" />
            <h1>{sections.find((section) => section.id === activeSection)?.label}</h1>
          </div>

          <div className="header-actions">
            <StatusChip
              label={snapshot.modelStatus === "ready" ? "Model ready" : "Model missing"}
              tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
              icon={<CheckIcon className="chip-icon-svg" />}
            />
            <StatusChip
              label={snapshot.shortcutsActive ? "Keys active" : "Keys off"}
              tone={snapshot.shortcutsActive ? "accent" : "warning"}
            />
            <button onClick={saveSettings} disabled={saving || !unsavedChanges}>
              {saving ? "Saving..." : unsavedChanges ? "Save" : "Saved"}
            </button>
          </div>
        </header>

        {message ? (
          <div className={`notice notice-${message.kind}`}>{message.text}</div>
        ) : null}
        {!message && snapshot.errorMessage ? (
          <div className="notice notice-error">{snapshot.errorMessage}</div>
        ) : null}

        <div className="content-stack">
          {activeSection === "overview" ? (
            <>
              <section className="tile-grid">
                <StatTile
                  icon={<CheckIcon className="tile-icon-svg" />}
                  label="Engine"
                  value={snapshot.modelStatus === "ready" ? "Parakeet ready" : "Missing"}
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
                  value={`${snapshot.history.length} saved`}
                  tone="muted"
                />
                <StatTile
                  icon={<KeysIcon className="tile-icon-svg" />}
                  label="Keys"
                  value={snapshot.shortcutsActive ? "Active" : "Unavailable"}
                  tone={snapshot.shortcutsActive ? "accent" : "warning"}
                />
              </section>

              <section className="surface preview-surface">
                <div className="surface-bar">
                  <div className="surface-title">
                    <span className="surface-title-label">Live</span>
                    <StatusChip
                      label={previewTitle}
                      tone={toneForPhase(snapshot.phase)}
                    />
                  </div>
                  <div className="inline-actions">
                    {snapshot.phase === "recording" ? (
                      <button className="secondary" onClick={stopRecording}>
                        Stop
                      </button>
                    ) : (
                      <>
                        <button
                          className="secondary"
                          onClick={() => startRecording("hold")}
                        >
                          Hold
                        </button>
                        <button
                          className="secondary"
                          onClick={() => startRecording("toggle")}
                        >
                          Toggle
                        </button>
                      </>
                    )}
                  </div>
                </div>

                <div className="pill-stage">
                  <TranscriptionPill
                    phase={snapshot.phase}
                    title={snapshot.overlay.title || previewTitle}
                    detail={previewDetail}
                    levels={snapshot.overlay.levels}
                    animationStyle={draft.overlayAnimationStyle}
                    showLiveTranscription={draft.showLiveTranscription}
                  />
                </div>

                <div className="mini-meta-row">
                  <span>{activeSource?.sampleRate ?? 0} Hz</span>
                  <span>{activeSource?.channels ?? 0} ch</span>
                  <span>{recentTranscript ? "Last saved locally" : "No transcript yet"}</span>
                </div>
              </section>
            </>
          ) : null}

          {activeSection === "models" ? (
            <>
              <section className="surface">
                <div className="toolbar">
                  <label className="search-field">
                    <SearchIcon className="search-icon" />
                    <input
                      type="search"
                      value={modelQuery}
                      onChange={(event) => setModelQuery(event.currentTarget.value)}
                      placeholder="Filter models"
                    />
                  </label>

                  <div className="segmented">
                    {modelFilters.map((filter) => (
                      <button
                        key={filter.id}
                        type="button"
                        className={`segment ${modelFilter === filter.id ? "segment-active" : ""}`}
                        onClick={() => setModelFilter(filter.id)}
                      >
                        {filter.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="table-shell">
                  <table className="model-table">
                    <thead>
                      <tr>
                        <th />
                        <th>Model</th>
                        <th>Lang</th>
                        <th>Speed</th>
                        <th>Quality</th>
                        <th>Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredModels.map((row) => (
                        <tr
                          key={row.id}
                          className={[
                            "model-row",
                            row.id === resolvedSelectedModelId
                              ? "model-row-selected"
                              : "",
                            row.active ? "model-row-active" : "",
                            row.state !== "ready" ? "model-row-dim" : "",
                          ]
                            .filter(Boolean)
                            .join(" ")}
                          onClick={() => setSelectedModelId(row.id)}
                        >
                          <td>
                            <span
                              className={`model-active-dot ${row.active ? "model-active-dot-on" : ""}`}
                            />
                          </td>
                          <td>
                            <div className="model-cell-main">
                              <strong>{row.name}</strong>
                              <span>{row.family}</span>
                            </div>
                          </td>
                          <td>{row.languages}</td>
                          <td>{row.speed}</td>
                          <td>{row.quality}</td>
                          <td>
                            <StatusChip
                              label={row.state}
                              tone={toneForModelState(row)}
                            />
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>

                  {filteredModels.length === 0 ? (
                    <div className="empty-state">No models match.</div>
                  ) : null}
                </div>
              </section>

              {selectedModel ? (
                <section className="model-detail-grid">
                  <article className="surface">
                    <div className="surface-bar">
                      <div className="surface-title">
                        <span className="surface-title-label">{selectedModel.name}</span>
                        <StatusChip
                          label={
                            selectedModel.active
                              ? "Active"
                              : selectedModel.source === "planned"
                                ? "Planned"
                                : "Library"
                          }
                          tone={
                            selectedModel.active
                              ? "success"
                              : selectedModel.source === "planned"
                                ? "warning"
                                : "accent"
                          }
                        />
                      </div>
                    </div>

                    <div className="metric-grid">
                      <div className="metric">
                        <span>Speed</span>
                        <strong>{selectedModel.speed}</strong>
                      </div>
                      <div className="metric">
                        <span>Quality</span>
                        <strong>{selectedModel.quality}</strong>
                      </div>
                      <div className="metric">
                        <span>Footprint</span>
                        <strong>{selectedModel.footprint}</strong>
                      </div>
                      <div className="metric">
                        <span>Languages</span>
                        <strong>{selectedModel.languages}</strong>
                      </div>
                    </div>

                    <div className="detail-copy">
                      <p>{selectedModel.summary}</p>
                      <p>{selectedModel.note}</p>
                      {selectedModel.path ? (
                        <code className="path-chip">{selectedModel.path}</code>
                      ) : null}
                    </div>

                    {selectedModel.source === "local" ? (
                      <div className="inline-actions">
                        <button
                          className="secondary"
                          onClick={() => removeCustomModel(selectedModel.id)}
                        >
                          Remove
                        </button>
                      </div>
                    ) : null}
                  </article>

                  <article className="surface">
                    <div className="surface-bar">
                      <div className="surface-title">
                        <span className="surface-title-label">Add local model</span>
                      </div>
                    </div>
                    <div className="field">
                      <span>Folder path</span>
                      <input
                        value={customModelPath}
                        onChange={(event) =>
                          setCustomModelPath(event.currentTarget.value)
                        }
                        placeholder="C:\\models\\parakeet"
                      />
                    </div>
                    <div className="inline-actions">
                      <button onClick={addCustomModel}>Add local</button>
                    </div>
                  </article>
                </section>
              ) : null}
            </>
          ) : null}

          {activeSection === "keybindings" ? (
            <section className="compact-grid-two">
              <article className="surface">
                <div className="field-grid">
                  <ShortcutField
                    label="Hold"
                    value={draft.holdShortcut}
                    armed={capturing === "holdShortcut"}
                    onArm={() => setCapturing("holdShortcut")}
                    onCapture={(value) => {
                      setDraft((current) => ({ ...current, holdShortcut: value }));
                      setCapturing(null);
                    }}
                    onCancel={() => setCapturing(null)}
                  />
                  <ShortcutField
                    label="Toggle"
                    value={draft.toggleShortcut}
                    armed={capturing === "toggleShortcut"}
                    onArm={() => setCapturing("toggleShortcut")}
                    onCapture={(value) => {
                      setDraft((current) => ({ ...current, toggleShortcut: value }));
                      setCapturing(null);
                    }}
                    onCancel={() => setCapturing(null)}
                  />
                </div>
              </article>

              <article className="surface">
                <label className="toggle-row toggle-row-card">
                  <input
                    type="checkbox"
                    checked={draft.autoPaste}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        autoPaste: event.currentTarget.checked,
                      }))
                    }
                  />
                  <div>
                    <strong>Auto paste</strong>
                    <span>Paste after final transcript.</span>
                  </div>
                </label>

                <div className="field">
                  <span>HUD</span>
                  <div className="segmented">
                    {overlayPositionOptions.map((option) => (
                      <button
                        key={option.id}
                        type="button"
                        className={`segment ${draft.overlayPosition === option.id ? "segment-active" : ""}`}
                        onClick={() =>
                          setDraft((current) => ({
                            ...current,
                            overlayPosition: option.id,
                          }))
                        }
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="field">
                  <span>Animation</span>
                  <div className="segmented">
                    {overlayAnimationOptions.map((option) => (
                      <button
                        key={option.id}
                        type="button"
                        className={`segment ${draft.overlayAnimationStyle === option.id ? "segment-active" : ""}`}
                        onClick={() =>
                          setDraft((current) => ({
                            ...current,
                            overlayAnimationStyle: option.id,
                          }))
                        }
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </div>

                <label className="toggle-row toggle-row-card">
                  <input
                    type="checkbox"
                    checked={draft.showLiveTranscription}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        showLiveTranscription: event.currentTarget.checked,
                      }))
                    }
                  />
                  <div>
                    <strong>Show live transcription</strong>
                    <span>Show draft text in the HUD while speaking.</span>
                  </div>
                </label>

                <div className="mini-meta-row">
                  <span>{snapshot.shortcutMessage}</span>
                  <span>{formatOverlayPosition(draft.overlayPosition)}</span>
                  <span>{formatOverlayAnimationStyle(draft.overlayAnimationStyle)}</span>
                  {snapshot.settings.overlayPosition === "caret" ? (
                    <span>Caret mode is hidden here until the settings UI is stable.</span>
                  ) : null}
                </div>
              </article>
            </section>
          ) : null}

          {activeSection === "inputs" ? (
            <>
              <section className="compact-grid-two">
                <article className="surface">
                  <label className="field">
                    <span>Source</span>
                    <select
                      value={draft.selectedSourceId}
                      onChange={(event) =>
                        setDraft((current) => ({
                          ...current,
                          selectedSourceId: event.currentTarget.value,
                        }))
                      }
                    >
                      {snapshot.sources.map((source) => (
                        <option key={source.id} value={source.id}>
                          {source.name}
                          {source.isDefault ? " (default)" : ""}
                        </option>
                      ))}
                    </select>
                  </label>

                  <div className="mini-meta-row">
                    <span>{activeSource?.sampleRate ?? 0} Hz</span>
                    <span>{activeSource?.channels ?? 0} ch</span>
                    <span>{activeSource?.isDefault ? "Default" : "Manual"}</span>
                  </div>

                  <div className="inline-actions">
                    <button className="secondary" onClick={refreshDevices}>
                      Refresh
                    </button>
                    {snapshot.phase === "recording" ? (
                      <button className="secondary" onClick={stopRecording}>
                        Stop
                      </button>
                    ) : (
                      <>
                        <button
                          className="secondary"
                          onClick={() => startRecording("hold")}
                        >
                          Hold
                        </button>
                        <button
                          className="secondary"
                          onClick={() => startRecording("toggle")}
                        >
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
                      phase={snapshot.phase}
                      title={snapshot.overlay.title || previewTitle}
                      detail={previewDetail}
                      levels={snapshot.overlay.levels}
                      animationStyle={draft.overlayAnimationStyle}
                      showLiveTranscription={draft.showLiveTranscription}
                    />
                  </div>
                </article>
              </section>
            </>
          ) : null}

          {activeSection === "history" ? (
            <>
              <section className="surface">
                <label className="search-field">
                  <SearchIcon className="search-icon" />
                  <input
                    type="search"
                    value={historyQuery}
                    onChange={(event) => setHistoryQuery(event.currentTarget.value)}
                    placeholder="Search transcripts"
                  />
                </label>
              </section>

              {filteredHistory.length === 0 ? (
                <section className="surface empty-state">
                  {snapshot.history.length === 0
                    ? "No transcripts yet."
                    : "No matches."}
                </section>
              ) : (
                <section className="history-list">
                  {filteredHistory.map((item) => (
                    <article className="surface history-row" key={item.id}>
                      <div className="history-row-head">
                        <div className="history-meta">
                          <span>{new Date(item.createdAt).toLocaleString()}</span>
                          <span>{item.sourceName}</span>
                          <span>{item.mode}</span>
                          <span>{formatDuration(item.durationMs)}</span>
                        </div>
                        <button
                          className="secondary small icon-button"
                          onClick={() => copyHistory(item.text)}
                        >
                          <CopyIcon className="small-icon" />
                          Copy
                        </button>
                      </div>
                      <p>{item.text}</p>
                    </article>
                  ))}
                </section>
              )}
            </>
          ) : null}

          {activeSection === "about" ? (
            <section className="compact-grid-three">
              <article className="surface info-tile">
                <CheckIcon className="tile-icon-svg" />
                <strong>Local only</strong>
                <span>Audio, paste, and history stay on-device.</span>
              </article>
              <article className="surface info-tile">
                <FolderIcon className="tile-icon-svg" />
                <strong>Tray-first</strong>
                <span>Close hides the window and keeps hotkeys alive.</span>
              </article>
              <article className="surface info-tile">
                <ExternalIcon className="tile-icon-svg" />
                <strong>Next</strong>
                <span>Better live preview, Whisper options, vector search.</span>
              </article>
            </section>
          ) : null}
        </div>
      </section>
    </main>
  );
}

export default function App() {
  const [snapshot, setSnapshot] = useSnapshotState();

  useEffect(() => {
    document.documentElement.classList.toggle("indicator-window", isIndicatorWindow);
    document.body.classList.toggle("indicator-window", isIndicatorWindow);

    return () => {
      document.documentElement.classList.remove("indicator-window");
      document.body.classList.remove("indicator-window");
    };
  }, []);

  return isIndicatorWindow ? (
    <IndicatorApp snapshot={snapshot} />
  ) : (
    <ControlApp snapshot={snapshot} setSnapshot={setSnapshot} />
  );
}

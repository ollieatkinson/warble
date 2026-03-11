import type {
  AudioRetentionPolicy,
  ColorTheme,
  EditableOverlayPosition,
  LivePreviewModel,
  LiveTranscriptLines,
  LiveTranscriptWidth,
  ModelFilter,
  OverlayAnimationStyle,
  SectionId,
} from "./types";

export const SNAPSHOT_EVENT = "transcribed://snapshot";
export const SIDEBAR_COLLAPSED_KEY = "transcribed:sidebar-collapsed";
export const CLEAR_HISTORY_CONFIRMATION_WINDOW_MS = 2_500;
export const isIndicatorWindow = new URLSearchParams(window.location.search).has(
  "indicator",
);
export const MEDIA_FILE_EXTENSIONS = [
  "wav",
  "mp3",
  "m4a",
  "aac",
  "flac",
  "ogg",
  "oga",
  "mp4",
  "mov",
  "mkv",
  "webm",
  "avi",
  "aif",
  "aiff",
] as const;

export const sections: Array<{
  id: SectionId;
  label: string;
}> = [
  { id: "overview", label: "Overview" },
  { id: "models", label: "Models" },
  { id: "keybindings", label: "Keys" },
  { id: "interface", label: "Interface" },
  { id: "cleanup", label: "Cleanup" },
  { id: "inputs", label: "Input" },
  { id: "history", label: "History" },
  { id: "about", label: "About" },
];

export const modelFilters: Array<{
  id: ModelFilter;
  label: string;
}> = [
  { id: "all", label: "All" },
  { id: "available", label: "Available" },
  { id: "streaming", label: "Streaming" },
];

export const overlayPositionOptions: Array<{
  id: EditableOverlayPosition;
  label: string;
}> = [
  { id: "bottom-center", label: "Center" },
  { id: "bottom-left", label: "Left" },
  { id: "bottom-right", label: "Right" },
];

export const overlayAnimationOptions: Array<{
  id: OverlayAnimationStyle;
  label: string;
  description: string;
}> = [
  { id: "waveform", label: "Wave", description: "Static centered waveform" },
  { id: "spectrum", label: "Bars", description: "Fixed reactive bars" },
  { id: "radial", label: "Radial", description: "Static reactive ring" },
];

export const livePreviewModelOptions: Array<{
  id: LivePreviewModel;
  label: string;
  description: string;
}> = [
  {
    id: "auto",
    label: "Auto",
    description: "Prefer Nemotron when installed, otherwise Realtime EOU.",
  },
  {
    id: "nemotron-streaming",
    label: "Nemotron",
    description: "Higher-quality streaming preview with punctuation.",
  },
  {
    id: "parakeet-eou",
    label: "Realtime EOU",
    description: "Lower-latency streaming preview with EOU detection.",
  },
];

export const liveTranscriptWidthOptions: Array<{
  id: LiveTranscriptWidth;
  label: string;
  description: string;
}> = [
  { id: "compact", label: "Compact", description: "Smaller live text width." },
  { id: "balanced", label: "Balanced", description: "Default live text width." },
  { id: "wide", label: "Wide", description: "More room for live text." },
];

export const liveTranscriptLineOptions: Array<{
  id: LiveTranscriptLines;
  label: string;
  description: string;
}> = [
  { id: "one", label: "1 line", description: "Keep the pill single-line." },
  { id: "two", label: "2 lines", description: "Show a little more live text." },
  { id: "three", label: "3 lines", description: "Show the deepest live preview." },
];

export const colorThemeOptions: Array<{
  id: ColorTheme;
  label: string;
  description: string;
}> = [
  { id: "system", label: "System", description: "Follow your system appearance." },
  { id: "light", label: "Light", description: "Always use light theme." },
  { id: "dark", label: "Dark", description: "Always use dark theme." },
];

export const DEMO_LEVELS = [
  0.18, 0.34, 0.62, 0.28, 0.82, 0.46, 0.24, 0.58, 0.38, 0.22, 0.48, 0.26,
];

export const CLEANUP_SUGGESTIONS = ["um", "uh", "erm", "uhm", "hmm", "you know"];

export const audioRetentionOptions: Array<{
  id: AudioRetentionPolicy;
  label: string;
  description: string;
}> = [
  { id: "one-day", label: "24h", description: "Keep clip audio for one day." },
  {
    id: "seven-days",
    label: "7d",
    description: "Keep clip audio for seven days.",
  },
  {
    id: "thirty-days",
    label: "30d",
    description: "Keep clip audio for thirty days.",
  },
];

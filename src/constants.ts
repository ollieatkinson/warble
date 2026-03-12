import type {
  AudioRetentionPolicy,
  ColorTheme,
  LivePreviewModel,
  LiveTranscriptLines,
  LiveTranscriptWidth,
  ModelFilter,
  OverlayAnimationStyle,
  OverlayPosition,
  SectionId,
  SettingsPaneId,
} from "./types";

export const SNAPSHOT_EVENT = "warble://snapshot";
export const SHELL_ACTION_EVENT = "warble://shell-action";
export const SIDEBAR_COLLAPSED_KEY = "warble:sidebar-collapsed";
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
  { id: "capture", label: "Capture" },
  { id: "models", label: "Models" },
  { id: "vocabulary", label: "Vocabulary" },
  { id: "history", label: "History" },
];

export const utilitySections: Array<{
  id: SectionId;
  label: string;
}> = [
  { id: "settings", label: "Settings" },
  { id: "help", label: "Help" },
];

export const settingsPanes: Array<{
  id: SettingsPaneId;
  label: string;
  description: string;
}> = [
  {
    id: "general",
    label: "General",
    description: "Output and storage preferences.",
  },
  {
    id: "shortcuts",
    label: "Shortcuts",
    description: "Global recording hotkeys.",
  },
  {
    id: "appearance",
    label: "Appearance",
    description: "Indicator and theme options.",
  },
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
  id: OverlayPosition;
  label: string;
  description: string;
  macOnly?: boolean;
}> = [
  {
    id: "dynamic-island",
    label: "Dynamic Island",
    description: "Menu-bar-integrated HUD that blends into a MacBook camera housing.",
  },
  {
    id: "top-center",
    label: "Top center",
    description: "Pin the HUD to the top middle of the screen.",
  },
  {
    id: "top-left",
    label: "Top left",
    description: "Keep the HUD tucked into the top-left corner.",
  },
  {
    id: "top-right",
    label: "Top right",
    description: "Keep the HUD tucked into the top-right corner.",
  },
  {
    id: "bottom-center",
    label: "Bottom center",
    description: "Default centered placement near the bottom edge.",
  },
  {
    id: "bottom-left",
    label: "Bottom left",
    description: "Keep the HUD tucked into the bottom-left corner.",
  },
  {
    id: "bottom-right",
    label: "Bottom right",
    description: "Keep the HUD tucked into the bottom-right corner.",
  },
  {
    id: "caret",
    label: "Follow caret",
    description: "Position the HUD near the active text field (macOS only).",
    macOnly: true,
  },
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

import type {
  AudioRetentionPolicy,
  EditableOverlayPosition,
  ModelFilter,
  OverlayAnimationStyle,
  SectionId,
} from "./types";

export const SNAPSHOT_EVENT = "transcribed://snapshot";
export const SIDEBAR_COLLAPSED_KEY = "transcribed:sidebar-collapsed";
export const isIndicatorWindow = new URLSearchParams(window.location.search).has(
  "indicator",
);

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
  { id: "multilingual", label: "Multilingual" },
  { id: "future", label: "Future" },
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

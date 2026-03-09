import type { ModelFeatureItem, ModelFilter, ModelRow, Snapshot, SystemProfile } from "../types";
import { formatBytes, formatSystemProfile } from "./utils";

const MODEL_CATALOG: Array<
  Omit<ModelRow, "state" | "source" | "managed" | "active" | "selectable" | "path">
> = [
  {
    id: "parakeet",
    name: "Parakeet TDT",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "FastConformer + TDT",
    languages: "25 languages",
    speechMode: "Batch ASR",
    speed: "Fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Ready in app",
    license: "See model card",
    summary:
      "Multilingual offline dictation model with fast local ONNX inference and timestamp support.",
    note:
      "This is the stable in-app speech path today, and the best default for local dictation.",
    bestFor: "Default multilingual dictation",
    capabilities: ["TDT decoder", "Auto language detection", "Token timestamps"],
    featureBadges: [
      { id: "local", label: "Local", icon: "cpu" },
      { id: "tdt", label: "TDT", icon: "spark" },
      { id: "timed", label: "Timed", icon: "clock" },
    ],
    highlights: [
      "Current default engine in Transcribed",
      "Stable local Rust ONNX backend",
      "Best balance of speed and multilingual coverage today",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3",
    artifactUrl: "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8",
    artifactLabel: "Compatible ONNX bundle",
    tags: ["nvidia", "offline", "timestamps", "default"],
    supportsInstall: true,
    supportsDownload: true,
    downloadSizeBytes: 593_000_000,
    audioLimitMs: 5 * 60 * 1_000,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 4,
    recommendedCores: 8,
  },
  {
    id: "parakeet-ctc",
    name: "Parakeet CTC",
    modelKind: "parakeet-ctc",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "FastConformer + CTC",
    languages: "English",
    speechMode: "Batch ASR",
    speed: "Very fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Stable runtime pending",
    license: "See model card",
    summary:
      "English-first Parakeet variant aimed at fast offline transcription with punctuation and capitalization.",
    note:
      "Cataloged here because it is part of the Parakeet family, but not yet enabled in the stable Transcribed runtime.",
    bestFor: "English punctuation-heavy offline transcription",
    capabilities: ["CTC decoding", "Punctuation and caps", "Word timestamps"],
    featureBadges: [
      { id: "offline", label: "Offline", icon: "cpu" },
      { id: "ctc", label: "CTC", icon: "spark" },
      { id: "timed", label: "Timed", icon: "clock" },
    ],
    highlights: [
      "English-focused Parakeet family variant",
      "Good fallback when you want a simpler decoding path",
      "Held back until the stable runtime path is re-enabled",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-ctc-0.6b",
    artifactUrl:
      "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/tree/main/onnx",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "offline", "timestamps"],
    supportsInstall: false,
    supportsDownload: false,
    downloadSizeBytes: 1_240_000_000,
    audioLimitMs: 5 * 60 * 1_000,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 4,
    recommendedCores: 8,
  },
  {
    id: "parakeet-eou",
    name: "Parakeet Realtime EOU",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "Streaming encoder-decoder",
    languages: "English",
    speechMode: "Streaming ASR",
    speed: "Realtime",
    quality: "Balanced",
    footprint: "120M",
    runtime: "Streaming runtime pending",
    license: "See model card",
    summary:
      "Lightweight streaming ASR model with end-of-utterance detection for low-latency voice UX.",
    note:
      "The best candidate for a future live preview path once we wire a proper streaming session into the app.",
    bestFor: "Low-latency live dictation",
    capabilities: ["EOU detection", "160 ms chunking", "Stateful streaming"],
    featureBadges: [
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "eou", label: "EOU", icon: "clock" },
      { id: "light", label: "120M", icon: "spark" },
    ],
    highlights: [
      "Lowest-footprint speech model in the current NVIDIA set",
      "Built for real-time incremental updates",
      "Good long-term preview model candidate",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet_realtime_eou_120m-v1",
    artifactUrl:
      "https://huggingface.co/altunenes/parakeet-rs/tree/main/realtime_eou_120m-v1-onnx",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "streaming", "future"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 6,
    recommendedCores: 8,
  },
  {
    id: "nemotron-streaming",
    name: "Nemotron Streaming",
    modelKind: "parakeet",
    family: "Nemotron Speech",
    provider: "NVIDIA",
    architecture: "Cache-aware FastConformer RNNT",
    languages: "English",
    speechMode: "Streaming ASR",
    speed: "Realtime",
    quality: "High",
    footprint: "0.6B",
    runtime: "Streaming runtime pending",
    license: "See model card",
    summary:
      "English streaming model with punctuation-oriented decoding and a cache-aware inference path.",
    note:
      "A stronger streaming candidate than the batch models when the goal is fast incremental text with cleaner punctuation.",
    bestFor: "Streaming English transcription with punctuation",
    capabilities: ["Cache-aware streaming", "Punctuation-friendly", "Batch + stream capable"],
    featureBadges: [
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "timed", label: "Punctuated", icon: "clock" },
      { id: "rnnt", label: "RNNT", icon: "spark" },
    ],
    highlights: [
      "Official NVIDIA streaming ASR family",
      "Good fit for future low-latency dictation",
      "Heavier than EOU but better for final streaming quality",
    ],
    hfUrl: "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b",
    artifactUrl:
      "https://huggingface.co/altunenes/parakeet-rs/tree/main/nemotron-speech-streaming-en-0.6b",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "streaming", "future"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 8 * 1024 ** 3,
    recommendedMemoryBytes: 16 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
  {
    id: "sortformer-v2",
    name: "Sortformer v2",
    modelKind: "parakeet",
    family: "Sortformer",
    provider: "NVIDIA",
    architecture: "Streaming diarization",
    languages: "Speaker labels",
    speechMode: "Speaker diarization",
    speed: "Realtime",
    quality: "High",
    footprint: "4 speakers",
    runtime: "Speaker pipeline pending",
    license: "See model card",
    summary:
      "Streaming diarization model for splitting live audio into speaker segments before transcription.",
    note:
      "Useful if we want speaker-prioritized or per-speaker transcripts instead of a single mixed transcript.",
    bestFor: "Real-time speaker separation",
    capabilities: ["Up to 4 speakers", "Streaming diarization", "Raw speaker probabilities"],
    featureBadges: [
      { id: "speaker", label: "Speaker", icon: "users" },
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "four", label: "4 spk", icon: "spark" },
    ],
    highlights: [
      "Speaker-first building block rather than a plain dictation model",
      "Pairs naturally with a future multitalker ASR path",
      "Useful for meetings and overlapping speech",
    ],
    hfUrl: "https://huggingface.co/nvidia/diar_streaming_sortformer_4spk-v2",
    tags: ["nvidia", "speaker", "streaming", "future"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 8 * 1024 ** 3,
    recommendedMemoryBytes: 12 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
  {
    id: "sortformer-v2-1",
    name: "Sortformer v2.1",
    modelKind: "parakeet",
    family: "Sortformer",
    provider: "NVIDIA",
    architecture: "Streaming diarization",
    languages: "Speaker labels",
    speechMode: "Speaker diarization",
    speed: "Realtime",
    quality: "Higher",
    footprint: "4 speakers",
    runtime: "Speaker pipeline pending",
    license: "See model card",
    summary:
      "Newer Sortformer diarization revision for speaker segmentation in live or chunked audio.",
    note:
      "Same integration class as v2, but worth surfacing because it is the newer NVIDIA speaker model card.",
    bestFor: "Higher-quality diarization",
    capabilities: ["Up to 4 speakers", "Streaming diarization", "Newer v2.1 revision"],
    featureBadges: [
      { id: "speaker", label: "Speaker", icon: "users" },
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "v21", label: "v2.1", icon: "clock" },
    ],
    highlights: [
      "Newer revision of the Sortformer speaker stack",
      "Good candidate for speaker-prioritized transcription work",
      "Would pair with multitalker or segmented batch transcription",
    ],
    hfUrl: "https://huggingface.co/nvidia/diar_streaming_sortformer_4spk-v2.1",
    tags: ["nvidia", "speaker", "streaming", "future"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 8 * 1024 ** 3,
    recommendedMemoryBytes: 12 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
  {
    id: "multitalker-parakeet",
    name: "Multitalker Parakeet",
    modelKind: "parakeet",
    family: "Parakeet + Sortformer",
    provider: "NVIDIA",
    architecture: "Speaker-aware streaming ASR",
    languages: "Speaker-separated",
    speechMode: "Speaker-aware ASR",
    speed: "Realtime",
    quality: "High",
    footprint: "0.6B pipeline",
    runtime: "Speaker pipeline pending",
    license: "See model card",
    summary:
      "Multi-instance streaming ASR pipeline that turns overlapping speech into per-speaker transcripts.",
    note:
      "This is the most direct route to speaker prioritization and separated transcripts from the current parakeet-rs feature set.",
    bestFor: "Per-speaker transcripts from mixed audio",
    capabilities: ["Per-speaker text", "Streaming pipeline", "Speaker-target injection"],
    featureBadges: [
      { id: "speaker", label: "Per speaker", icon: "users" },
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "pipeline", label: "Pipeline", icon: "spark" },
    ],
    highlights: [
      "Most aligned with future speaker prioritization in Transcribed",
      "Depends on both ASR and diarization paths",
      "Heaviest integration in the current NVIDIA set",
    ],
    hfUrl: "https://huggingface.co/nvidia/multitalker-parakeet-streaming-0.6b-v1",
    tags: ["nvidia", "speaker", "streaming", "future"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 12 * 1024 ** 3,
    recommendedMemoryBytes: 16 * 1024 ** 3,
    minimumCores: 10,
    recommendedCores: 12,
  },
];

export function formatModelSizeLabel(row: ModelRow) {
  if (row.diskSizeBytes && row.diskSizeBytes > 0) {
    return formatBytes(row.diskSizeBytes);
  }

  if (row.downloadSizeBytes && row.downloadSizeBytes > 0) {
    return formatBytes(row.downloadSizeBytes);
  }

  return row.footprint;
}

export function formatModelAudioLimit(row: ModelRow) {
  if (typeof row.audioLimitMs === "number" && row.audioLimitMs > 0) {
    const minutes = row.audioLimitMs / 60_000;
    const label = Number.isInteger(minutes)
      ? `${minutes.toFixed(0)} min`
      : `${minutes.toFixed(1)} min`;
    return `~${label} per pass`;
  }

  if (row.tags.includes("streaming")) {
    return "Streaming / chunk-based";
  }

  return "Not specified";
}

function isManagedModelPath(path: string) {
  return /[\\/]catalog-models(?:[\\/]|$)/i.test(path);
}

export function buildModelRows(snapshot: Snapshot): ModelRow[] {
  const activeModelId = snapshot.settings.selectedModelId;

  return MODEL_CATALOG.map<ModelRow>((entry) => {
    if (entry.id === "parakeet") {
      const installedPath = snapshot.settings.installedModelPaths.parakeet ?? null;
      const diskSizeBytes = snapshot.installedModelSizes.parakeet ?? 0;
      const builtInReady = snapshot.parakeetModelStatus === "ready";
      const isReady = builtInReady || Boolean(installedPath);
      const active =
        activeModelId === "parakeet" &&
        snapshot.settings.selectedModelKind === "parakeet" &&
        snapshot.modelStatus === "ready";

      return {
        ...entry,
        state: isReady ? "ready" : "downloadable",
        source: "built-in",
        managed: Boolean(installedPath && isManagedModelPath(installedPath)),
        active,
        selectable: isReady,
        runtime: builtInReady || installedPath ? "Ready in app" : "Download in app",
        note: builtInReady
          ? entry.note
          : installedPath
            ? "Downloaded into Transcribed and ready to use as the default local speech engine."
            : "Parakeet TDT is not bundled on this machine right now, but the compatible ONNX bundle can be downloaded in-app.",
        path: installedPath,
        diskSizeBytes,
      };
    }

    const installedPath = snapshot.settings.installedModelPaths[entry.id] ?? null;
    const isSelectedEngine =
      activeModelId === entry.id &&
      snapshot.settings.selectedModelKind === entry.modelKind;
    const runtimeDisabled = entry.id === "parakeet-ctc";
    const isReady = Boolean(installedPath);
    const supportsDownload = Boolean(entry.supportsDownload);
    const isManaged = Boolean(installedPath && isManagedModelPath(installedPath));

    return {
      ...entry,
      state: runtimeDisabled
        ? "planned"
        : isReady
          ? "ready"
          : supportsDownload
            ? "downloadable"
            : "planned",
      source: "catalog",
      managed: isManaged,
      active: !runtimeDisabled && isSelectedEngine && snapshot.modelStatus === "ready",
      selectable: !runtimeDisabled && isReady,
      runtime: isReady
        ? "Ready in app"
        : supportsDownload
          ? "Download in app"
          : entry.runtime,
      note: runtimeDisabled
        ? "Catalog reference only for now. The stable runtime currently falls back to Parakeet TDT."
        : isReady
          ? isManaged
            ? "Downloaded into Transcribed and ready to use locally."
            : "Linked to a local model folder. You can activate it from this catalog entry."
          : supportsDownload
            ? "Download this NVIDIA speech model or point Transcribed at an existing compatible folder."
            : entry.note,
      path: installedPath,
      diskSizeBytes: snapshot.installedModelSizes[entry.id] ?? 0,
      tags: isReady ? Array.from(new Set([...entry.tags, "available"])) : entry.tags,
    };
  });
}

function formatHardwareTarget(row: ModelRow) {
  const memoryLabel = row.recommendedMemoryBytes
    ? formatBytes(row.recommendedMemoryBytes)
    : row.minimumMemoryBytes
      ? formatBytes(row.minimumMemoryBytes)
      : null;
  const coreLabel = row.recommendedCores ?? row.minimumCores ?? null;

  if (memoryLabel && coreLabel) {
    return `${memoryLabel} RAM and ${coreLabel}+ threads`;
  }

  if (memoryLabel) {
    return `${memoryLabel} RAM`;
  }

  if (coreLabel) {
    return `${coreLabel}+ threads`;
  }

  return "unknown hardware target";
}

export function describeHardwareFit(row: ModelRow, profile: SystemProfile) {
  if (!row.minimumMemoryBytes && !row.minimumCores) {
    return {
      label: row.supportsDownload || row.selectable ? "Unknown" : "Planned",
      tone: row.supportsDownload || row.selectable ? "muted" : "warning",
      detail:
        row.supportsDownload || row.selectable
          ? "No hardware estimate for this model yet."
          : "This NVIDIA speech entry is reference-only for now.",
    } as const;
  }

  if (profile.totalMemoryBytes <= 0) {
    return {
      label: "Unknown",
      tone: "muted",
      detail: `Need RAM info to estimate fit. Target: ${formatHardwareTarget(row)}.`,
    } as const;
  }

  const memory = profile.totalMemoryBytes;
  const cores = profile.logicalCores;
  const minimumMemory = row.minimumMemoryBytes ?? 0;
  const recommendedMemory = row.recommendedMemoryBytes ?? minimumMemory;
  const minimumCores = row.minimumCores ?? 1;
  const recommendedCores = row.recommendedCores ?? minimumCores;

  if (memory >= recommendedMemory && cores >= recommendedCores) {
    return {
      label: "Great fit",
      tone: "success",
      detail: `${formatSystemProfile(profile)} should run ${row.name} comfortably.`,
    } as const;
  }

  if (memory >= minimumMemory && cores >= minimumCores) {
    return {
      label: "Should work",
      tone: "accent",
      detail: `${formatSystemProfile(profile)} should handle ${row.name}, but expect heavier CPU/RAM use than the recommended target of ${formatHardwareTarget(row)}.`,
    } as const;
  }

  return {
    label: "Heavy",
    tone: "warning",
    detail: `${formatSystemProfile(profile)} is below the recommended target of ${formatHardwareTarget(row)}.`,
  } as const;
}

export function modelSpeedScore(row: ModelRow) {
  switch (row.id) {
    case "parakeet":
      return 4.6;
    case "parakeet-ctc":
      return 4.4;
    case "parakeet-eou":
      return 4.9;
    case "nemotron-streaming":
      return 4.2;
    case "sortformer-v2":
      return 4.0;
    case "sortformer-v2-1":
      return 4.1;
    case "multitalker-parakeet":
      return 3.7;
    default:
      return 3;
  }
}

export function modelAccuracyScore(row: ModelRow) {
  switch (row.id) {
    case "parakeet":
      return 4.4;
    case "parakeet-ctc":
      return 4.1;
    case "parakeet-eou":
      return 3.9;
    case "nemotron-streaming":
      return 4.4;
    case "sortformer-v2":
      return 4.0;
    case "sortformer-v2-1":
      return 4.2;
    case "multitalker-parakeet":
      return 4.3;
    default:
      return 3.5;
  }
}

export function modelFeatureItems(row: ModelRow): ModelFeatureItem[] {
  return row.featureBadges;
}

export function matchesModel(row: ModelRow, query: string, filter: ModelFilter) {
  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery) {
    const haystack = [
      row.name,
      row.family,
      row.provider,
      row.architecture,
      row.speechMode,
      row.languages,
      row.speed,
      row.quality,
      row.footprint,
      formatModelSizeLabel(row),
      row.runtime,
      row.license,
      row.source,
      row.summary,
      row.note,
      row.bestFor,
      row.artifactLabel ?? "",
      ...row.capabilities,
      ...row.highlights,
      ...row.featureBadges.map((badge) => badge.label),
    ]
      .join(" ")
      .toLowerCase();

    if (!haystack.includes(normalizedQuery)) {
      return false;
    }
  }

  switch (filter) {
    case "available":
      return row.state === "ready" || row.state === "downloadable";
    case "streaming":
      return row.tags.includes("streaming");
    case "speaker":
      return row.tags.includes("speaker");
    case "all":
    default:
      return true;
  }
}

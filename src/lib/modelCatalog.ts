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
    supportsDefaultSelection: true,
    directmlCapable: true,
    summary:
      "Multilingual long-form offline dictation model for final microphone and file transcription.",
    note:
      "Transcribed uses parakeet-rs for this model and will prefer DirectML on Windows, then fall back to CPU if needed. In-app chunking now treats TDT v3 as the long-form batch option.",
    bestFor: "Long-form multilingual dictation",
    capabilities: ["TDT decoder", "Auto language detection", "Token timestamps"],
    featureBadges: [
      { id: "local", label: "Local", icon: "cpu" },
      { id: "tdt", label: "TDT", icon: "spark" },
      { id: "directml", label: "DirectML", icon: "bolt" },
      { id: "timed", label: "Timed", icon: "clock" },
    ],
    highlights: [
      "Current default final transcription engine in Transcribed",
      "Uses parakeet-rs with DirectML preference on Windows",
      "Best balance of speed, multilingual coverage, and long-form support today",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3",
    artifactUrl: "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8",
    artifactLabel: "Compatible ONNX bundle",
    tags: ["nvidia", "offline", "timestamps", "default"],
    supportsInstall: false,
    supportsDownload: true,
    downloadSizeBytes: 593_000_000,
    audioLimitMs: 24 * 60 * 1_000,
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
    runtime: "Ready when installed",
    license: "See model card",
    supportsDefaultSelection: true,
    directmlCapable: true,
    summary:
      "English-first Parakeet variant aimed at fast offline transcription with punctuation and capitalization.",
    note:
      "Uses the same parakeet-rs runtime path as TDT, but with a CTC decoder and English-focused ONNX export. Kept on a shorter soft chunk size than TDT in Transcribed.",
    bestFor: "English punctuation-heavy offline transcription",
    capabilities: ["CTC decoding", "Punctuation and caps", "Word timestamps"],
    featureBadges: [
      { id: "offline", label: "Offline", icon: "cpu" },
      { id: "ctc", label: "CTC", icon: "spark" },
      { id: "directml", label: "DirectML", icon: "bolt" },
      { id: "timed", label: "Timed", icon: "clock" },
    ],
    highlights: [
      "Real selectable batch model in Transcribed",
      "English-focused Parakeet family variant",
      "Useful when you prefer a simpler CTC decoding path",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-ctc-0.6b",
    artifactUrl:
      "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/tree/main/onnx",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "offline", "timestamps"],
    supportsInstall: false,
    supportsDownload: true,
    downloadSizeBytes: 614_000_000,
    audioLimitMs: 10 * 60 * 1_000,
    minimumGpuMemoryBytes: 2 * 1024 ** 3,
    recommendedGpuMemoryBytes: 4 * 1024 ** 3,
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
    runtime: "Streaming add-on",
    license: "See model card",
    supportsDefaultSelection: false,
    directmlCapable: true,
    summary:
      "Lightweight streaming ASR model with end-of-utterance detection for low-latency voice UX.",
    note:
      "The best candidate for a low-latency live transcript path, with GPU acceleration available on Windows through DirectML in parakeet-rs.",
    bestFor: "Low-latency live dictation",
    capabilities: ["EOU detection", "160 ms chunking", "Stateful streaming"],
    unlockedFeatures: ["Live transcript", "Low-latency preview", "Long-form guidance"],
    featureBadges: [
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "eou", label: "EOU", icon: "clock" },
      { id: "directml", label: "DirectML", icon: "bolt" },
    ],
    highlights: [
      "Lowest-footprint speech model in the current supported set",
      "Built for real-time incremental updates",
      "Used only for live preview, never final pasted text",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet_realtime_eou_120m-v1",
    artifactUrl:
      "https://huggingface.co/altunenes/parakeet-rs/tree/main/realtime_eou_120m-v1-onnx",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "streaming"],
    supportsInstall: false,
    supportsDownload: true,
    downloadSizeBytes: 480_708_981,
    minimumGpuMemoryBytes: 2 * 1024 ** 3,
    recommendedGpuMemoryBytes: 4 * 1024 ** 3,
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
    runtime: "Streaming add-on",
    license: "See model card",
    supportsDefaultSelection: false,
    directmlCapable: true,
    summary:
      "English streaming model with punctuation-oriented decoding and a cache-aware inference path.",
    note:
      "A stronger live transcript candidate than the batch models when you want cleaner punctuation, with DirectML GPU acceleration available on Windows through parakeet-rs.",
    bestFor: "Streaming English transcription with punctuation",
    capabilities: ["Cache-aware streaming", "Punctuation-friendly", "Batch + stream capable"],
    unlockedFeatures: ["Live transcript", "Punctuated live transcript", "Long-form guidance"],
    featureBadges: [
      { id: "stream", label: "Streaming", icon: "bolt" },
      { id: "timed", label: "Punctuated", icon: "clock" },
      { id: "directml", label: "DirectML", icon: "bolt" },
    ],
    highlights: [
      "Higher-quality live preview option than EOU",
      "Better punctuation than the lighter preview path",
      "Used only for live preview, never final pasted text",
    ],
    hfUrl: "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b",
    artifactUrl:
      "https://huggingface.co/altunenes/parakeet-rs/tree/main/nemotron-speech-streaming-en-0.6b",
    artifactLabel: "Compatible ONNX export",
    tags: ["nvidia", "streaming"],
    supportsInstall: false,
    supportsDownload: true,
    downloadSizeBytes: 2_515_376_329,
    minimumGpuMemoryBytes: 4 * 1024 ** 3,
    recommendedGpuMemoryBytes: 8 * 1024 ** 3,
    minimumMemoryBytes: 8 * 1024 ** 3,
    recommendedMemoryBytes: 16 * 1024 ** 3,
    minimumCores: 8,
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
    const hours = row.audioLimitMs / 3_600_000;
    const minutes = row.audioLimitMs / 60_000;
    const label =
      hours >= 1
        ? Number.isInteger(hours)
          ? `${hours.toFixed(0)} hr`
          : `${hours.toFixed(1)} hr`
        : Number.isInteger(minutes)
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
            ? "Downloaded into Transcribed and ready as the default final transcription engine."
            : "Download this managed TDT bundle into Transcribed to use it for final microphone and file transcription.",
        path: installedPath,
        diskSizeBytes,
      };
    }

    const installedPath = snapshot.settings.installedModelPaths[entry.id] ?? null;
    const isSelectedEngine =
      activeModelId === entry.id &&
      snapshot.settings.selectedModelKind === entry.modelKind;
    const supportsDefaultSelection = entry.supportsDefaultSelection !== false;
    const isReady = Boolean(installedPath);
    const supportsDownload = Boolean(entry.supportsDownload);
    const isManaged = Boolean(installedPath && isManagedModelPath(installedPath));

    return {
      ...entry,
      state: isReady ? "ready" : supportsDownload ? "downloadable" : "planned",
      source: "catalog",
      managed: isManaged,
      active:
        supportsDefaultSelection &&
        isSelectedEngine &&
        snapshot.modelStatus === "ready",
      selectable: supportsDefaultSelection && isReady,
      runtime: isReady
        ? supportsDefaultSelection
          ? "Ready in app"
          : entry.directmlCapable && snapshot.systemProfile.directmlAvailable
            ? "Installed add-on · DirectML ready"
            : "Installed add-on"
        : supportsDownload
          ? "Download in app"
          : entry.runtime,
      note: isReady
        ? supportsDefaultSelection
          ? "Downloaded into Transcribed and ready as a selectable final transcription engine."
          : `Installed in Transcribed. Live preview can now use ${entry.name}.`
        : supportsDownload
          ? supportsDefaultSelection
            ? "Download this model into Transcribed, then choose it as the Default speech model for final dictation."
            : "Download this streaming add-on into Transcribed to unlock live preview with this model."
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
  const gpuLabel = row.recommendedGpuMemoryBytes
    ? formatBytes(row.recommendedGpuMemoryBytes)
    : row.minimumGpuMemoryBytes
      ? formatBytes(row.minimumGpuMemoryBytes)
      : null;
  const coreLabel = row.recommendedCores ?? row.minimumCores ?? null;

  if (memoryLabel && gpuLabel && coreLabel) {
    return `${memoryLabel} RAM, ${gpuLabel} VRAM, and ${coreLabel}+ threads`;
  }

  if (memoryLabel && coreLabel) {
    return `${memoryLabel} RAM and ${coreLabel}+ threads`;
  }

  if (memoryLabel) {
    return `${memoryLabel} RAM`;
  }

  if (gpuLabel) {
    return `${gpuLabel} VRAM`;
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
  const gpuMemory = profile.gpuMemoryBytes;
  const minimumMemory = row.minimumMemoryBytes ?? 0;
  const recommendedMemory = row.recommendedMemoryBytes ?? minimumMemory;
  const minimumGpuMemory = row.minimumGpuMemoryBytes ?? 0;
  const recommendedGpuMemory =
    row.recommendedGpuMemoryBytes ?? minimumGpuMemory;
  const minimumCores = row.minimumCores ?? 1;
  const recommendedCores = row.recommendedCores ?? minimumCores;
  const requiresGpu = Boolean(row.directmlCapable);
  const gpuMissing = requiresGpu && !profile.directmlAvailable;
  const gpuRecommendedMet =
    !requiresGpu || recommendedGpuMemory <= 0 || gpuMemory >= recommendedGpuMemory;
  const gpuMinimumMet =
    !requiresGpu || minimumGpuMemory <= 0 || gpuMemory >= minimumGpuMemory;

  if (gpuMissing) {
    return {
      label: "CPU only",
      tone: "warning",
      detail: `${row.name} can use DirectML GPU acceleration on Windows, but this machine does not currently look DirectML-ready. Target: ${formatHardwareTarget(row)}.`,
    } as const;
  }

  if (
    memory >= recommendedMemory &&
    cores >= recommendedCores &&
    gpuRecommendedMet
  ) {
    return {
      label: requiresGpu ? "Great fit · DirectML" : "Great fit",
      tone: "success",
      detail: `${formatSystemProfile(profile)} should run ${row.name} comfortably.`,
    } as const;
  }

  if (memory >= minimumMemory && cores >= minimumCores && gpuMinimumMet) {
    return {
      label: requiresGpu ? "Should work · DirectML" : "Should work",
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
    case "all":
    default:
      return true;
  }
}

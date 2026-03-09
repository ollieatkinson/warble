import type { ModelFilter, ModelRow, Snapshot, SystemProfile } from "../types";
import { formatBytes, formatSystemProfile } from "./utils";

export type ModelFeatureItem = {
  id: string;
  label: string;
  icon: "spark" | "cpu" | "globe" | "input" | "bolt";
};

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
    speed: "Fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Ready in app",
    license: "See model card",
    summary:
      "Multilingual Parakeet TDT with local ONNX inference and fast dictation latency.",
    note:
      "Best current in-app path for local dictation, auto language detection, and broader language support.",
    highlights: [
      "Current default engine in Transcribed",
      "Uses the stable local Rust ONNX backend",
      "Supports timestamps and local offline dictation",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3",
    artifactUrl: "https://huggingface.co/smcleod/parakeet-tdt-0.6b-v3-int8",
    artifactLabel: "ONNX export bundle",
    tags: ["multilingual", "nvidia", "available", "timestamps"],
    supportsInstall: true,
    supportsDownload: true,
    downloadSizeBytes: 593_000_000,
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
    speed: "Very fast",
    quality: "High",
    footprint: "0.6B",
    runtime: "Stable runtime pending",
    license: "See model card",
    summary:
      "English-only CTC variant in the catalog, held back until the stable local runtime is ready.",
    note:
      "Shown here for comparison, but not currently enabled in the stable Transcribed runtime.",
    highlights: [
      "English-focused Parakeet family variant",
      "Stronger punctuation-oriented CTC path",
      "Kept catalog-only until the runtime is re-enabled",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet-ctc-0.6b",
    artifactUrl:
      "https://huggingface.co/onnx-community/parakeet-ctc-0.6b-ONNX/tree/main/onnx",
    artifactLabel: "ONNX export bundle",
    tags: ["english", "nvidia", "available", "timestamps"],
    supportsInstall: false,
    supportsDownload: false,
    downloadSizeBytes: 1_240_000_000,
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
    architecture: "Streaming encoder-decoder + EOU",
    languages: "English",
    speed: "Realtime",
    quality: "Balanced",
    footprint: "120M",
    runtime: "Streaming integration pending",
    license: "See model card",
    summary:
      "Realtime Parakeet model with end-of-utterance detection, exposed by parakeet-rs.",
    note:
      "Best next target for a truer live transcription experience, but not yet wired into the app session flow.",
    highlights: [
      "Designed for chunked live transcription",
      "End-of-utterance aware",
      "Good candidate for a future streaming HUD mode",
    ],
    hfUrl: "https://huggingface.co/nvidia/parakeet_realtime_eou_120m-v1",
    tags: ["english", "nvidia", "future", "streaming"],
    supportsInstall: false,
    supportsDownload: false,
    minimumMemoryBytes: 4 * 1024 ** 3,
    recommendedMemoryBytes: 8 * 1024 ** 3,
    minimumCores: 8,
    recommendedCores: 12,
  },
  {
    id: "nemotron-streaming",
    name: "Nemotron Streaming",
    modelKind: "parakeet",
    family: "Parakeet",
    provider: "NVIDIA",
    architecture: "Cache-aware streaming RNNT",
    languages: "English",
    speed: "Realtime",
    quality: "High",
    footprint: "0.6B",
    runtime: "Streaming integration pending",
    license: "See model card",
    summary:
      "Streaming Nemotron speech model supported by parakeet-rs for chunked ASR.",
    note:
      "Promising for future low-latency dictation with punctuation, but not yet connected to the current app flow.",
    highlights: [
      "Cache-aware streaming path",
      "Good punctuation-oriented future option",
      "Not yet supported in-app",
    ],
    hfUrl: "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b",
    tags: ["english", "nvidia", "future", "streaming"],
    supportsInstall: false,
    supportsDownload: false,
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
            ? "Downloaded into Transcribed and ready to use as the local Parakeet engine."
            : "Parakeet isn't bundled on this machine right now, but you can download the compatible ONNX bundle directly in the app.",
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
          : "Linked to a local Parakeet model folder. You can activate it from this catalog entry."
        : supportsDownload
          ? "Download this Parakeet variant from Hugging Face or point Transcribed at an existing compatible model folder."
          : "Reference-only for now. Browse the model card, but the runtime is not wired into Transcribed yet.",
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
          : "This catalog entry is reference-only for now.",
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
      return 4.3;
    case "parakeet-eou":
      return 4.8;
    case "nemotron-streaming":
      return 4.1;
    default:
      return 3;
  }
}

export function modelAccuracyScore(row: ModelRow) {
  switch (row.id) {
    case "parakeet":
      return 4.3;
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
  return [
    {
      id: "runtime",
      label:
        row.state === "planned"
          ? "Research"
          : row.supportsDownload || row.selectable
            ? "Local"
            : "Catalog",
      icon: row.state === "planned" ? "spark" : "cpu",
    },
    {
      id: "language",
      label: row.tags.includes("multilingual") ? "Multilingual" : "English",
      icon: row.tags.includes("multilingual") ? "globe" : "input",
    },
    {
      id: "focus",
      label: row.tags.includes("streaming")
        ? "Streaming"
        : row.id === "parakeet"
          ? "Dictation"
          : row.id === "parakeet-ctc"
            ? "English"
            : "Reference",
      icon: row.tags.includes("streaming")
        ? "bolt"
        : row.id === "parakeet"
          ? "input"
          : "spark",
    },
  ];
}

export function matchesModel(row: ModelRow, query: string, filter: ModelFilter) {
  const normalizedQuery = query.trim().toLowerCase();
  if (normalizedQuery) {
    const haystack = [
      row.name,
      row.family,
      row.provider,
      row.architecture,
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
      row.artifactLabel ?? "",
      ...row.highlights,
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
    case "multilingual":
      return row.tags.includes("multilingual");
    case "future":
      return row.state === "planned";
    case "all":
    default:
      return true;
  }
}

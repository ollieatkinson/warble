import {
  createModelRow,
  createSnapshot,
  createSystemProfile,
} from "../../test/fixtures";

import {
  formatModelSizeLabel,
  formatModelAudioLimit,
  buildModelRows,
  describeHardwareFit,
  modelSpeedScore,
  modelAccuracyScore,
  matchesModel,
  modelFeatureItems,
} from "../modelCatalog";

// ---------------------------------------------------------------------------
// formatModelSizeLabel
// ---------------------------------------------------------------------------
describe("formatModelSizeLabel", () => {
  it("prefers diskSizeBytes when present", () => {
    const row = createModelRow({ diskSizeBytes: 1_073_741_824, downloadSizeBytes: 500_000_000 });
    expect(formatModelSizeLabel(row)).toBe("1.0 GB");
  });

  it("falls back to downloadSizeBytes when diskSizeBytes is absent", () => {
    const row = createModelRow({ diskSizeBytes: undefined, downloadSizeBytes: 593_000_000 });
    expect(formatModelSizeLabel(row)).toBe("566 MB");
  });

  it("falls back to footprint when both sizes are absent", () => {
    const row = createModelRow({
      diskSizeBytes: undefined,
      downloadSizeBytes: undefined,
      footprint: "0.6B",
    });
    expect(formatModelSizeLabel(row)).toBe("0.6B");
  });
});

// ---------------------------------------------------------------------------
// formatModelAudioLimit
// ---------------------------------------------------------------------------
describe("formatModelAudioLimit", () => {
  it("formats 24-minute limit for parakeet", () => {
    const row = createModelRow({ audioLimitMs: 24 * 60 * 1000 });
    expect(formatModelAudioLimit(row)).toBe("~24 min per pass");
  });

  it("formats 10-minute limit for parakeet-ctc", () => {
    const row = createModelRow({ audioLimitMs: 10 * 60 * 1000 });
    expect(formatModelAudioLimit(row)).toBe("~10 min per pass");
  });

  it("returns streaming label for a streaming model with no audio limit", () => {
    const row = createModelRow({
      audioLimitMs: undefined,
      tags: ["nvidia", "streaming"],
    });
    expect(formatModelAudioLimit(row)).toBe("Streaming / chunk-based");
  });

  it("returns 'Not specified' when no limit and not streaming", () => {
    const row = createModelRow({
      audioLimitMs: undefined,
      tags: ["nvidia", "offline"],
    });
    expect(formatModelAudioLimit(row)).toBe("Not specified");
  });
});

// ---------------------------------------------------------------------------
// buildModelRows
// ---------------------------------------------------------------------------
describe("buildModelRows", () => {
  it("returns exactly 4 rows from the catalog", () => {
    const snapshot = createSnapshot();
    const rows = buildModelRows(snapshot);
    expect(rows).toHaveLength(4);
  });

  it("marks parakeet as ready / built-in when parakeetModelStatus is ready", () => {
    const snapshot = createSnapshot({ parakeetModelStatus: "ready" });
    const rows = buildModelRows(snapshot);
    const parakeet = rows.find((r) => r.id === "parakeet")!;
    expect(parakeet.state).toBe("ready");
    expect(parakeet.source).toBe("built-in");
  });

  it("marks a non-installed catalog model as downloadable", () => {
    const snapshot = createSnapshot({
      parakeetModelStatus: "not-available",
      settings: { installedModelPaths: {} },
    });
    const rows = buildModelRows(snapshot);
    const ctc = rows.find((r) => r.id === "parakeet-ctc")!;
    expect(ctc.state).toBe("downloadable");
  });
});

// ---------------------------------------------------------------------------
// describeHardwareFit
// ---------------------------------------------------------------------------
describe("describeHardwareFit", () => {
  it("returns 'Great fit' when recommended specs are met with GPU", () => {
    const row = createModelRow({
      minimumMemoryBytes: 4 * 1024 ** 3,
      recommendedMemoryBytes: 8 * 1024 ** 3,
      minimumCores: 4,
      recommendedCores: 8,
      supportedAccelerationProviders: ["directml"],
    });
    const profile = createSystemProfile({
      totalMemoryBytes: 16 * 1024 ** 3,
      logicalCores: 8,
      gpuMemoryBytes: 6 * 1024 ** 3,
      supportedAccelerationProviders: ["directml"],
    });
    const result = describeHardwareFit(row, profile);
    expect(result.label).toContain("Great fit");
    expect(result.tone).toBe("success");
  });

  it("returns 'Should work' when only minimum specs are met", () => {
    const row = createModelRow({
      minimumMemoryBytes: 4 * 1024 ** 3,
      recommendedMemoryBytes: 16 * 1024 ** 3,
      minimumCores: 4,
      recommendedCores: 12,
      supportedAccelerationProviders: ["directml"],
    });
    const profile = createSystemProfile({
      totalMemoryBytes: 8 * 1024 ** 3,
      logicalCores: 6,
      gpuMemoryBytes: 6 * 1024 ** 3,
      supportedAccelerationProviders: ["directml"],
    });
    const result = describeHardwareFit(row, profile);
    expect(result.label).toContain("Should work");
    expect(result.tone).toBe("accent");
  });

  it("returns 'Heavy' when below minimum specs", () => {
    const row = createModelRow({
      minimumMemoryBytes: 8 * 1024 ** 3,
      recommendedMemoryBytes: 16 * 1024 ** 3,
      minimumCores: 8,
      recommendedCores: 12,
      supportedAccelerationProviders: ["directml"],
    });
    const profile = createSystemProfile({
      totalMemoryBytes: 4 * 1024 ** 3,
      logicalCores: 4,
      gpuMemoryBytes: 6 * 1024 ** 3,
      supportedAccelerationProviders: ["directml"],
    });
    const result = describeHardwareFit(row, profile);
    expect(result.label).toBe("Heavy");
    expect(result.tone).toBe("warning");
  });

  it("returns 'CPU only' when model supports GPU but none is available", () => {
    const row = createModelRow({
      minimumMemoryBytes: 4 * 1024 ** 3,
      recommendedMemoryBytes: 8 * 1024 ** 3,
      minimumCores: 4,
      recommendedCores: 8,
      supportedAccelerationProviders: ["directml", "webgpu"],
    });
    const profile = createSystemProfile({
      totalMemoryBytes: 4 * 1024 ** 3,
      logicalCores: 2,
      gpuMemoryBytes: 0,
      gpuName: "",
      supportedAccelerationProviders: [],
    });
    const result = describeHardwareFit(row, profile);
    expect(result.label).toBe("CPU only");
    expect(result.tone).toBe("warning");
  });

  it("returns 'Unknown' when model has no hardware specs", () => {
    const row = createModelRow({
      minimumMemoryBytes: undefined,
      recommendedMemoryBytes: undefined,
      minimumCores: undefined,
      recommendedCores: undefined,
      selectable: true,
      supportsDownload: true,
    });
    const profile = createSystemProfile();
    const result = describeHardwareFit(row, profile);
    expect(result.label).toBe("Unknown");
    expect(result.tone).toBe("muted");
  });
});

// ---------------------------------------------------------------------------
// modelSpeedScore
// ---------------------------------------------------------------------------
describe("modelSpeedScore", () => {
  it.each([
    ["parakeet", 4.6],
    ["parakeet-ctc", 4.4],
    ["parakeet-eou", 4.9],
    ["nemotron-streaming", 4.2],
  ] as const)("returns %s -> %s", (id, expected) => {
    const row = createModelRow({ id });
    expect(modelSpeedScore(row)).toBe(expected);
  });

  it("returns 3 for an unknown model id", () => {
    const row = createModelRow({ id: "unknown-model" });
    expect(modelSpeedScore(row)).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// modelAccuracyScore
// ---------------------------------------------------------------------------
describe("modelAccuracyScore", () => {
  it.each([
    ["parakeet", 4.4],
    ["parakeet-ctc", 4.1],
    ["parakeet-eou", 3.9],
    ["nemotron-streaming", 4.4],
  ] as const)("returns %s -> %s", (id, expected) => {
    const row = createModelRow({ id });
    expect(modelAccuracyScore(row)).toBe(expected);
  });

  it("returns 3.5 for an unknown model id", () => {
    const row = createModelRow({ id: "unknown-model" });
    expect(modelAccuracyScore(row)).toBe(3.5);
  });
});

// ---------------------------------------------------------------------------
// matchesModel
// ---------------------------------------------------------------------------
describe("matchesModel", () => {
  it("matches by text query across row fields", () => {
    const row = createModelRow({ name: "Parakeet TDT", provider: "NVIDIA" });
    expect(matchesModel(row, "parakeet", "all")).toBe(true);
    expect(matchesModel(row, "NVIDIA", "all")).toBe(true);
  });

  it("returns false when text query does not match", () => {
    const row = createModelRow({ name: "Parakeet TDT" });
    expect(matchesModel(row, "whisper", "all")).toBe(false);
  });

  it("filters by 'streaming' tag", () => {
    const streamingRow = createModelRow({ tags: ["nvidia", "streaming"], state: "ready" });
    const batchRow = createModelRow({ tags: ["nvidia", "offline"], state: "ready" });
    expect(matchesModel(streamingRow, "", "streaming")).toBe(true);
    expect(matchesModel(batchRow, "", "streaming")).toBe(false);
  });

  it("filters by 'available' — includes ready and downloadable states", () => {
    const readyRow = createModelRow({ state: "ready" });
    const downloadableRow = createModelRow({ state: "downloadable" });
    const plannedRow = createModelRow({ state: "planned" });
    expect(matchesModel(readyRow, "", "available")).toBe(true);
    expect(matchesModel(downloadableRow, "", "available")).toBe(true);
    expect(matchesModel(plannedRow, "", "available")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// modelFeatureItems
// ---------------------------------------------------------------------------
describe("modelFeatureItems", () => {
  it("returns the featureBadges array from the row", () => {
    const badges = [
      { id: "local", label: "Local", icon: "cpu" as const },
      { id: "tdt", label: "TDT", icon: "spark" as const },
    ];
    const row = createModelRow({ featureBadges: badges });
    expect(modelFeatureItems(row)).toBe(badges);
  });
});

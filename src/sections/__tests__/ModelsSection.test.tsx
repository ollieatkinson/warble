import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

vi.mock("../../components/icons", () => ({
  BoltIcon: () => null,
  CheckIcon: () => null,
  ChevronDownIcon: () => null,
  ClockIcon: () => null,
  CpuIcon: () => null,
  DownloadIcon: () => null,
  ExternalIcon: () => null,
  SparkIcon: () => null,
  TrashIcon: () => null,
}));

import { ModelsSection } from "../../sections/ModelsSection";
import {
  createModelRow,
  createSnapshot,
  createSystemProfile,
} from "../../test/fixtures";
import type {
  ChoiceOption,
  LivePreviewModel,
  ModelRow,
  Snapshot,
} from "../../types";

function createSectionRows() {
  const batchModels: ModelRow[] = [
    createModelRow({
      id: "parakeet",
      name: "Parakeet TDT",
      modelKind: "parakeet",
      active: true,
      selectable: true,
      state: "ready",
      source: "built-in",
      managed: false,
      tags: ["nvidia", "offline"],
    }),
    createModelRow({
      id: "parakeet-ctc",
      name: "Parakeet CTC",
      modelKind: "parakeet-ctc",
      active: false,
      selectable: true,
      state: "ready",
      source: "catalog",
      managed: false,
      tags: ["nvidia", "offline"],
    }),
  ];
  const streamingModels: ModelRow[] = [
    createModelRow({
      id: "parakeet-eou",
      name: "Parakeet Realtime EOU",
      active: false,
      selectable: false,
      state: "ready",
      source: "catalog",
      managed: false,
      tags: ["nvidia", "streaming"],
      unlockedFeatures: ["Live transcript"],
    }),
    createModelRow({
      id: "nemotron-streaming",
      name: "Nemotron Streaming",
      active: false,
      selectable: false,
      state: "ready",
      source: "catalog",
      managed: false,
      tags: ["nvidia", "streaming"],
      unlockedFeatures: ["Live transcript"],
    }),
  ];

  return { batchModels, streamingModels };
}

function renderModelsSection(
  snapshot: Snapshot,
  onChooseModelRuntime = vi.fn(),
) {
  const { batchModels, streamingModels } = createSectionRows();
  const readyModelOptions: ChoiceOption[] = batchModels.map((row) => ({
    id: row.id,
    label: row.name,
  }));
  const livePreviewOptions: ChoiceOption[] = [
    { id: "auto", label: "Auto" },
    { id: "nemotron-streaming", label: "Nemotron Streaming" },
    { id: "parakeet-eou", label: "Parakeet Realtime EOU" },
  ];

  render(
    <ModelsSection
      snapshot={snapshot}
      batchModels={batchModels}
      streamingModels={streamingModels}
      activeModel={batchModels[0]}
      activeReadyModelId="parakeet"
      readyModelOptions={readyModelOptions}
      livePreviewModel={"auto" as LivePreviewModel}
      livePreviewOptions={livePreviewOptions}
      effectiveLivePreviewModelId="nemotron-streaming"
      buttonFeedback={{}}
      onChooseDefaultModel={() => {}}
      onChooseLivePreviewModel={() => {}}
      onChooseModelRuntime={onChooseModelRuntime}
      onActivateModel={() => {}}
      onDownloadCatalogModel={() => {}}
      onRemoveCatalogModel={() => {}}
      onOpenModelReference={() => {}}
    />,
  );

  return { onChooseModelRuntime };
}

function findModelRow(name: string) {
  return (
    screen
      .getAllByText(name)
      .map((element) => element.closest("article"))
      .find((element): element is HTMLElement =>
        Boolean(element?.classList.contains("model-list-row")),
      ) ?? null
  );
}

describe("ModelsSection", () => {
  it("renders per-model runtime dropdowns only on macOS", () => {
    renderModelsSection(
      createSnapshot({
        platform: "macos",
        systemProfile: createSystemProfile({
          supportedAccelerationProviders: ["coreml"],
        }),
      }),
    );

    expect(screen.getAllByText("Runtime")).toHaveLength(4);
  });

  it("does not render per-model runtime dropdowns on other platforms", () => {
    renderModelsSection(
      createSnapshot({
        platform: "windows",
        systemProfile: createSystemProfile({
          supportedAccelerationProviders: ["directml"],
        }),
      }),
    );

    expect(screen.queryByText("Runtime")).not.toBeInTheDocument();
  });

  it("keeps runtime selection scoped to each row", async () => {
    const user = userEvent.setup();
    const { onChooseModelRuntime } = renderModelsSection(
      createSnapshot({
        platform: "macos",
        systemProfile: createSystemProfile({
          supportedAccelerationProviders: ["coreml"],
        }),
        settings: {
          macosModelRuntimePreferences: {
            "nemotron-streaming": "coreml",
          },
        },
      }),
    );

    const parakeetRow = findModelRow("Parakeet TDT");
    const nemotronRow = findModelRow("Nemotron Streaming");

    expect(parakeetRow).not.toBeNull();
    expect(nemotronRow).not.toBeNull();
    expect(
      within(nemotronRow!).getByRole("button", {
        name: "CoreML (Experimental)",
      }),
    ).toBeInTheDocument();

    await user.click(within(parakeetRow!).getByRole("button", { name: "CPU" }));
    await user.click(
      within(parakeetRow!).getByRole("button", {
        name: /CoreML \(Experimental\)/,
      }),
    );

    expect(onChooseModelRuntime).toHaveBeenCalledTimes(1);
    expect(onChooseModelRuntime).toHaveBeenCalledWith("parakeet", "coreml");
    expect(
      within(nemotronRow!).getByRole("button", {
        name: "CoreML (Experimental)",
      }),
    ).toBeInTheDocument();
  });
});

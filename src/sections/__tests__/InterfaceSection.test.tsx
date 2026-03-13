import "../../test/tauriMock";

import { render, screen, within } from "@testing-library/react";

import { InterfaceSection } from "../../sections/InterfaceSection";
import { createSettingsDraft, createSnapshot } from "../../test/fixtures";

describe("InterfaceSection", () => {
  it("disables transcript width selection for Dynamic Island mode", () => {
    render(
      <InterfaceSection
        snapshot={createSnapshot({
          platform: "macos",
          dynamicIslandAvailable: true,
          dynamicIslandMetrics: {
            x: 640,
            y: 0,
            width: 118,
            height: 32,
          },
        })}
        draft={createSettingsDraft({
          overlayPosition: "dynamic-island",
          showLiveTranscription: true,
        })}
        onApplySettings={() => {}}
      />,
    );

    const widthRow = screen.getByText("Transcript width").closest(".setting-row");
    expect(widthRow).not.toBeNull();
    expect(within(widthRow as HTMLElement).getByRole("button")).toBeDisabled();
    expect(
      screen.getByText("Dynamic Island mode sizes this to the island automatically."),
    ).toBeInTheDocument();
    expect(screen.getByText("Automatic width")).toBeInTheDocument();
  });
});

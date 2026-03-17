import "../../test/tauriMock";

import { render, screen } from "@testing-library/react";

import { InterfaceSection } from "../../sections/InterfaceSection";
import { createSettingsDraft, createSnapshot } from "../../test/fixtures";

describe("InterfaceSection", () => {
  it("renders the transcript width control when live transcription is enabled", () => {
    render(
      <InterfaceSection
        snapshot={createSnapshot({
          platform: "macos",
        })}
        draft={createSettingsDraft({
          overlayPosition: "bottom-center",
          showLiveTranscription: true,
        })}
        onApplySettings={() => {}}
      />,
    );

    expect(screen.getByText("Transcript width")).toBeInTheDocument();
    expect(
      screen.getByText("Reserve more room for the newest words in the pill."),
    ).toBeInTheDocument();
  });
});

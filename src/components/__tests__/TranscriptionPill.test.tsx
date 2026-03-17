import "../../test/tauriMock";

import { render } from "@testing-library/react";

import {
  InterfacePreviewCard,
  TranscriptionPill,
} from "../../components/TranscriptionPill";

describe("TranscriptionPill", () => {
  it("renders the standard pill layout", () => {
    const { container } = render(
      <TranscriptionPill
        phase="recording"
        title="Listening"
        detail="Capturing live transcript"
        levels={Array(12).fill(0.25)}
        overlayPosition="bottom-center"
        animationStyle="spectrum"
        showRecordingTimer
        showLiveTranscription
        liveTranscriptWidth="wide"
        liveTranscriptLines="three"
        onCancel={() => {}}
      />,
    );

    const shell = container.querySelector(".indicator-shell");
    expect(shell).toBeInTheDocument();
    expect(shell).toHaveClass("indicator-shell-detail-width-wide");
    expect(container.querySelector(".indicator-live-viewport-lines-three")).toBeInTheDocument();
  });
});

describe("InterfacePreviewCard", () => {
  it("applies width preset classes in standard mode", () => {
    const { container } = render(
      <InterfacePreviewCard
        overlayPosition="bottom-center"
        animationStyle="spectrum"
        showRecordingTimer
        showLiveTranscription
        liveTranscriptWidth="wide"
        liveTranscriptLines="two"
      />,
    );

    const screen = container.querySelector(".interface-demo-screen");
    expect(screen).toHaveClass("interface-demo-screen-bottom-center");
    expect(screen).toHaveClass("interface-demo-screen-width-wide");
  });
});

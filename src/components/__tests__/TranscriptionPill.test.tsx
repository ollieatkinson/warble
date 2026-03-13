import "../../test/tauriMock";

import { render } from "@testing-library/react";

import {
  InterfacePreviewCard,
  TranscriptionPill,
} from "../../components/TranscriptionPill";

const dynamicIslandMetrics = {
  x: 640,
  y: 0,
  width: 118,
  height: 32,
} as const;

describe("TranscriptionPill", () => {
  it("renders the dedicated Dynamic Island pod and tray layout", () => {
    const { container } = render(
      <TranscriptionPill
        phase="recording"
        title="Listening"
        detail="Capturing live transcript"
        levels={Array(12).fill(0.25)}
        overlayPosition="dynamic-island"
        animationStyle="spectrum"
        showRecordingTimer
        showLiveTranscription
        liveTranscriptWidth="wide"
        liveTranscriptLines="three"
        dynamicIslandMetrics={dynamicIslandMetrics}
        onCancel={() => {}}
      />,
    );

    const shell = container.querySelector(".indicator-shell-dynamic-island");
    expect(shell).toBeInTheDocument();
    expect(shell).not.toHaveClass("indicator-shell-detail-width-wide");
    expect(shell).toHaveStyle("--indicator-dynamic-island-gap-width: 118px");
    expect(container.querySelector(".indicator-dynamic-island-pod-left")).toBeInTheDocument();
    expect(container.querySelector(".indicator-dynamic-island-pod-right")).toBeInTheDocument();
    expect(container.querySelector(".indicator-dynamic-island-gap")).toBeInTheDocument();
    expect(container.querySelector(".indicator-dynamic-island-tray")).toBeInTheDocument();
    expect(container.querySelector(".indicator-live-viewport-lines-three")).toBeInTheDocument();
  });

  it("omits the timer pod when the timer is disabled", () => {
    const { container } = render(
      <TranscriptionPill
        phase="recording"
        title="Listening"
        detail="Capturing"
        levels={Array(12).fill(0.2)}
        overlayPosition="dynamic-island"
        animationStyle="waveform"
        showRecordingTimer={false}
        showLiveTranscription={false}
        dynamicIslandMetrics={dynamicIslandMetrics}
        onCancel={() => {}}
      />,
    );

    expect(container.querySelector(".indicator-dynamic-island-pod-right")).toBeNull();
    expect(container.querySelector(".indicator-dynamic-island-row-no-timer")).toBeInTheDocument();
  });
});

describe("InterfacePreviewCard", () => {
  it("ignores width preset classes in Dynamic Island preview mode", () => {
    const { container } = render(
      <InterfacePreviewCard
        overlayPosition="dynamic-island"
        animationStyle="spectrum"
        showRecordingTimer
        showLiveTranscription
        liveTranscriptWidth="wide"
        liveTranscriptLines="two"
      />,
    );

    const screen = container.querySelector(".interface-demo-screen");
    expect(screen).toHaveClass("interface-demo-screen-dynamic-island");
    expect(screen).not.toHaveClass("interface-demo-screen-width-wide");
    expect(container.querySelector(".indicator-shell-dynamic-island")).toBeInTheDocument();
  });
});

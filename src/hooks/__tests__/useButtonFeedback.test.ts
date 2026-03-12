import { vi } from "vitest";
import { renderHook, act } from "@testing-library/react";

import { useButtonFeedback } from "../../hooks/useButtonFeedback";

describe("useButtonFeedback", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns empty feedback initially", () => {
    const { result } = renderHook(() => useButtonFeedback());
    expect(result.current.buttonFeedback).toEqual({});
  });

  it("setButtonFeedbackState sets a working state", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.setButtonFeedbackState("copy", "working");
    });

    expect(result.current.buttonFeedback).toEqual({ copy: "working" });
  });

  it("setButtonFeedbackState overwrites an existing state", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.setButtonFeedbackState("copy", "working");
    });

    act(() => {
      result.current.setButtonFeedbackState("copy", "done");
    });

    expect(result.current.buttonFeedback).toEqual({ copy: "done" });
  });

  it("finishButtonFeedback transitions to done then clears after timeout", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.setButtonFeedbackState("save", "working");
    });

    act(() => {
      result.current.finishButtonFeedback("save");
    });

    expect(result.current.buttonFeedback).toEqual({ save: "done" });

    act(() => {
      vi.advanceTimersByTime(1200);
    });

    expect(result.current.buttonFeedback).toEqual({});
  });

  it("finishButtonFeedback respects custom holdMs", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.finishButtonFeedback("save", 500);
    });

    expect(result.current.buttonFeedback).toEqual({ save: "done" });

    act(() => {
      vi.advanceTimersByTime(499);
    });

    expect(result.current.buttonFeedback).toEqual({ save: "done" });

    act(() => {
      vi.advanceTimersByTime(1);
    });

    expect(result.current.buttonFeedback).toEqual({});
  });

  it("clearButtonFeedback removes a state", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.setButtonFeedbackState("download", "working");
    });

    act(() => {
      result.current.clearButtonFeedback("download");
    });

    expect(result.current.buttonFeedback).toEqual({});
  });

  it("tracks multiple concurrent action IDs independently", () => {
    const { result } = renderHook(() => useButtonFeedback());

    act(() => {
      result.current.setButtonFeedbackState("copy", "working");
      result.current.setButtonFeedbackState("save", "done");
    });

    expect(result.current.buttonFeedback).toEqual({
      copy: "working",
      save: "done",
    });

    act(() => {
      result.current.clearButtonFeedback("copy");
    });

    expect(result.current.buttonFeedback).toEqual({ save: "done" });
  });
});

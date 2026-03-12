import { vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("../../components/icons", () => ({
  BoltIcon: () => null,
  CheckIcon: () => null,
  ChevronDownIcon: () => null,
  CloseIcon: ({ className }: { className?: string }) => (
    <span className={className} data-testid="close-icon" />
  ),
  SectionIcon: () => null,
}));

import { ActionButton, StatusChip, NoticeBanner } from "../../components/common";

describe("ActionButton", () => {
  it("renders idle label", () => {
    render(<ActionButton idleLabel="Copy" onClick={() => {}} />);
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "Copy");
    expect(button).toHaveTextContent("Copy");
  });

  it("shows working label when state is working", () => {
    render(
      <ActionButton
        idleLabel="Copy"
        workingLabel="Copying..."
        state="working"
        onClick={() => {}}
      />,
    );
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "Copying...");
    expect(button).toHaveTextContent("Copying...");
  });

  it("shows done label when state is done", () => {
    render(
      <ActionButton
        idleLabel="Copy"
        doneLabel="Copied!"
        state="done"
        onClick={() => {}}
      />,
    );
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "Copied!");
    expect(button).toHaveTextContent("Copied!");
  });

  it("falls back to idle label when working/done labels are not provided", () => {
    render(<ActionButton idleLabel="Copy" state="working" onClick={() => {}} />);
    expect(screen.getByRole("button")).toHaveAttribute("aria-label", "Copy");
  });

  it("calls onClick when clicked", async () => {
    const handleClick = vi.fn();
    render(<ActionButton idleLabel="Copy" onClick={handleClick} />);

    await userEvent.click(screen.getByRole("button"));

    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("is disabled when disabled prop is true", async () => {
    const handleClick = vi.fn();
    render(<ActionButton idleLabel="Copy" onClick={handleClick} disabled />);

    const button = screen.getByRole("button");
    expect(button).toBeDisabled();

    await userEvent.click(button);
    expect(handleClick).not.toHaveBeenCalled();
  });

  it("hides label text when iconOnly is true", () => {
    render(<ActionButton idleLabel="Copy" onClick={() => {}} iconOnly />);
    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("aria-label", "Copy");
    expect(button).not.toHaveTextContent("Copy");
  });
});

describe("StatusChip", () => {
  it("renders label text", () => {
    render(<StatusChip label="Active" />);
    expect(screen.getByText("Active")).toBeInTheDocument();
  });

  it("applies tone class", () => {
    const { container } = render(<StatusChip label="OK" tone="success" />);
    const chip = container.querySelector(".status-chip");
    expect(chip).toHaveClass("status-chip-success");
  });

  it("defaults to muted tone", () => {
    const { container } = render(<StatusChip label="Idle" />);
    const chip = container.querySelector(".status-chip");
    expect(chip).toHaveClass("status-chip-muted");
  });
});

describe("NoticeBanner", () => {
  it("renders error text", () => {
    render(<NoticeBanner kind="error" text="Something went wrong" onDismiss={() => {}} />);
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("applies error kind class", () => {
    const { container } = render(
      <NoticeBanner kind="error" text="Oops" onDismiss={() => {}} />,
    );
    const notice = container.querySelector(".notice");
    expect(notice).toHaveClass("notice-error");
  });

  it("calls onDismiss when dismiss button is clicked", async () => {
    const handleDismiss = vi.fn();
    render(
      <NoticeBanner kind="error" text="Error occurred" onDismiss={handleDismiss} />,
    );

    await userEvent.click(screen.getByRole("button", { name: "Dismiss message" }));

    expect(handleDismiss).toHaveBeenCalledTimes(1);
  });
});

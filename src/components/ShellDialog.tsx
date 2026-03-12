import { useEffect } from "react";
import type { ReactNode } from "react";

import { CloseIcon } from "./icons";

export function ShellDialog({
  title,
  description,
  size = "large",
  fullHeight = false,
  onClose,
  children,
}: {
  title: string;
  description?: string;
  size?: "medium" | "large" | "wide";
  fullHeight?: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    function handleEscape(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }

      if (
        event.target instanceof HTMLElement &&
        (event.target.closest(".choice-dropdown-open") ||
          event.target.closest(".shortcut-button-armed") ||
          ["INPUT", "TEXTAREA"].includes(event.target.tagName) ||
          event.target.isContentEditable)
      ) {
        return;
      }

      if (event.key === "Escape") {
        onClose();
      }
    }

    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose]);

  return (
    <div
      className="shell-dialog-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        className={`shell-dialog shell-dialog-${size} ${fullHeight ? "shell-dialog-full-height" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <header className="shell-dialog-header">
          <div className="shell-dialog-copy">
            <h2>{title}</h2>
            {description ? <p>{description}</p> : null}
          </div>
          <button
            type="button"
            className="shell-dialog-close"
            onClick={onClose}
            aria-label={`Close ${title}`}
            title="Close"
          >
            <CloseIcon className="small-icon" />
          </button>
        </header>

        <div className="shell-dialog-body">{children}</div>
      </section>
    </div>
  );
}

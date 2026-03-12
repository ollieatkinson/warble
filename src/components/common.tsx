import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import type {
  ButtonFeedbackState,
  ChoiceOption,
  SectionId,
  StatusTone,
} from "../types";
import { captureShortcut } from "../lib/utils";
import {
  BoltIcon,
  CheckIcon,
  ChevronDownIcon,
  CloseIcon,
  SectionIcon,
} from "./icons";

export function ActionButton({
  className,
  state,
  idleLabel,
  workingLabel,
  doneLabel,
  idleIcon,
  workingIcon,
  doneIcon,
  onClick,
  disabled,
  iconOnly,
}: {
  className?: string;
  state?: ButtonFeedbackState;
  idleLabel: string;
  workingLabel?: string;
  doneLabel?: string;
  idleIcon?: ReactNode;
  workingIcon?: ReactNode;
  doneIcon?: ReactNode;
  onClick: () => void | Promise<void>;
  disabled?: boolean;
  iconOnly?: boolean;
}) {
  const label =
    state === "working"
      ? (workingLabel ?? idleLabel)
      : state === "done"
        ? (doneLabel ?? idleLabel)
        : idleLabel;
  const icon =
    state === "done"
      ? (doneIcon ?? idleIcon)
      : state === "working"
        ? (workingIcon ?? idleIcon)
        : idleIcon;

  return (
    <button
      className={[
        className,
        iconOnly ? "icon-only-button" : icon ? "icon-button" : "",
        state === "working" ? "action-button-working" : "",
        state === "done" ? "action-button-done" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
    >
      {icon ? (
        <span
          className={[
            "action-button-icon",
            state === "working" ? "action-button-icon-spin" : "",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          {icon}
        </span>
      ) : null}
      {iconOnly ? null : <span>{label}</span>}
    </button>
  );
}

export function ProgressRing({
  progress,
  size = 18,
  strokeWidth = 2,
  children,
}: {
  progress?: number | null;
  size?: number;
  strokeWidth?: number;
  children?: ReactNode;
}) {
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const normalized = progress == null ? null : Math.max(0, Math.min(1, progress));
  const dashOffset =
    normalized == null ? circumference * 0.68 : circumference * (1 - normalized);

  return (
    <span
      className={[
        "progress-ring",
        normalized == null ? "progress-ring-indeterminate" : "",
      ]
        .filter(Boolean)
        .join(" ")}
      style={{ width: size, height: size }}
      aria-hidden="true"
    >
      <svg viewBox={`0 0 ${size} ${size}`} className="progress-ring-svg">
        <circle
          className="progress-ring-track"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
        />
        <circle
          className="progress-ring-meter"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          strokeWidth={strokeWidth}
          strokeDasharray={`${circumference} ${circumference}`}
          strokeDashoffset={dashOffset}
        />
      </svg>
      <span className="progress-ring-content">{children}</span>
    </span>
  );
}

export function ShortcutField({
  label,
  value,
  armed,
  onArm,
  onCapture,
  onCancel,
}: {
  label: string;
  value: string;
  armed: boolean;
  onArm: () => void;
  onCapture: (value: string) => void;
  onCancel: () => void;
}) {
  const buttonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!armed) {
      return;
    }

    buttonRef.current?.focus();

    function handleKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      const captured = captureShortcut(event);
      if (captured) {
        onCapture(captured);
      }
    }

    function handlePointerDown(event: PointerEvent) {
      if (
        buttonRef.current &&
        event.target instanceof Node &&
        !buttonRef.current.contains(event.target)
      ) {
        onCancel();
      }
    }

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("pointerdown", handlePointerDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [armed, onCancel, onCapture]);

  return (
    <label className="field">
      <span>{label}</span>
      <button
        ref={buttonRef}
        type="button"
        className={`shortcut-button ${armed ? "shortcut-button-armed" : ""}`}
        onClick={onArm}
        onKeyDown={(event) => {
          if (!armed) {
            return;
          }

          event.preventDefault();
          const captured = captureShortcut(event);
          if (captured) {
            onCapture(captured);
          }
        }}
        onBlur={onCancel}
      >
        {armed ? "Press shortcut..." : value}
      </button>
    </label>
  );
}

export function ChoiceDropdown({
  label,
  value,
  options,
  onChange,
  placeholder = "Select",
  renderPreview,
}: {
  label: string;
  value: string;
  options: ChoiceOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  renderPreview?: (value: string, mode: "trigger" | "option") => ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const selected = options.find((option) => option.id === value) ?? options[0] ?? null;

  useEffect(() => {
    if (!open) {
      return;
    }

    function handlePointerDown(event: MouseEvent) {
      if (
        rootRef.current &&
        event.target instanceof Node &&
        !rootRef.current.contains(event.target)
      ) {
        setOpen(false);
      }
    }

    function handleEscape(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        setOpen(false);
      }
    }

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleEscape);

    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open]);

  return (
    <div className="field">
      <span>{label}</span>
      <div
        ref={rootRef}
        className={`choice-dropdown ${open ? "choice-dropdown-open" : ""}`}
      >
        <button
          type="button"
          className="choice-trigger"
          onClick={() => setOpen((current) => !current)}
          aria-expanded={open}
          aria-haspopup="listbox"
          disabled={options.length === 0}
        >
          {selected && renderPreview ? (
            <span className="choice-preview">{renderPreview(selected.id, "trigger")}</span>
          ) : null}
          <div className="choice-trigger-copy">
            <strong>{selected?.label ?? placeholder}</strong>
          </div>
          <ChevronDownIcon className="choice-chevron" />
        </button>

        {open && options.length > 0 ? (
          <div className="choice-menu" role="listbox">
            {options.map((option) => (
              <button
                key={option.id}
                type="button"
                className={`choice-option ${value === option.id ? "choice-option-active" : ""}`}
                onClick={() => {
                  onChange(option.id);
                  setOpen(false);
                }}
              >
                <div className="choice-option-main">
                  {renderPreview ? (
                    <span className="choice-preview choice-preview-option">
                      {renderPreview(option.id, "option")}
                    </span>
                  ) : null}
                  <div className="choice-option-copy">
                    <strong>{option.label}</strong>
                    {option.description ? <span>{option.description}</span> : null}
                  </div>
                </div>
                {value === option.id ? <CheckIcon className="choice-check" /> : null}
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function SidebarButton({
  active,
  label,
  section,
  collapsed,
  onClick,
}: {
  active: boolean;
  label: string;
  section: SectionId;
  collapsed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`sidebar-button ${active ? "sidebar-button-active" : ""}`}
      onClick={onClick}
      aria-label={label}
      title={label}
    >
      <SectionIcon section={section} className="sidebar-icon" />
      {collapsed ? null : <span>{label}</span>}
    </button>
  );
}

export function StatusChip({
  label,
  tone = "muted",
  icon,
}: {
  label: string;
  tone?: StatusTone;
  icon?: ReactNode;
}) {
  return (
    <span className={`status-chip status-chip-${tone}`}>
      {icon ? <span className="status-chip-icon">{icon}</span> : null}
      {label}
    </span>
  );
}

export function SidebarToggleIcon({
  collapsed,
  className,
}: {
  collapsed: boolean;
  className?: string;
}) {
  return (
    <svg
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      className={className}
    >
      <rect x="3.5" y="4.5" width="13" height="11" rx="2.4" />
      {collapsed ? (
        <path
          d="M 10.75 7.2 L 13.4 10 L 10.75 12.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ) : (
        <path
          d="M 9.25 7.2 L 6.6 10 L 9.25 12.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      <path d="M 7.1 4.5 V 15.5" />
    </svg>
  );
}

export function ModelPickerPreview({
  active = false,
  selectable = false,
}: {
  active?: boolean;
  selectable?: boolean;
}) {
  return (
    <span
      className={[
        "model-picker-dot",
        active ? "model-picker-dot-active" : "",
        !active && selectable ? "model-picker-dot-ready" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    />
  );
}

export function ScoreMeter({
  value,
  kind,
}: {
  value: number;
  kind: "speed" | "accuracy";
}) {
  const rounded = Math.max(0, Math.min(5, Math.round(value)));

  return (
    <div className={`score-meter score-meter-${kind}`}>
      <div className="score-meter-icons" aria-hidden="true">
        {Array.from({ length: 5 }, (_, index) =>
          kind === "speed" ? (
            <BoltIcon
              key={`${kind}-${index}`}
              className={`score-bolt ${index < rounded ? "score-bolt-on" : ""}`}
            />
          ) : (
            <span
              key={`${kind}-${index}`}
              className={`score-dot ${index < rounded ? "score-dot-on" : ""}`}
            />
          ),
        )}
      </div>
      <span>{value.toFixed(1)}</span>
    </div>
  );
}

export function ModelFeatureBadge({
  icon,
  label,
}: {
  icon: ReactNode;
  label: string;
}) {
  return (
    <span className="model-feature-badge" title={label} aria-label={label}>
      {icon}
    </span>
  );
}

export function StatTile({
  icon,
  label,
  value,
  tone,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  tone: StatusTone;
}) {
  return (
    <article className={`tile tile-${tone}`}>
      <div className="tile-icon">{icon}</div>
      <div className="tile-copy">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </article>
  );
}

export function NoticeBanner({
  kind,
  text,
  onDismiss,
}: {
  kind: "error";
  text: string;
  onDismiss: () => void;
}) {
  return (
    <div className={`notice notice-${kind}`}>
      <div className="notice-copy">{text}</div>
      <button
        type="button"
        className="notice-dismiss"
        onClick={onDismiss}
        aria-label="Dismiss message"
        title="Dismiss"
      >
        <CloseIcon className="small-icon" />
      </button>
    </div>
  );
}

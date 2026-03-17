import type { ReactNode, SVGProps } from "react";

import type { SectionId } from "../types";

type IconProps = SVGProps<SVGSVGElement>;

function GlyphBase({
  children,
  className,
  ...props
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
      {...props}
    >
      {children}
    </svg>
  );
}

export function OverviewIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.75" y="4.75" width="5.25" height="5.25" rx="1.35" />
      <rect x="14" y="4.75" width="5.25" height="5.25" rx="1.35" />
      <rect x="4.75" y="14" width="5.25" height="5.25" rx="1.35" />
      <rect x="14" y="14" width="5.25" height="5.25" rx="1.35" />
    </GlyphBase>
  );
}

export function CaptureIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5a3 3 0 0 1 3 3v4a3 3 0 0 1-6 0v-4a3 3 0 0 1 3-3Z" />
      <path d="M7.5 10.5a4.5 4.5 0 1 0 9 0" />
      <path d="M12 15v4.5" />
      <path d="M8 19.5h8" />
    </GlyphBase>
  );
}

export function ModelsIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M6 8.5 12 5l6 3.5-6 3.5L6 8.5Z" />
      <path d="M6 12.5 12 16l6-3.5" />
      <path d="M6 16.5 12 20l6-3.5" />
    </GlyphBase>
  );
}

export function KeysIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="6.5" width="15" height="11" rx="3" />
      <path d="M8 10.5h2" />
      <path d="M12 10.5h4" />
      <path d="M8 14.5h8" />
    </GlyphBase>
  );
}

export function InputIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5a3 3 0 0 1 3 3v4a3 3 0 0 1-6 0v-4a3 3 0 0 1 3-3Z" />
      <path d="M7.5 10.5a4.5 4.5 0 1 0 9 0" />
      <path d="M12 15v4.5" />
    </GlyphBase>
  );
}

export function CleanupIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M5 18.5h11.5" />
      <path d="M8 18.5 15.5 5" />
      <path d="M12 18.5 18.5 9" />
      <path d="M14.5 5h4" />
    </GlyphBase>
  );
}

export function InterfaceIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="4.5" y="5.5" width="15" height="13" rx="3" />
      <path d="M7.5 9h9" />
      <rect x="7" y="12" width="10" height="3.5" rx="1.75" />
    </GlyphBase>
  );
}

export function DebugIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="5" y="6" width="14" height="12" rx="2.5" />
      <path d="M9 10h6" />
      <path d="M9 14h4" />
      <path d="M12 3.5v2" />
      <path d="M7 4.5 8.2 6" />
      <path d="M17 4.5 15.8 6" />
    </GlyphBase>
  );
}

export function HistoryIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 12a7.5 7.5 0 1 0 2.2-5.3" />
      <path d="M4.5 5.5v4h4" />
      <path d="M12 8.5v4l2.5 1.5" />
    </GlyphBase>
  );
}

export function AboutIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 10v5" />
      <path d="M12 7.5h.01" />
    </GlyphBase>
  );
}

export function SparkIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 3.5 13.7 8l4.8 1.7L13.7 11.4 12 16l-1.7-4.6L5.5 9.7 10.3 8 12 3.5Z" />
    </GlyphBase>
  );
}

export function WarbleIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M5 12c1.5-2.8 3-2.8 4.5 0s3 2.8 4.5 0 3-2.8 4.5 0" />
      <path d="M6.2 8.5c1.1-2 2.2-2 3.3 0s2.2 2 3.3 0 2.2-2 3.3 0" />
      <path d="M6.2 15.5c1.1-2 2.2-2 3.3 0s2.2 2 3.3 0 2.2-2 3.3 0" />
    </GlyphBase>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m6.5 12 3.5 3.5 7-7" />
    </GlyphBase>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m7 7 10 10" />
      <path d="m17 7-10 10" />
    </GlyphBase>
  );
}

export function ChevronDownIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="m6 9 6 6 6-6" />
    </GlyphBase>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="11" cy="11" r="5.5" />
      <path d="m16 16 3.5 3.5" />
    </GlyphBase>
  );
}

export function RefreshIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M19.5 11.5A7.5 7.5 0 0 0 6.7 6.2" />
      <path d="M6.5 4.5v3.8h3.8" />
      <path d="M4.5 12.5a7.5 7.5 0 0 0 12.8 5.3" />
      <path d="M17.5 19.5v-3.8h-3.8" />
    </GlyphBase>
  );
}

export function DownloadIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M12 4.5v10" />
      <path d="m8.5 11 3.5 3.5 3.5-3.5" />
      <path d="M5 18.5h14" />
    </GlyphBase>
  );
}

export function BoltIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M13.5 3.5 7.5 13h4l-1 7.5 6-9h-4l1-8Z" />
    </GlyphBase>
  );
}

export function UsersIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M9 11a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z" />
      <path d="M15.75 10a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z" />
      <path d="M4.75 17.5a4.25 4.25 0 0 1 8.5 0" />
      <path d="M13.25 17.5a3.5 3.5 0 0 1 6 0" />
    </GlyphBase>
  );
}

export function ClockIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M12 7.5v5l3 1.8" />
    </GlyphBase>
  );
}

export function GlobeIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M4.5 12h15" />
      <path d="M12 4.5a12 12 0 0 1 0 15" />
      <path d="M12 4.5a12 12 0 0 0 0 15" />
    </GlyphBase>
  );
}

export function CpuIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="7.5" y="7.5" width="9" height="9" rx="2" />
      <path d="M9.5 2.5v3" />
      <path d="M14.5 2.5v3" />
      <path d="M9.5 18.5v3" />
      <path d="M14.5 18.5v3" />
      <path d="M2.5 9.5h3" />
      <path d="M2.5 14.5h3" />
      <path d="M18.5 9.5h3" />
      <path d="M18.5 14.5h3" />
    </GlyphBase>
  );
}

export function FolderIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M4.5 8.5h5l1.5 2h8.5v7a2 2 0 0 1-2 2h-11a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2Z" />
    </GlyphBase>
  );
}

export function CopyIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <rect x="8" y="8" width="10" height="10" rx="2" />
      <path d="M6.5 15.5h-1A2.5 2.5 0 0 1 3 13V5.5A2.5 2.5 0 0 1 5.5 3H13a2.5 2.5 0 0 1 2.5 2.5v1" />
    </GlyphBase>
  );
}

export function ExternalIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M14 5h5v5" />
      <path d="m10 14 9-9" />
      <path d="M19 13v4a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h4" />
    </GlyphBase>
  );
}

export function TrashIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <path d="M5.5 7.5h13" />
      <path d="M9 7.5V5.8A1.8 1.8 0 0 1 10.8 4h2.4A1.8 1.8 0 0 1 15 5.8v1.7" />
      <path d="M7.5 7.5 8.2 18a2 2 0 0 0 2 1.9h3.6a2 2 0 0 0 2-1.9l.7-10.5" />
      <path d="M10 11v5" />
      <path d="M14 11v5" />
    </GlyphBase>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="2.6" />
      <path d="M12 3.8v2.1" />
      <path d="M12 18.1v2.1" />
      <path d="m5.3 5.3 1.5 1.5" />
      <path d="m17.2 17.2 1.5 1.5" />
      <path d="M3.8 12h2.1" />
      <path d="M18.1 12h2.1" />
      <path d="m5.3 18.7 1.5-1.5" />
      <path d="m17.2 6.8 1.5-1.5" />
    </GlyphBase>
  );
}

export function HelpIcon(props: IconProps) {
  return (
    <GlyphBase {...props}>
      <circle cx="12" cy="12" r="8" />
      <path d="M9.7 9.2a2.6 2.6 0 1 1 4.6 1.7c-.5.6-1 .9-1.6 1.3-.7.4-1.2 1-1.2 2" />
      <path d="M12 17.1h.01" />
    </GlyphBase>
  );
}

export function SectionIcon({
  section,
  className,
}: {
  section: SectionId;
  className?: string;
}) {
  switch (section) {
    case "capture":
      return <CaptureIcon className={className} />;
    case "models":
      return <ModelsIcon className={className} />;
    case "vocabulary":
      return <CleanupIcon className={className} />;
    case "history":
      return <HistoryIcon className={className} />;
    case "settings":
      return <SettingsIcon className={className} />;
    case "help":
      return <HelpIcon className={className} />;
    default:
      return <CaptureIcon className={className} />;
  }
}

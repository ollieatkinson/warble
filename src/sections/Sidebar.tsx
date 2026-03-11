import { sections } from "../constants";
import { formatPhaseLabel, toneForPhase } from "../lib/utils";
import type { AppPhase, SectionId } from "../types";
import { WarbleIcon } from "../components/icons";
import { SidebarButton, SidebarToggleIcon, StatusChip } from "../components/common";

export function Sidebar({
  activeSection,
  collapsed,
  phase,
  onSelect,
  onToggleCollapsed,
}: {
  activeSection: SectionId;
  collapsed: boolean;
  phase: AppPhase;
  onSelect: (section: SectionId) => void;
  onToggleCollapsed: () => void;
}) {
  return (
    <aside className={`sidebar ${collapsed ? "sidebar-collapsed" : ""}`}>
      <div className="sidebar-brand">
        <div className="sidebar-brand-mark">
          <WarbleIcon className="brand-icon" />
        </div>
        {collapsed ? null : (
          <div className="sidebar-brand-copy">
            <strong>Warble</strong>
            <span>Local dictation</span>
          </div>
        )}
        <button
          type="button"
          className="sidebar-toggle"
          onClick={onToggleCollapsed}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <SidebarToggleIcon collapsed={collapsed} className="sidebar-toggle-icon" />
        </button>
      </div>

      <nav className="sidebar-nav">
        {sections.map((section) => (
          <SidebarButton
            key={section.id}
            active={activeSection === section.id}
            label={section.label}
            section={section.id}
            collapsed={collapsed}
            onClick={() => onSelect(section.id)}
          />
        ))}
      </nav>

      {collapsed ? null : (
        <div className="sidebar-footer">
          <StatusChip label={formatPhaseLabel(phase)} tone={toneForPhase(phase)} />
          <StatusChip label="Local" tone="accent" />
        </div>
      )}
    </aside>
  );
}

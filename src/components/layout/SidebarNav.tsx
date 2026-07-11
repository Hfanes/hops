import {
  FiChevronLeft,
  FiChevronRight,
  FiMoon,
  FiSun,
} from "react-icons/fi";
import { NAV_ITEMS } from "../../lib/navigation";
import type { TabId, ThemePreference } from "../../types";

export function SidebarNav({
  activeTab,
  isCollapsed,
  theme,
  className,
  onTabChange,
  onThemeToggle,
  onCollapseToggle,
}: {
  activeTab: TabId;
  isCollapsed: boolean;
  theme: ThemePreference;
  className: string;
  onTabChange: (tab: TabId) => void;
  onThemeToggle: () => void;
  onCollapseToggle: () => void;
}) {
  return (
    <aside className={className} aria-label="Primary">
      <div className="sidebar-brand">
        <span className="sidebar-brand-mark">
          <img
            className="sidebar-brand-logo"
            src="/hops.webp"
            alt=""
            aria-hidden="true"
          />
        </span>
        <span
          className={`sidebar-brand-name ${
            isCollapsed ? "sr-only" : ""
          }`}
        >
          Hops
        </span>
      </div>
      <nav className="sidebar-nav-list">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              type="button"
              className={`sidebar-nav-item ${activeTab === item.id ? "active" : ""} ${
                isCollapsed ? "icon-only" : ""
              }`}
              onClick={() => onTabChange(item.id)}
              title={item.label}
            >
              <Icon aria-hidden="true" />
              <span className={isCollapsed ? "sr-only" : ""}>
                {item.label}
              </span>
            </button>
          );
        })}
      </nav>
      <div className="sidebar-footer-actions">
        <button
          type="button"
          className={`sidebar-toggle secondary ${isCollapsed ? "icon-only" : ""}`}
          onClick={onThemeToggle}
          aria-label={
            theme === "light" ? "Switch to dark theme" : "Switch to light theme"
          }
          title={
            theme === "light" ? "Switch to dark theme" : "Switch to light theme"
          }
        >
          {theme === "light" ? (
            <FiMoon aria-hidden="true" />
          ) : (
            <FiSun aria-hidden="true" />
          )}
          <span className={isCollapsed ? "sr-only" : ""}>
            {theme === "light" ? "Dark" : "Light"}
          </span>
        </button>
        <button
          type="button"
          className={`sidebar-toggle secondary ${isCollapsed ? "icon-only" : ""}`}
          onClick={onCollapseToggle}
          aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          title="Toggle sidebar (Ctrl+B)"
        >
          {isCollapsed ? (
            <FiChevronRight aria-hidden="true" />
          ) : (
            <FiChevronLeft aria-hidden="true" />
          )}
          <span className={isCollapsed ? "sr-only" : ""}>Collapse</span>
        </button>
      </div>
    </aside>
  );
}

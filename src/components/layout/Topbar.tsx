import type { ReactNode } from "react";
import { TAB_TITLES } from "../../lib/navigation";
import type { TabId } from "../../types";

export function Topbar({
  activeTab,
  actions,
}: {
  activeTab: TabId;
  actions: ReactNode;
}) {
  return (
    <header className="topbar">
      <div>
        <p className="section-label">Hops Control Center</p>
        <h1>{TAB_TITLES[activeTab].title}</h1>
        <p>{TAB_TITLES[activeTab].subtitle}</p>
      </div>
      <div className="actions">{actions}</div>
    </header>
  );
}

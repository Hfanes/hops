import type { ReactNode, RefObject } from "react";

/** The settings window after onboarding: the sidebar and the active tab. */
export function MainScreen({
  isSidebarCollapsed,
  sidebar,
  contentAreaRef,
  footer,
  children,
}: {
  isSidebarCollapsed: boolean;
  sidebar: ReactNode;
  contentAreaRef: RefObject<HTMLDivElement | null>;
  footer: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="app-shell">
      <section
        className={`app-panel ${isSidebarCollapsed ? "sidebar-collapsed" : ""}`}
      >
        {sidebar}

        <div className="content-area" ref={contentAreaRef}>
          {children}
        </div>
      </section>
      {footer}
    </main>
  );
}

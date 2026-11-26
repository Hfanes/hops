import type { ReactNode, RefObject } from "react";

/** The settings window while onboarding runs: a setup header and the step. */
export function OnboardingScreen({
  step,
  contentAreaRef,
  footer,
  children,
}: {
  step: number;
  contentAreaRef: RefObject<HTMLDivElement | null>;
  footer: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="app-shell">
      <section className="onboarding-panel">
        <div className="content-area" ref={contentAreaRef}>
          <header className="topbar">
            <div>
              <p className="section-label">Hops Setup</p>
              <h1>Welcome to Hops</h1>
              <p>Quick onboarding to make external links route correctly.</p>
            </div>
            <p className="badge">Step {step + 1} of 4</p>
          </header>

          {children}
        </div>
      </section>
      {footer}
    </main>
  );
}

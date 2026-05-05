import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  message: string;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = {
    hasError: false,
    message: "",
  };

  static getDerivedStateFromError(error: Error): State {
    return {
      hasError: true,
      message: error.message,
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Keep a useful trace in DevTools while showing an in-app fallback.
    console.error("Hops UI crashed:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return (
        <main className="shell">
          <section className="panel">
            <h1>Hops encountered a UI error</h1>
            <p>{this.state.message || "Unknown UI error."}</p>
            <p>Reload the app window after saving your terminal logs.</p>
          </section>
        </main>
      );
    }

    return this.props.children;
  }
}

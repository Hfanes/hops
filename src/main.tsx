import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { ErrorBoundary } from "./ErrorBoundary";
import PickerWindow from "./features/picker/PickerWindow";

function resolveRootComponent() {
  try {
    return getCurrentWindow().label === "picker" ? PickerWindow : App;
  } catch {
    return App;
  }
}

const RootComponent = resolveRootComponent();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <RootComponent />
    </ErrorBoundary>
  </React.StrictMode>,
);

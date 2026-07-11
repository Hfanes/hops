import React from "react";
import ReactDOM from "react-dom/client";
import { ErrorBoundary } from "./ErrorBoundary";
import PickerWindow from "./features/picker/PickerWindow";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <PickerWindow />
    </ErrorBoundary>
  </React.StrictMode>,
);

import { useEffect } from "react";
import type { ThemePreference } from "../types";

export function useDocumentTheme(theme: ThemePreference) {
  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
  }, [theme]);
}

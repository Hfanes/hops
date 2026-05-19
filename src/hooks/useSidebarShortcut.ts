import { useEffect } from "react";

export function useSidebarShortcut(onToggle: () => void) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.ctrlKey &&
        (event.key.toLowerCase() === "b" || event.code === "KeyB")
      ) {
        event.preventDefault();
        event.stopPropagation();
        onToggle();
      }
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
    };
  }, [onToggle]);
}

import type { IconType } from "react-icons";
import {
  FiGlobe,
  FiInfo,
  FiList,
  FiNavigation,
  FiSettings,
} from "react-icons/fi";
import type { TabId } from "../types";

export const NAV_ITEMS: Array<{ id: TabId; label: string; icon: IconType }> = [
  { id: "settings", label: "Settings", icon: FiSettings },
  { id: "browsers", label: "Browsers", icon: FiGlobe },
  { id: "rules", label: "Rules", icon: FiList },
  { id: "router", label: "Route tester", icon: FiNavigation },
  { id: "about", label: "About", icon: FiInfo },
];

export const TAB_TITLES: Record<TabId, { title: string; subtitle: string }> = {
  settings: {
    title: "Settings",
    subtitle: "Routing defaults, picker behavior, and Windows registration.",
  },
  browsers: {
    title: "Browsers",
    subtitle: "Detected and manual browser entries used by Hops.",
  },
  rules: {
    title: "Rules",
    subtitle: "URL match rules evaluated from top to bottom.",
  },
  router: {
    title: "Route Tester",
    subtitle: "Preview routing decisions before opening a URL.",
  },
  about: {
    title: "About",
    subtitle: "Version, release details, and app updates.",
  },
};

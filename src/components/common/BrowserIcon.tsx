const KNOWN_BROWSER_ICON_KEYS = new Set([
  "arc",
  "brave",
  "chrome",
  "edge",
  "firefox",
  "floorp",
  "generic",
  "helium",
  "librewolf",
  "opera",
  "tor",
  "vivaldi",
  "zen",
]);

function normalizedIconKey(iconKey: string | null | undefined) {
  if (!iconKey || !KNOWN_BROWSER_ICON_KEYS.has(iconKey)) {
    return "generic";
  }

  return iconKey;
}

export function browserIconSrc(iconKey: string | null | undefined) {
  return `/browser-icons/${normalizedIconKey(iconKey)}.webp`;
}

export function BrowserIcon({
  iconKey,
  className,
}: {
  iconKey: string | null | undefined;
  className?: string;
}) {
  return (
    <img
      className={className ?? "browser-icon"}
      src={browserIconSrc(iconKey)}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}

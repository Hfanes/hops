import { FiArrowUp } from "react-icons/fi";

export function ScrollTopButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      className="floating-top-button"
      onClick={onClick}
      aria-label="Scroll to top"
      title="Top"
    >
      <FiArrowUp aria-hidden="true" />
    </button>
  );
}

import { useState } from "react";

/** `values` plus `id`, or `values` itself when `id` is already there. */
export function withAdded<T>(values: Set<T>, id: T): Set<T> {
  if (values.has(id)) {
    return values;
  }
  const next = new Set(values);
  next.add(id);
  return next;
}

/** `values` without `id`, or `values` itself when `id` is not there. */
export function withRemoved<T>(values: Set<T>, id: T): Set<T> {
  if (!values.has(id)) {
    return values;
  }
  const next = new Set(values);
  next.delete(id);
  return next;
}

/**
 * A set of ids kept in state, such as the rules with unsaved edits.
 *
 * Updates are functional, so `add`, `remove` and `clear` are safe to call
 * from timers and async callbacks that outlive the render they came from.
 * An update that changes nothing keeps the same Set, so it does not re-render.
 */
export function useIdSet() {
  const [ids, setIds] = useState<Set<string>>(() => new Set());

  return {
    ids,
    add: (id: string) => setIds((current) => withAdded(current, id)),
    remove: (id: string) => setIds((current) => withRemoved(current, id)),
    clear: () =>
      setIds((current) => (current.size === 0 ? current : new Set())),
  };
}

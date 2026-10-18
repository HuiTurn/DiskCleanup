/** Reuse completed scans briefly and share pending requests across page mounts. */
export function createScanCache<T>(ttlMs = 60_000, maxEntries = 8) {
  const entries = new Map<string, { promise: Promise<T>; expires: number }>();
  return {
    get(key: string, load: () => Promise<T>, force = false): Promise<T> {
      const previous = entries.get(key);
      // Even a manual refresh must not launch a second copy of an active scan.
      if (previous && (previous.expires === Infinity || (!force && previous.expires > Date.now()))) {
        return previous.promise;
      }
      const entry = { promise: undefined as unknown as Promise<T>, expires: Infinity };
      entry.promise = Promise.resolve().then(load).then((value) => {
        entry.expires = Date.now() + ttlMs;
        return value;
      }, (error) => {
        if (entries.get(key) === entry) entries.delete(key);
        throw error;
      });
      entries.delete(key);
      entries.set(key, entry);
      // Pending jobs stay deduplicated until they finish; evict completed entries first.
      for (const [oldKey, old] of entries) {
        if (entries.size <= maxEntries) break;
        if (old.expires !== Infinity) entries.delete(oldKey);
      }
      return entry.promise;
    },
    clear() { entries.clear(); },
  };
}

import type { DuplicateGroup, FileEntry } from '../types';
export type KeeperChoices = Record<string, string[]>;
/** Untouched similar groups require a manual decision; exact groups keep the newest copy. */
export function keepersFor(group: DuplicateGroup, choices: KeeperChoices): Set<string> {
  const valid = new Set(group.files.map(f => f.path));
  const explicit = choices[group.id]?.filter(path => valid.has(path));
  const protectedFiles = group.files.filter(f => f.protectedReason).map(f => f.path);
  if (explicit?.length) return new Set([...explicit, ...protectedFiles]);
  if (group.kind === 'similar') return valid;
  const newest = [...group.files].sort((a, b) => b.modified.localeCompare(a.modified))[0];
  return new Set([...protectedFiles, ...(newest ? [newest.path] : [])]);
}
/** A retained path wins even if another overlapping group proposes deleting it. */
export function duplicateCleanup(groups: DuplicateGroup[], shown: DuplicateGroup[], choices: KeeperChoices): FileEntry[] {
  const protectedPaths = new Set(groups.flatMap(g => [...keepersFor(g, choices)]));
  const candidates = new Map<string, FileEntry>();
  for (const group of shown) for (const file of group.files) {
    if (!file.protectedReason && !protectedPaths.has(file.path)) candidates.set(file.path, file);
  }
  return [...candidates.values()];
}

export interface DiskInfo {
  name: string;
  mountPoint: string;
  totalBytes: number;
  usedBytes: number;
  isSystem: boolean;
}

export interface ScanStatus {
  running: boolean;
  percent: number;
  scannedBytes: number;
  scannedFiles: number;
  currentPath: string;
  recentFiles: string[];
  skippedEntries?: number;
  error?: string | null;
}

export interface CategoryStat {
  name: string;
  bytes: number;
  percent: number;
}

export interface FolderStat {
  name: string;
  path: string;
  sizeBytes: number;
  fileCount: number;
  percent: number;
}

export interface DirNameStat {
  name: string;
  sizeBytes: number;
  fileCount: number;
  occurrences: number;
  percent: number;
}

export interface BucketStat {
  label: string;
  count: number;
  bytes: number;
  percent: number;
}

export interface Snapshot {
  sizeBasis?: string;
  ts: number;
  root: string;
  totalBytes: number;
  totalFiles: number;
}

export interface CleanupRecord {
  ts: number;
  items: { path: string; sizeBytes: number }[];
}

export interface ScanResult {
  sizeBasis?: string;
  root: string;
  totalBytes: number;
  totalFiles: number;
  categories: CategoryStat[];
  topFolders: FolderStat[];
  dirNames: DirNameStat[];
  sizeBuckets: BucketStat[];
}

export interface JunkItem {
  id: string;
  name: string;
  path: string;
  description: string;
  sizeBytes: number;
  safe: boolean;
}

export interface JunkCategory {
  id: string;
  name: string;
  totalBytes: number;
  items: JunkItem[];
}

export interface FileEntry {
  protectedReason?: string | null;
  path: string;
  name: string;
  dir: string;
  sizeBytes: number;
  modified: string;
}

export interface DuplicateGroup {
  id: string;
  sizeBytes: number;
  category: string;
  kind: "exact" | "similar";
  similarity: number;
  files: FileEntry[];
}

export interface ToolInfo {
  id: string;
  name: string;
  description: string;
  action: string;
}

export interface LaunchItem {
  name: string;
}

export interface Settings {
  launchAtLogin: boolean;
  closeConfirm: boolean;
  language: string;
  autoCleanEnabled: boolean;
  autoCleanIntervalDays: number;
  lastAutoCleanTs: number | null;
  largeFileMinMb: number;
  excludeDirs: string[];
  theme: "light" | "dark" | "system";
}

export interface CleanSummary {
  freedBytes: number;
  movedBytes?: number;
  failures?: { path: string; reason: string }[];
  files: number;
  durationMs: number;
}

export type Page =
  | "home"
  | "analyze"
  | "results"
  | "junk"
  | "done"
  | "file-migration"
  | "large-files"
  | "duplicates"
  | "launch-items"
  | "system-tools"
  | "settings";

export interface MigrationFolder {
  id: string;
  name: string;
  path: string;
  blockedReason: string | null;
}
export interface MigrationDrive {
  path: string;
  availableBytes: number;
}
export interface MigrationOptions {
  supported: boolean;
  folders: MigrationFolder[];
  drives: MigrationDrive[];
}
export interface MigrationRequest {
  folderId: string;
  targetDrive: string;
  expectedSource: string;
}
export interface MigrationPlan {
  request: MigrationRequest;
  name: string;
  source: string;
  target: string;
  totalBytes: number;
  fileCount: number;
  availableBytes: number;
}
export interface MigrationStatus {
  phase: string;
  message: string;
  source: string;
  target: string;
  journal: string;
}

export interface RestoreSummary { restored: number; failures: { path: string; reason: string }[] }
export interface TaskStatus { running: boolean; visited: number; skipped: number; currentPath: string; cancelled: boolean; notice: string }

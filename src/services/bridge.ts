import { createScanCache } from "./scanCache";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type UnlistenFn } from "@tauri-apps/api/event";
import { isMockMode, mockInvoke, mockListen } from "./mock";
import type {
  MigrationOptions,
  RestoreSummary,
  TaskStatus,
  MigrationRequest,
  MigrationPlan,
  MigrationStatus,
  DiskInfo,
  ScanStatus,
  ScanResult,
  JunkCategory,
  FileEntry,
  DuplicateGroup,
  ToolInfo,
  LaunchItem,
  Settings,
  CleanSummary,
  Snapshot,
  CleanupRecord,
} from "../types";

// 包装 invoke / listen：非 Tauri 环境下走 mock，真实 Tauri 窗口走原生实现。
async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isMockMode) return mockInvoke(cmd, args) as T;
  return tauriInvoke<T>(cmd, args as never);
}

async function listen<T>(
  event: string,
  cb: (e: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (isMockMode) return mockListen();
  return tauriListen<T>(event, (e) => cb(e as never));
}

/** 监听「退出前确认」事件（后端拦截窗口关闭后推送） */
export function onCloseRequested(cb: () => void): Promise<UnlistenFn> {
  return listen("close-requested", () => cb());
}

const largeFileCache = createScanCache<FileEntry[]>();

// 统一的后端调用入口：UI 层不感知平台差异
export const bridge = {
  getTaskStatus: (kind: string) => invoke<TaskStatus>("get_task_status", { kind }),
  cancelTask: (kind: string) => invoke<void>("cancel_task", { kind }),
  getMigrationOptions: () => invoke<MigrationOptions>("get_migration_options"),
  prepareMigration: (request: MigrationRequest) =>
    invoke<MigrationPlan>("prepare_migration", { request }),
  migrateFolder: (request: MigrationRequest) =>
    invoke<MigrationStatus>("migrate_folder", { request }),
  getMigrationStatus: () => invoke<MigrationStatus>("get_migration_status"),
  getPlatform: () => invoke<string>("get_platform"),
  getDisks: () => invoke<DiskInfo[]>("get_disks"),
  startScan: (root?: string) =>
    invoke<void>("start_scan", { root: root ?? null }),
  cancelScan: () => invoke<void>("cancel_scan"),
  getScanStatus: () => invoke<ScanStatus>("get_scan_status"),
  getScanResult: () => invoke<ScanResult | null>("get_scan_result"),
  getJunkCategories: () => invoke<JunkCategory[]>("get_junk_categories"),
  cleanJunk: (paths: string[]) => invoke<void>("clean_junk", { paths }),
  getLargeFiles: (
    root: string | null,
    minMb: number,
    limit = 500,
    force = false,
  ) =>
    largeFileCache.get(
      JSON.stringify([root, minMb, limit]),
      () => invoke<FileEntry[]>("get_large_files", { root, minMb, limit }),
      force,
    ),
  findDuplicates: (root: string | null) =>
    invoke<DuplicateGroup[]>("find_duplicates", { root }),
  getSystemTools: () => invoke<ToolInfo[]>("get_system_tools"),
  runSystemTool: (id: string) =>
    invoke<string | null>("run_system_tool", { id }),
  getLaunchItems: () => invoke<LaunchItem[]>("get_launch_items"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: async (settings: Settings) => {
    await invoke<void>("save_settings", { settings });
    largeFileCache.clear();
  },
  getAutoCleanStatus: () => invoke<boolean>("get_auto_clean_status"),
  runAutoCleanNow: async () => {
    const freed = await invoke<CleanSummary>("run_auto_clean_now");
    largeFileCache.clear();
    return freed;
  },
  getScanHistory: () => invoke<Snapshot[]>("get_scan_history"),
  restoreLastCleanup: async () => {
    const restored = await invoke<RestoreSummary>("restore_last_cleanup");
    largeFileCache.clear();
    return restored;
  },
  pickDirectory: () => invoke<string | null>("pick_directory"),
  openTrash: () => invoke<void>("open_trash"),
  lastCleanupRecord: () => invoke<CleanupRecord | null>("last_cleanup_record"),
  closeWindow: () => invoke<void>("close_window"),
};

// 事件监听封装
export function onScanProgress(
  cb: (s: ScanStatus) => void,
): Promise<UnlistenFn> {
  return listen<ScanStatus>("scan-progress", (e) => cb(e.payload));
}
export function onScanFinished(
  cb: (r: ScanResult) => void,
): Promise<UnlistenFn> {
  return listen<ScanFinishedEvent>("scan-finished", (e) =>
    cb(e.payload.result),
  );
}
export function onCleanProgress(
  cb: (p: {
    done: number;
    total: number;
    path: string;
    freedBytes: number;
  }) => void,
): Promise<UnlistenFn> {
  return listen<{
    done: number;
    total: number;
    path: string;
    freedBytes: number;
  }>("clean-progress", (e) => cb(e.payload));
}
export function onCleanFinished(
  cb: (s: CleanSummary) => void,
): Promise<UnlistenFn> {
  return listen<{ summary: CleanSummary }>("clean-finished", (e) => {
    largeFileCache.clear();
    cb(e.payload.summary);
  });
}
interface ScanFinishedEvent {
  result: ScanResult;
}

export function onMigrationCloseBlocked(cb: () => void): Promise<UnlistenFn> {
  return listen("migration-close-blocked", cb);
}

// 前端最小 mock 层：当不在 Tauri 运行时（纯浏览器 / CI 截图）时，
// bridge 的 invoke / listen 走这里返回的静态数据，使所有页面可独立渲染。
// 真实 Tauri 环境下 __TAURI_INTERNALS__ 存在，自动跳过 mock，不影响任何功能。
import type {
  DiskInfo,
  ScanStatus,
  ScanResult,
  JunkCategory,
  DuplicateGroup,
  LaunchItem,
  ToolInfo,
  Snapshot,
  CleanupRecord,
  CleanSummary,
  Settings,
  MigrationOptions,
  MigrationPlan,
  MigrationStatus,
  FileEntry,
  RestoreSummary,
  TaskStatus,
} from "../types";

const params =
  typeof window !== "undefined"
    ? new URLSearchParams(window.location.search)
    : new URLSearchParams("");

/** 纯浏览器或显式 ?mock=1 时启用 mock；真实 Tauri 窗口不启用。 */
export const isMockMode =
  typeof window !== "undefined" &&
  (params.has("mock") || !("__TAURI_INTERNALS__" in window));

const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const mockDisks: DiskInfo[] = [
  {
    name: "Macintosh HD",
    mountPoint: "/",
    totalBytes: 494 * GB,
    usedBytes: 351 * GB,
    isSystem: true,
  },
  {
    name: "Data",
    mountPoint: "/Volumes/Data",
    totalBytes: 2000 * GB,
    usedBytes: 800 * GB,
    isSystem: false,
  },
];

export const mockSettings: Settings = {
  launchAtLogin: true,
  closeConfirm: true,
  language: "zh-CN",
  autoCleanEnabled: true,
  autoCleanIntervalDays: 7,
  lastAutoCleanTs: Date.now() - 2 * 86400000,
  largeFileMinMb: 100,
  excludeDirs: ["/System", "/Library/Caches"],
  theme: "light",
};

export const mockScanStatus: ScanStatus = {
  running: true,
  percent: 62,
  scannedBytes: 217 * GB,
  scannedFiles: 1_143_208,
  currentPath: "/Users/Shared/Music/iTunes Media",
  recentFiles: [
    "/Users/me/Pictures/2024/IMG_0421.heic",
    "/Users/me/Library/Caches/com.apple.Safari/fsCachedData/ABCD",
    "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain",
  ],
};

export const mockScanResult: ScanResult = {
  root: "/",
  totalBytes: 351 * GB,
  totalFiles: 1_843_205,
  categories: [
    { name: "应用程序", bytes: 96 * GB, percent: 27 },
    { name: "照片与视频", bytes: 78 * GB, percent: 22 },
    { name: "系统缓存", bytes: 41 * GB, percent: 12 },
    { name: "音乐与音频", bytes: 33 * GB, percent: 9 },
    { name: "文档", bytes: 52 * GB, percent: 15 },
    { name: "其他", bytes: 51 * GB, percent: 15 },
  ],
  topFolders: [
    { name: "Applications", path: "/Applications", sizeBytes: 96 * GB, fileCount: 184320, percent: 27 },
    { name: "Pictures", path: "/Users/me/Pictures", sizeBytes: 71 * GB, fileCount: 48210, percent: 20 },
    { name: "Library", path: "/Users/me/Library", sizeBytes: 58 * GB, fileCount: 612044, percent: 17 },
    { name: "Movies", path: "/Users/me/Movies", sizeBytes: 44 * GB, fileCount: 1208, percent: 13 },
    { name: "Downloads", path: "/Users/me/Downloads", sizeBytes: 31 * GB, fileCount: 4821, percent: 9 },
    { name: "Music", path: "/Users/me/Music", sizeBytes: 22 * GB, fileCount: 9304, percent: 6 },
  ],
  dirNames: [
    { name: "node_modules", sizeBytes: 38 * GB, fileCount: 2_104_330, occurrences: 184, percent: 11 },
    { name: "Caches", sizeBytes: 27 * GB, fileCount: 921_004, occurrences: 96, percent: 8 },
    { name: ".git", sizeBytes: 12 * GB, fileCount: 410_220, occurrences: 71, percent: 3 },
  ],
  sizeBuckets: [
    { label: "<1 MB", count: 1_204_330, bytes: 12 * GB, percent: 3 },
    { label: "1–10 MB", count: 302_104, bytes: 41 * GB, percent: 12 },
    { label: "10–100 MB", count: 88_210, bytes: 78 * GB, percent: 22 },
    { label: "100 MB–1 GB", count: 21_004, bytes: 96 * GB, percent: 27 },
    { label: ">1 GB", count: 4_330, bytes: 124 * GB, percent: 36 },
  ],
};

export const mockJunk: JunkCategory[] = [
  {
    id: "cache",
    name: "系统与应用缓存",
    totalBytes: 18 * GB,
    items: [
      { id: "c1", name: "Safari 缓存", path: "/Users/me/Library/Caches/com.apple.Safari", description: "浏览器临时文件", sizeBytes: 6 * GB, safe: true },
      { id: "c2", name: "Xcode 派生数据", path: "/Users/me/Library/Developer/Xcode/DerivedData", description: "编译中间产物", sizeBytes: 9 * GB, safe: true },
      { id: "c3", name: "日志文件", path: "/Users/me/Library/Logs", description: "历史运行日志", sizeBytes: 3 * GB, safe: false },
    ],
  },
  {
    id: "trash",
    name: "废纸篓",
    totalBytes: 7 * GB,
    items: [
      { id: "t1", name: "废纸篓内容", path: "~/.Trash", description: "已删除待清理", sizeBytes: 7 * GB, safe: true },
    ],
  },
  {
    id: "dev",
    name: "开发缓存",
    totalBytes: 22 * GB,
    items: [
      { id: "d1", name: "npm 缓存", path: "/Users/me/.npm/_cacache", description: "包管理器缓存", sizeBytes: 4 * GB, safe: true },
      { id: "d2", name: "CocoaPods 缓存", path: "/Users/me/Library/Caches/CocoaPods", description: "依赖缓存", sizeBytes: 5 * GB, safe: true },
      { id: "d3", name: "Docker 镜像", path: "/Users/me/Library/Containers/com.docker", description: "容器镜像层", sizeBytes: 13 * GB, safe: false },
    ],
  },
];

export const mockDupGroups: DuplicateGroup[] = [
  {
    id: "g1",
    sizeBytes: 4 * GB,
    category: "照片与视频",
    kind: "exact",
    similarity: 100,
    files: [
      { path: "/Users/me/Pictures/2024/IMG_0421.heic", name: "IMG_0421.heic", dir: "/Users/me/Pictures/2024", sizeBytes: 4 * GB, modified: "2024-03-12" },
      { path: "/Users/me/Pictures/备份/IMG_0421.heic", name: "IMG_0421.heic", dir: "/Users/me/Pictures/备份", sizeBytes: 4 * GB, modified: "2024-03-12" },
    ],
  },
  {
    id: "g2",
    sizeBytes: 1.2 * GB,
    category: "文档",
    kind: "similar",
    similarity: 88,
    files: [
      { path: "/Users/me/Documents/report-final.pdf", name: "report-final.pdf", dir: "/Users/me/Documents", sizeBytes: 1.2 * GB, modified: "2024-05-01" },
      { path: "/Users/me/Documents/report-final 副本.pdf", name: "report-final 副本.pdf", dir: "/Users/me/Documents", sizeBytes: 1.2 * GB, modified: "2024-05-02" },
    ],
  },
];

export const mockLaunchItems: LaunchItem[] = [
  { name: "Dropbox" },
  { name: "Spotify" },
  { name: "Discord" },
  { name: "Docker Desktop" },
  { name: "1Password 7" },
];

export const mockSystemTools: ToolInfo[] = [
  { id: "storage", name: "管理存储空间", description: "查看并优化磁盘占用", action: "open" },
  { id: "disk-util", name: "磁盘工具", description: "修复与维护宗卷", action: "open" },
  { id: "activity", name: "活动监视器", description: "查看进程与资源占用", action: "open" },
  { id: "login-items", name: "登录项", description: "管理开机启动程序", action: "open" },
];

export const mockHistory: Snapshot[] = [
  { ts: Date.now() - 6 * 86400000, root: "/", totalBytes: 366 * GB, totalFiles: 1_820_104 },
  { ts: Date.now() - 4 * 86400000, root: "/", totalBytes: 360 * GB, totalFiles: 1_831_220 },
  { ts: Date.now() - 2 * 86400000, root: "/", totalBytes: 354 * GB, totalFiles: 1_840_880 },
  { ts: Date.now() - 1 * 86400000, root: "/", totalBytes: 351 * GB, totalFiles: 1_843_205 },
];

export const mockLastCleanup: CleanupRecord = {
  ts: Date.now() - 86400000,
  items: [
    { path: "/Users/me/Library/Caches/com.apple.Safari", sizeBytes: 6 * GB },
    { path: "/Users/me/Library/Developer/Xcode/DerivedData", sizeBytes: 9 * GB },
  ],
};

export const mockCleanSummary: CleanSummary = {
  freedBytes: 15 * GB,
  movedBytes: 15 * GB,
  failures: [],
  files: 412,
  durationMs: 38_400,
};

export const mockLargeFiles: FileEntry[] = [
  { path: "/Users/me/Movies/Project/CamA.mov", name: "CamA.mov", dir: "/Users/me/Movies/Project", sizeBytes: 12 * GB, modified: "2024-02-18" },
  { path: "/Users/me/Downloads/ubuntu-24.04.iso", name: "ubuntu-24.04.iso", dir: "/Users/me/Downloads", sizeBytes: 5.3 * GB, modified: "2024-01-09" },
  { path: "/Users/me/Pictures/Archive.zip", name: "Archive.zip", dir: "/Users/me/Pictures", sizeBytes: 3.8 * GB, modified: "2023-11-22" },
  { path: "/Users/me/Documents/backup.dmg", name: "backup.dmg", dir: "/Users/me/Documents", sizeBytes: 2.1 * GB, modified: "2024-03-30" },
  { path: "/Users/me/Library/Application Support/Steam/appcache.vdf", name: "appcache.vdf", dir: "/Users/me/Library/Application Support/Steam", sizeBytes: 1.4 * GB, modified: "2024-04-11" },
];

export const mockMigrationOptions: MigrationOptions = {
  supported: true,
  folders: [
    { id: "desktop", name: "桌面", path: "/Users/me/Desktop", blockedReason: null },
    { id: "documents", name: "文稿", path: "/Users/me/Documents", blockedReason: null },
    { id: "downloads", name: "下载", path: "/Users/me/Downloads", blockedReason: null },
    { id: "pictures", name: "图片", path: "/Users/me/Pictures", blockedReason: null },
    { id: "music", name: "音乐", path: "/Users/me/Music", blockedReason: "当前磁盘不是系统盘，已跳过" },
  ],
  drives: [
    { path: "/Volumes/Data", availableBytes: 1200 * GB },
    { path: "/Volumes/Backup", availableBytes: 480 * GB },
  ],
};

export const mockMigrationStatus: MigrationStatus = {
  phase: "idle",
  message: "",
  source: "",
  target: "",
  journal: "",
};

/** init() 直接预填的核心 state，保证各页面进入即可见内容。 */
export const mockState = {
  platform: "macos" as string,
  disks: mockDisks,
  settings: mockSettings,
  scanStatus: mockScanStatus,
  scanResult: mockScanResult,
  junk: mockJunk,
  dupGroups: mockDupGroups,
  history: mockHistory,
  lastCleanup: mockLastCleanup,
  cleanSummary: mockCleanSummary,
  largeFiles: mockLargeFiles,
  launchItems: mockLaunchItems,
  systemTools: mockSystemTools,
  migrationOptions: mockMigrationOptions,
  migrationStatus: mockMigrationStatus,
};

/** 覆盖 bridge 中所有 invoke 命令的返回；未列出的命令返回 undefined。 */
export function mockInvoke(
  cmd: string,
  _args?: Record<string, unknown>,
): unknown {
  switch (cmd) {
    case "get_platform":
      return "macos";
    case "get_disks":
      return mockDisks;
    case "get_settings":
      return mockSettings;
    case "get_scan_status":
      return mockScanStatus;
    case "get_scan_result":
      return mockScanResult;
    case "get_junk_categories":
      return mockJunk;
    case "find_duplicates":
      return mockDupGroups;
    case "get_large_files":
      return mockLargeFiles;
    case "get_launch_items":
      return mockLaunchItems;
    case "get_system_tools":
      return mockSystemTools;
    case "get_scan_history":
      return mockHistory;
    case "last_cleanup_record":
      return mockLastCleanup;
    case "get_migration_options":
      return mockMigrationOptions;
    case "get_migration_status":
      return mockMigrationStatus;
    case "prepare_migration":
      return {
        request: _args?.request,
        name: "桌面",
        source: "/Users/me/Desktop",
        target: "/Volumes/Data/Users/me/Desktop",
        totalBytes: 30 * GB,
        fileCount: 1820,
        availableBytes: 1200 * GB,
      } as MigrationPlan;
    case "migrate_folder":
      return {
        phase: "preparing",
        message: "正在复制文件…",
        source: "/Users/me/Desktop",
        target: "/Volumes/Data/Users/me/Desktop",
        journal: "/Users/me/.cache/diskcleanup/migrations/plan.json",
      } as MigrationStatus;
    case "get_auto_clean_status":
      return true;
    case "run_auto_clean_now":
      return mockCleanSummary;
    case "restore_last_cleanup":
      return { restored: 0, failures: [] } as RestoreSummary;
    case "run_system_tool":
      return "已打开工具（模拟）";
    case "get_task_status":
      return {
        running: false,
        visited: 0,
        skipped: 0,
        currentPath: "",
        cancelled: false,
        notice: "",
      } as TaskStatus;
    case "pick_directory":
      return "/tmp/mock-folder";
    // 以下为动作类命令，mock 下为 no-op
    case "start_scan":
    case "cancel_scan":
    case "clean_junk":
    case "save_settings":
    case "open_trash":
    case "close_window":
    case "cancel_task":
      return undefined;
    default:
      return undefined;
  }
}

/** mock 模式下事件监听为 no-op（不推送模拟事件）。 */
export function mockListen(): Promise<() => void> {
  return Promise.resolve(() => {});
}

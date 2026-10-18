import { create } from "zustand";
import type {
  DiskInfo,
  Page,
  ScanStatus,
  ScanResult,
  JunkCategory,
  DuplicateGroup,
  CleanSummary,
  Settings,
  Snapshot,
  CleanupRecord,
} from "../types";
import { bridge, onScanProgress, onScanFinished, onCleanFinished } from "../services/bridge";
import { isMockMode, mockState } from "../services/mock";

interface AppState {
  platform: string;
  page: Page;
  disks: DiskInfo[];
  activeDisk: string | null;
  scanStatus: ScanStatus;
  scanResult: ScanResult | null;
  scanError: string | null;
  scanStartedAt: number | null;
  scanStarting: boolean;
  junk: JunkCategory[];
  junkLoading: boolean;
  junkLoaded: boolean;
  junkSelection: Record<string, boolean>;
  dupGroups: DuplicateGroup[];
  dupLoading: boolean;
  dupError: string | null;
  cleaning: boolean;
  cleanRequested: number;
  pendingCleanup: string[] | null;
  confirmCleanup: () => Promise<void>;
  cancelCleanup: () => void;
  cleanError: string | null;
  junkError: string | null;
  cleanSummary: CleanSummary | null;
  settings: Settings | null;
  history: Snapshot[];
  lastCleanup: CleanupRecord | null;

  go: (p: Page) => void;
  init: () => Promise<void>;
  startScan: () => Promise<void>;
  cancelScan: () => Promise<void>;
  loadJunk: (force?: boolean) => Promise<void>;
  toggleJunkItem: (id: string, on: boolean) => void;
  setAllJunk: (on: boolean) => void;
  cleanSelected: (paths: string[]) => Promise<void>;
  loadDuplicates: () => Promise<void>;
  saveSettings: (s: Settings) => Promise<string | null>;
  selectedJunkBytes: () => number;
  loadHistory: () => Promise<void>;
  runAutoCleanNow: () => Promise<CleanSummary>;
  restoreLastCleanup: () => Promise<string>;
  loadLastCleanup: () => Promise<void>;
}

const emptyStatus: ScanStatus = {
  running: false,
  percent: 0,
  scannedBytes: 0,
  scannedFiles: 0,
  currentPath: "",
  recentFiles: [],
};

let initPromise: Promise<void> | null = null;
let scanAcceptEvents = false;

export const useApp = create<AppState>((set, get) => ({
  platform: "unknown",
  page: "home",
  disks: [],
  activeDisk: null,
  scanStatus: emptyStatus,
  scanResult: null,
  scanError: null,
  scanStartedAt: null,
  scanStarting: false,
  junk: [],
  junkLoading: false,
  junkLoaded: false,
  junkSelection: {},
  dupGroups: [],
  dupLoading: false,
  dupError: null,
  cleaning: false,
  cleanRequested: 0,
  pendingCleanup: null,
  cleanError: null,
  junkError: null,
  cleanSummary: null,
  settings: null,
  history: [],
  lastCleanup: null,

  go: (p) => {
    if (p !== "analyze") { set({ page: p }); return; }
    if (get().scanStatus.running) { set({ page: "analyze" }); return; }
    if (get().scanResult) { set({ page: "results" }); return; }
    void get().startScan();
  },

  init: () => {
    if (initPromise) return initPromise;
    if (isMockMode) {
      initPromise = Promise.resolve();
      const sel: Record<string, boolean> = {};
      for (const c of mockState.junk) for (const i of c.items) sel[i.id] = i.safe;
      set({
        platform: mockState.platform,
        disks: mockState.disks,
        settings: mockState.settings,
        activeDisk: mockState.disks.find((d) => d.isSystem)?.mountPoint ?? null,
        scanStatus: mockState.scanStatus,
        scanResult: mockState.scanResult,
        junk: mockState.junk,
        junkSelection: sel,
        junkLoaded: true,
        dupGroups: mockState.dupGroups,
        history: mockState.history,
        lastCleanup: mockState.lastCleanup,
        cleanSummary: mockState.cleanSummary,
      });
      return initPromise;
    }
    initPromise = (async () => {
      const [platform, disks, settings] = await Promise.all([
        bridge.getPlatform(), bridge.getDisks(), bridge.getSettings(),
      ]);
      const sys = disks.find((d) => d.isSystem);
      set({ platform, disks, settings, activeDisk: get().activeDisk ?? sys?.mountPoint ?? disks[0]?.mountPoint ?? null });
      // Await subscription registration before a scan can start, including very short scans.
      const subscriptions: (() => void)[] = [];
      try {
        subscriptions.push(await onScanProgress((s) => {
          if (!scanAcceptEvents) return;
          set({ scanStatus: s, scanError: s.error ?? null });
        }));
        subscriptions.push(await onScanFinished((r) => {
          if (!scanAcceptEvents) return;
          set((st) => ({ scanResult: r, scanStatus: { ...st.scanStatus, running: false, percent: 100,
            scannedBytes: r.totalBytes, scannedFiles: r.totalFiles },
            page: st.page === "analyze" ? "results" : st.page }));
        }));
        subscriptions.push(await onCleanFinished((s) => {
          set({ cleanSummary: s, cleaning: false, cleanError: null, page: "done", dupGroups: [] });
          void get().loadLastCleanup().catch(() => {});
          void bridge.getDisks().then(disks => set({ disks })).catch(() => {});
          get().loadJunk(true);
        }));
      } catch (error) {
        subscriptions.forEach((unlisten) => unlisten());
        throw error;
      }
    })().catch((error) => {
      initPromise = null;
      set({ scanError: `初始化失败：${String(error)}` });
      throw error;
    });
    return initPromise;
  },

  startScan: async () => {
    if (get().scanStatus.running || get().scanStarting) { set({ page: "analyze" }); return; }
    scanAcceptEvents = true;
    set({ scanStatus: { ...emptyStatus, running: true }, scanResult: null, scanError: null,
      scanStarting: true, scanStartedAt: Date.now(), page: "analyze" });
    try {
      await get().init();
      const root = get().activeDisk;
      set((st) => ({ scanStatus: { ...st.scanStatus, currentPath: root ?? "" } }));
      await bridge.startScan(root ?? undefined);
    } catch (error) {
      scanAcceptEvents = false;
      set((st) => ({ scanError: `无法开始扫描：${String(error)}`, scanStatus: { ...st.scanStatus, running: false } }));
    } finally {
      set({ scanStarting: false });
    }
  },

  cancelScan: async () => {
    try {
      await bridge.cancelScan();
      scanAcceptEvents = false;
      set((st) => ({ scanStatus: { ...st.scanStatus, running: false }, scanResult: null, page: "home" }));
    } catch (error) {
      set({ scanError: `取消失败：${String(error)}` });
    }
  },

  loadJunk: async (force = false) => {
    if (get().junkLoading) return;
    if (!force && get().junkLoaded) return;
    set({ junkLoading: true, junkError: null });
    try {
      const cats = await bridge.getJunkCategories();
      const sel: Record<string, boolean> = {};
      for (const c of cats) for (const i of c.items) sel[i.id] = i.safe;
      set({ junk: cats, junkSelection: sel, junkLoading: false, junkLoaded: true });
    } catch (e) {
      set({ junkLoading: false, junkError: `扫描失败：${String(e)}` });
    }
  },

  toggleJunkItem: (id, on) =>
    set((st) => ({ junkSelection: { ...st.junkSelection, [id]: on } })),

  setAllJunk: (on) =>
    set((st) => {
      const sel: Record<string, boolean> = {};
      for (const c of st.junk) for (const i of c.items) sel[i.id] = on;
      return { junkSelection: sel };
    }),

  cleanSelected: async (paths) => {
    if (get().cleaning || !paths.length) return;
    set({ pendingCleanup: [...new Set(paths)] });
  },
  cancelCleanup: () => set({ pendingCleanup: null }),
  confirmCleanup: async () => {
    const paths = get().pendingCleanup;
    if (get().cleaning || !paths?.length) return;
    set({ pendingCleanup: null });
    set({ cleaning: true, cleanError: null, cleanRequested: paths.length, lastCleanup: null });
    try { await get().init(); await bridge.cleanJunk(paths); }
    catch (e) { set({ cleaning: false, cleanError: `清理失败：${String(e)}` }); }
  },

  loadDuplicates: async () => {
    if (get().dupLoading) return;
    set({ dupLoading: true, dupError: null });
    try {
      const groups = await bridge.findDuplicates(get().activeDisk ?? null);
      set({ dupGroups: groups, dupLoading: false });
    } catch (e) {
      set({ dupLoading: false, dupError: `扫描失败：${String(e)}` });
    }
  },

  /** 保存设置，返回错误文案（成功返回 null） */
  saveSettings: async (s) => {
    try {
      await bridge.saveSettings(s);
      set({ settings: s, junkLoaded: false, junk: [], junkSelection: {}, dupGroups: [] });
      return null;
    } catch (e) {
      return typeof e === "string" ? e : String(e);
    }
  },

  loadHistory: async () => {
    const history = await bridge.getScanHistory();
    set({ history });
  },

  runAutoCleanNow: async () => {
    const summary = await bridge.runAutoCleanNow();
    const settings = await bridge.getSettings();
    set({ settings, cleanSummary: summary, cleanRequested: summary.files + (summary.failures?.length ?? 0) });
    await get().loadLastCleanup();
    void bridge.getDisks().then(disks => set({ disks }));
    set({ junkLoaded: false });
    return summary;
  },

  restoreLastCleanup: async () => {
    try {
      const n = await bridge.restoreLastCleanup();
      await get().loadLastCleanup();
      return `已恢复 ${n.restored} 项${n.failures.length ? `；${n.failures.length} 项未恢复，记录已保留：\n${n.failures.map(f => `${f.path}：${f.reason}`).join("\n")}` : ""}`;
    } catch (e) {
      return typeof e === "string" ? e : String(e);
    }
  },

  loadLastCleanup: async () => {
    const rec = await bridge.lastCleanupRecord();
    set({ lastCleanup: rec });
  },

  selectedJunkBytes: () => {
    const { junk, junkSelection } = get();
    let sum = 0;
    for (const c of junk) for (const i of c.items) if (junkSelection[i.id]) sum += i.sizeBytes;
    return sum;
  },
}));

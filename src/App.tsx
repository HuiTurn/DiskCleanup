import { useEffect, useState } from "react";
import Sidebar from "./components/Sidebar";
import { useApp } from "./stores/appStore";
import type { Page } from "./types";
import {
  bridge,
  onCloseRequested,
  onMigrationCloseBlocked,
} from "./services/bridge";
import FileMigration from "./pages/FileMigration";
import Home from "./pages/Home";
import Analyze from "./pages/Analyze";
import Results from "./pages/Results";
import Junk from "./pages/Junk";
import Done from "./pages/Done";
import LargeFiles from "./pages/LargeFiles";
import Duplicates from "./pages/Duplicates";
import LaunchItems from "./pages/LaunchItems";
import SystemTools from "./pages/SystemTools";
import SettingsPage from "./pages/Settings";

const PAGES: Record<string, React.ComponentType> = {
  home: Home,
  "file-migration": FileMigration,
  analyze: Analyze,
  results: Results,
  junk: Junk,
  done: Done,
  "large-files": LargeFiles,
  duplicates: Duplicates,
  "launch-items": LaunchItems,
  "system-tools": SystemTools,
  settings: SettingsPage,
};

export default function App() {
  const page = useApp((s) => s.page);
  const trash = useApp(s => s.platform === "windows" ? "回收站" : "废纸篓");
  const pendingCleanup = useApp(s => s.pendingCleanup);
  const cleaning = useApp((s) => s.cleaning);
  const theme = useApp((s) => s.settings?.theme ?? "light");
  const [askClose, setAskClose] = useState(false);
  const Page = PAGES[page] ?? Home;

  // 支持 ?page=xxx 直接定位初始页面（用于 CI / 浏览器预览截图）
  useEffect(() => {
    const target = new URLSearchParams(window.location.search).get("page");
    const valid: Page[] = [
      "home", "file-migration", "analyze", "results", "junk", "done",
      "large-files", "duplicates", "launch-items", "system-tools", "settings",
    ];
    if (target && (valid as string[]).includes(target)) {
      useApp.setState({ page: target as Page });
    }
  }, []);

  useEffect(() => {
    useApp
      .getState()
      .init()
      .catch(() => {});
    void useApp.getState().loadLastCleanup().catch(() => {});
  }, []);

  // 主题：写入根节点，由 CSS 变量切换整套配色
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const applyTheme = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (media.matches ? "dark" : "light") : theme;
    };
    applyTheme();
    media.addEventListener("change", applyTheme);
    return () => media.removeEventListener("change", applyTheme);
  }, [theme]);

  // 退出前确认（后端拦截关闭后推送事件）
  useEffect(() => {
    let un: (() => void) | undefined;
    let disposed = false;
    onCloseRequested(() => setAskClose(true)).then(f => { if (disposed) f(); else un = f; }).catch(() => {});
    return () => { disposed = true; un?.(); };
  }, []);

  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    onMigrationCloseBlocked(() => {
      setAskClose(false);
      useApp.getState().go("file-migration");
    })
      .then((fn) => {
        if (disposed) fn();
        else off = fn;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      off?.();
    };
  }, []);

  return (
    <div className="layout">
      <Sidebar />
      <main key={page} className={`main page-${page}`}>
        {cleaning && <div className="feedback cleaning-status" role="status">正在将所选文件移入{useApp.getState().platform === "windows" ? "回收站" : "废纸篓"}，完成后会显示处理结果。</div>}
        <Page />
      </main>

      {pendingCleanup && <div className="modal-mask"><div className="modal" role="dialog" aria-modal="true" aria-labelledby="cleanup-review-title" onKeyDown={e => { if (e.key === "Escape") useApp.getState().cancelCleanup(); }}>
        <h2 id="cleanup-review-title">确认移入{trash}</h2>
        <p>已选择 {pendingCleanup.length} 个文件。请核对完整路径；系统、应用依赖、虚拟磁盘及排除目录会被保护。移入{trash}不会立即释放磁盘空间。</p>
        <ul style={{ maxHeight: '40vh', overflow: 'auto', overflowWrap: 'anywhere' }}>{pendingCleanup.map(path => <li key={path}>{path}</li>)}</ul>
        <div className="modal-actions"><button className="btn btn-ghost" autoFocus onClick={() => useApp.getState().cancelCleanup()}>取消</button><button className="btn btn-primary" onClick={() => void useApp.getState().confirmCleanup()}>确认移入</button></div>
      </div></div>}
      {askClose && (
        <div className="modal-mask">
          <div className="modal">
            <div className="modal-title">确认退出 DiskCleanup？</div>
            <div className="modal-desc">若有正在进行的扫描任务将被中断。</div>
            <div className="modal-actions">
              <button
                className="btn btn-ghost"
                onClick={() => setAskClose(false)}
              >
                取消
              </button>
              <button
                className="btn btn-primary"
                onClick={() => {
                  setAskClose(false);
                  bridge.closeWindow().catch(() => {});
                }}
              >
                退出
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

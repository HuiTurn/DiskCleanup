import appLogo from "../assets/brand/app-logo-v2.png";
import type { Page } from "../types";
import { useApp } from "../stores/appStore";
import {
  IconMove,
  IconHome,
  IconScan,
  IconBroom,
  IconFile,
  IconCopy,
  IconPower,
  IconTools,
  IconSettings,
} from "./Icons";

const NAV: {
  id: Page;
  label: string;
  icon: (p: { size?: number }) => React.ReactNode;
}[] = [
  { id: "home", label: "首页", icon: IconHome },
  { id: "analyze", label: "磁盘分析", icon: IconScan },
  { id: "junk", label: "垃圾清理", icon: IconBroom },
  { id: "large-files", label: "大文件管理", icon: IconFile },
  { id: "file-migration", label: "文件迁移", icon: IconMove },
  { id: "duplicates", label: "重复文件", icon: IconCopy },
  { id: "launch-items", label: "启动项管理", icon: IconPower },
  { id: "system-tools", label: "系统工具", icon: IconTools },
];

export default function Sidebar() {
  const page = useApp((s) => s.page);
  const go = useApp((s) => s.go);
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-logo">
          <img src={appLogo} alt="" draggable={false} />
        </div>
        <div>
          <div className="brand-name">DiskCleanup</div>
          <div className="brand-sub">让电脑更清爽更快捷</div>
        </div>
      </div>
      <nav className="nav" aria-label="主要功能">
        {NAV.map((n) => (
          <button
            key={n.id}
            className={`nav-item ${page === n.id || (n.id === "analyze" && page === "results") ? "active" : ""}`}
            aria-current={
              page === n.id || (n.id === "analyze" && page === "results")
                ? "page"
                : undefined
            }
            onClick={() => go(n.id)}
          >
            <n.icon size={21} />
            {n.label}
          </button>
        ))}
      </nav>
      <button
        className={`nav-item ${page === "settings" ? "active" : ""}`}
        aria-current={page === "settings" ? "page" : undefined}
        onClick={() => go("settings")}
      >
        <IconSettings size={21} />
        设置
      </button>
    </aside>
  );
}

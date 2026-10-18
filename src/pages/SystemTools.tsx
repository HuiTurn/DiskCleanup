import { IconPower, IconBroom, IconDrive, IconShield, IconInfo, IconTrash, IconFolder, IconTools, IconArrow } from "../components/Icons";
import Illustration from "../components/Illustration";
import { useEffect, useState } from "react";
import { useApp } from "../stores/appStore";
import { bridge } from "../services/bridge";
import type { ToolInfo } from "../types";

const ICON_STYLE = [
  { c: "#1677ff", b: "#e8f1ff" },
  { c: "#2e7bf6", b: "#e8f1ff" },
  { c: "#22c55e", b: "#e8f9ee" },
  { c: "#f59e0b", b: "#fef4e6" },
  { c: "#13a88a", b: "#e3f7f1" },
  { c: "#31c3ff", b: "#e6f8ff" },
];

function ToolGlyph({ id, color }: { id: string; color: string }) {
  const icons: Record<string, typeof IconTools> = {
    "login-items": IconPower, startup: IconPower, "startup-folders": IconFolder,
    "junk-clean": IconBroom, "update-clean": IconBroom,
    smart: IconShield, "disk-utility": IconDrive, defrag: IconDrive,
    storage: IconDrive, "storage-sense": IconDrive, about: IconInfo, trash: IconTrash,
  };
  const Icon = icons[id] ?? IconTools;
  return <Icon size={24} color={color} />;
}

export default function SystemTools() {
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [detail, setDetail] = useState<string | null>(null);

  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState<string | null>(null);
  const reload = async () => {
    setError(''); setLoading(true);
    try { setTools(await bridge.getSystemTools()); }
    catch (e) { setError(`读取失败：${String(e)}`); }
    finally { setLoading(false); }
  };
  useEffect(() => { void reload(); }, []);

  const run = async (t: ToolInfo) => {
    if (t.action === "junk-page") {
      useApp.getState().go("junk");
      return;
    }
    setRunning(t.id); setError('');
    try { const res = await bridge.runSystemTool(t.id); setDetail(res ?? `已打开「${t.name}」`); }
    catch (e) { setError(`打开失败：${String(e)}`); }
    finally { setRunning(null); }
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <div className="page-title">系统工具</div>
          <div className="page-sub">
            常用系统能力，集中管理
            <span className="muted" style={{ marginLeft: 8 }}>
              （{useApp.getState().platform === "macos" ? "已适配 macOS 系统能力" : "已适配 Windows 系统能力"}）
            </span>
          </div>
        </div>
      </div>

      <div className="feature-banner tools-banner"><div><h2>常用工具，触手可及</h2><p>从磁盘到启动项，快速进入系统管理工具。</p></div><Illustration name="tools" /></div>
      {error && <p className="feedback error" role="alert">{error} <button className="link-btn" onClick={reload}>重新加载</button></p>}
      {loading && <div className="empty-state"><div className="spin" /></div>}
      <div className="tool-grid">
        {tools.map((t, i) => {
          const st = ICON_STYLE[i % ICON_STYLE.length];
          return (
            <button key={t.id} className="tool-card" disabled={running !== null} onClick={() => run(t)}>
              <div className="tool-ico" style={{ background: st.b }}>
                <ToolGlyph id={t.id} color={st.c} />
              </div>
              <div>
                <div className="t">{t.name}</div>
                <div className="d">{t.description}</div>
              </div>
              <span className="tool-arrow" aria-hidden="true"><IconArrow size={16} /></span>
            </button>
          );
        })}
      </div>

      <p className="muted" style={{ marginTop: 20 }}>点击工具将在系统窗口中打开，部分操作可能需要管理员权限。</p>
      {detail && (
        <div className="footer-bar" style={{ justifyContent: "space-between" }}>
          <span role="status" style={{ fontWeight: 600 }}>{detail}</span>
          <button className="link-btn" onClick={() => setDetail(null)}>关闭</button>
        </div>
      )}
    </div>
  );
}

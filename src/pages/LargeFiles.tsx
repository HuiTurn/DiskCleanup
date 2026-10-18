import ScanTaskStatus from "../components/ScanTaskStatus";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useApp } from "../stores/appStore";
import { fmtBytes, FileIco, TabBar } from "../components/ui";
import { bridge } from "../services/bridge";
import type { FileEntry } from "../types";

const SIZE_OPTIONS = [
  { v: 10, label: "> 10 MB" },
  { v: 100, label: "> 100 MB" },
  { v: 500, label: "> 500 MB" },
  { v: 1024, label: "> 1 GB" },
];

const fileCategory = (name: string) => {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["mp4", "mkv", "mov", "avi"].includes(ext)) return "视频";
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return "压缩文件";
  if (["exe", "msi", "dmg", "pkg", "iso", "app"].includes(ext)) return "安装包";
  if (["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt"].includes(ext)) return "文档";
  return "其他";
};

export default function LargeFiles() {
  const busy = useApp(s => s.cleaning);
  const cleanError = useApp(s => s.cleanError);
  const platform = useApp(s => s.platform);
  const trash = platform === 'windows' ? '回收站' : '废纸篓';
  const activeDisk = useApp((s) => s.activeDisk);
  const largeFileMinMb = useApp((s) => s.settings?.largeFileMinMb ?? 100);
  const [minMb, setMinMb] = useState(largeFileMinMb);
  const [category, setCategory] = useState("全部文件");
  const [keyword, setKeyword] = useState("");
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState<Record<string, boolean>>({});

  const requestId = useRef(0);
  const [error, setError] = useState("");
  const reload = useCallback(async (force = false) => {
    const id = ++requestId.current;
    setError("");
    setLoading(true);
    try {
      const res = await bridge.getLargeFiles(activeDisk, minMb, 500, force);
      if (id !== requestId.current) return;
      setFiles(res);
      setSelected({});
    } catch (e) {
      if (id === requestId.current) setError(`扫描失败：${String(e)}`);
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, [activeDisk, minMb]);

  useEffect(() => {
    reload();
    return () => { requestId.current++; };
  }, [reload]);

  const tabs = useMemo(() => ["全部文件", "视频", "压缩文件", "安装包", "文档", "其他"].map((name) => ({
    id: name,
    label: name === "全部文件" ? name : `${name} (${fmtBytes(files.filter((f) => fileCategory(f.name) === name).reduce((sum, f) => sum + f.sizeBytes, 0))})`,
  })), [files]);
  const shown = useMemo(() => files.filter((f) => f.name.toLowerCase().includes(keyword.toLowerCase()) && (category === "全部文件" || fileCategory(f.name) === category)), [files, keyword, category]);
  const selCount = shown.filter((f) => selected[f.path] && !f.protectedReason).length;
  const selBytes = shown.filter((f) => selected[f.path] && !f.protectedReason).reduce((a, f) => a + f.sizeBytes, 0);

  const cleanSelected = async () => {
    const paths = shown.filter((f) => selected[f.path] && !f.protectedReason).map((f) => f.path);
    if (paths.length === 0) return;
    await useApp.getState().cleanSelected(paths);
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <div className="page-title">大文件管理</div>
          <div className="page-sub">查找占用空间较大的文件，便于清理备份</div>
        </div>
        <button className="btn btn-ghost" onClick={() => reload(true)} disabled={loading || busy}>重新扫描</button>
      </div>
        <div className="file-toolbar"><span className="muted">文件大小</span>
          <select className="select" aria-label="文件大小阈值" disabled={loading || busy} value={minMb} onChange={(e) => setMinMb(Number(e.target.value))}>
            {SIZE_OPTIONS.map((o) => (
              <option key={o.v} value={o.v}>{o.label}</option>
            ))}
          </select>
          <input
            className="search-input"
            aria-label="搜索文件"
            placeholder="⌕ 搜索文件或关键词"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
          />
        </div>

      <ScanTaskStatus kind="large" loading={loading} />
      {cleanError && <p className="feedback error" role="alert">{cleanError}</p>}
      <div className="file-tabs"><TabBar tabs={tabs} active={category} onChange={setCategory} /></div>
      <div className="file-results">
        {error ? <div className="empty-state" role="alert">{error}</div> : loading ? (
          <div className="empty-state"><div className="spin" /><p style={{ marginTop: 12 }}>正在扫描大文件…</p></div>
        ) : (
          <div className="scroll-table">
            <table className="table">
              <thead>
                <tr>
                  <th style={{ width: 34 }}>
                    <input
                      type="checkbox"
                      className="checkbox"
                      aria-label="选择当前列表全部文件"
                      checked={shown.length > 0 && shown.filter(f => !f.protectedReason).length > 0 && shown.filter(f => !f.protectedReason).every((f) => selected[f.path])}
                      onChange={(e) =>
                        setSelected(Object.fromEntries(shown.filter(f => !f.protectedReason).map((f) => [f.path, e.target.checked])))
                      }
                    />
                  </th>
                  <th>类型</th>
                  <th>文件名</th>
                  <th className="num">大小</th>
                  <th>目录</th>
                  <th>修改日期</th>
                  <th style={{ width: 60 }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {shown.map((f) => (
                  <tr key={f.path}>
                    <td>
                      <input
                        type="checkbox"
                        className="checkbox"
                        aria-label={`选择 ${f.name}`}
                        disabled={!!f.protectedReason}
                        checked={!!selected[f.path]}
                        onChange={(e) => setSelected({ ...selected, [f.path]: e.target.checked })}
                      />
                    </td>
                    <td><FileIco name={f.name} /></td>
                    <td style={{ fontWeight: 600 }}>{f.name}{f.protectedReason && <span className="similarity-badge" title={f.protectedReason}>受保护</span>}</td>
                    <td className="num">{fmtBytes(f.sizeBytes)}</td>
                    <td className="path-cell" title={f.dir}>{f.dir}</td>
                    <td style={{ color: "var(--text-3)" }}>{f.modified}</td>
                    <td>
                      <button className="link-btn" title={f.protectedReason ?? "核对后移入回收站"} disabled={busy || !!f.protectedReason} onClick={() => void useApp.getState().cleanSelected([f.path])}>
                        {f.protectedReason ? "受保护" : "清理"}
                      </button>
                    </td>
                  </tr>
                ))}
                {shown.length === 0 && (
                  <tr><td colSpan={7}><div className="empty-state">未发现符合条件的文件</div></td></tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="footer-bar">
        <div>
          <span style={{ fontWeight: 700 }}>已选 {selCount} 个文件 ({fmtBytes(selBytes)})</span>
          <span className="muted" style={{ marginLeft: 8 }}>（移入{trash}）</span>
        </div>
        <div style={{ display: "flex", gap: 10 }}>
          <button className="btn btn-ghost" onClick={() => setSelected({})}>取消所选</button>
          <button className="btn btn-primary" disabled={selCount === 0 || busy} onClick={cleanSelected}>
            {busy ? "正在处理…" : `移入${trash}`}
          </button>
        </div>
      </div>
    </div>
  );
}

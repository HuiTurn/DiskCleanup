import ScanTaskStatus from "../components/ScanTaskStatus";
import Illustration from "../components/Illustration";
import { useEffect, useState } from "react";
import { useApp } from "../stores/appStore";
import { fmtBytes, FileIco } from "../components/ui";
import type { JunkItem } from "../types";
import { IconRefresh } from "../components/Icons";

export default function Junk() {
  const platform = useApp(s => s.platform);
  const busy = useApp(s => s.cleaning);
  const error = useApp(s => s.junkError);
  const cleanError = useApp(s => s.cleanError);
  const trash = platform === 'windows' ? '回收站' : '废纸篓';
  const junk = useApp((s) => s.junk);
  const loading = useApp((s) => s.junkLoading);
  const selection = useApp((s) => s.junkSelection);
  const loadJunk = useApp((s) => s.loadJunk);
  const toggle = useApp((s) => s.toggleJunkItem);
  const setAll = useApp((s) => s.setAllJunk);
  const clean = useApp((s) => s.cleanSelected);
  const [detail, setDetail] = useState<JunkItem | null>(null);
  const [visible, setVisible] = useState(100);
  const [cat, setCat] = useState<string>("");

  useEffect(() => {
    loadJunk();
  }, [loadJunk]);


  const items = cat ? junk.find((c) => c.id === cat)?.items ?? [] : junk.flatMap((c) => c.items);
  const selectedBytes = useApp.getState().selectedJunkBytes();
  const selectedCount = Object.values(selection).filter(Boolean).length;
  const totalBytes = junk.reduce((a, c) => a + c.totalBytes, 0);

  return (
    <div>
      <div className="page-head">
        <div>
          <div className="page-title">垃圾清理</div>
          <div className="page-sub">选择需要整理的缓存与临时文件</div>
        </div>
        <div className="head-actions">
          <button className="btn btn-ghost" disabled={loading || busy} onClick={() => loadJunk(true)}>
            <IconRefresh size={13} />
            重新扫描
          </button>
        </div>
      </div>

      <ScanTaskStatus kind="junk" loading={loading} />
      {(error || cleanError) && <p className="feedback error" role="alert">{error || cleanError}</p>}
      <section className="feature-banner"><Illustration name="cleanup" /><div><h2>{loading ? '正在扫描…' : `发现 ${fmtBytes(totalBytes)} 可整理文件`}</h2><p>已按文件去重；常见缓存默认勾选，其余项目请确认。目录结构将保留。</p></div><div className="banner-meta"><b>{fmtBytes(selectedBytes)}</b>已选择</div></section>
      {detail && <div className="modal-mask" onClick={() => setDetail(null)}><div className="modal" onKeyDown={(e) => { if (e.key === "Escape") setDetail(null); }} role="dialog" aria-modal="true" aria-labelledby="junk-detail-title" onClick={(e) => e.stopPropagation()}><div className="modal-title" id="junk-detail-title">{detail.name}</div><div className="modal-desc">{detail.description}</div><p className="detail-path">{detail.path}</p><div className="modal-desc">占用空间：{fmtBytes(detail.sizeBytes)}</div><div className="modal-actions"><button className="btn btn-primary" autoFocus onClick={() => setDetail(null)}>关闭</button></div></div></div>}
      {loading ? (
        <div className="empty-state"><div className="spin" /><p style={{ marginTop: 12 }}>正在扫描垃圾文件…</p></div>
      ) : junk.length === 0 ? (
        <div className="empty-state">{error ? "扫描未完成，请重试。" : "没有发现可清理的缓存文件。"}</div>
      ) : (
        <>
          <div className="junk-tabs">
            <button className={`junk-tab ${cat === "" ? "active" : ""}`} onClick={() => setCat("")}>全部 ({fmtBytes(totalBytes)})</button>
            {junk.map((c) => (
              <button key={c.id} className={`junk-tab ${cat === c.id ? "active" : ""}`} onClick={() => setCat(c.id)}>
                {c.name}
                <span className="b">{fmtBytes(c.totalBytes)}</span>
              </button>
            ))}
          </div>

          <div className="scroll-table junk-table">
            <table className="table">
              <thead><tr>
                <th style={{ width: 38 }}><input type="checkbox" className="checkbox" aria-label="选择当前分类全部清理项" checked={items.length > 0 && items.every((item) => selection[item.id])} onChange={(e) => items.forEach((item) => toggle(item.id, e.target.checked))} /></th>
                <th>文件名</th><th className="num">大小</th><th>清理说明</th><th style={{ width: 62 }}>详情</th>
              </tr></thead>
              <tbody>{items.slice(0, visible).map((item) => (
                <tr key={item.id} className={selection[item.id] ? "selected" : ""}>
                  <td><input type="checkbox" className="checkbox" aria-label={`选择${item.name}`} checked={!!selection[item.id]} onChange={(e) => toggle(item.id, e.target.checked)} /></td>
                  <td><div className="file-name"><FileIco name={item.path} size={22} /><span>{item.name}</span>{!item.safe && <span className="similarity-badge">需确认</span>}</div></td>
                  <td className="num">{fmtBytes(item.sizeBytes)}</td>
                  <td className="path-cell" title={item.description}>{item.description}</td>
                  <td><button className="link-btn" onClick={() => setDetail(item)}>详情</button></td>
                </tr>
              ))}</tbody>
            </table>
          </div>

          {items.length > visible && <button className="btn btn-ghost" onClick={() => setVisible(v => v + 100)}>显示更多（当前 {visible} / {items.length}）</button>}
          <div className="footer-bar">
            <div>
              <span style={{ fontWeight: 700 }}>已选择 {fmtBytes(selectedBytes)}</span>
              <span className="muted" style={{ marginLeft: 8 }}>
                （{selectedCount} 项 · 移入{trash}）
              </span>
            </div>
            <div style={{ display: "flex", gap: 10 }}>
              <button className="btn btn-ghost" onClick={() => setAll(true)}>全选</button>
              <button
                className="btn btn-primary"
                disabled={selectedCount === 0 || busy}
                onClick={() => clean(Object.keys(selection).filter((k) => selection[k]))}
              >
                {busy ? "正在处理…" : `移入${trash}`} ({fmtBytes(selectedBytes)})
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

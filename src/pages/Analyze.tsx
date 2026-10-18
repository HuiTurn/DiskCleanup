import Illustration from "../components/Illustration";
import { useEffect, useState } from "react";
import { useApp } from "../stores/appStore";
import { fmtBytes, fmtDuration, ProgressRing, FileIco } from "../components/ui";


export default function Analyze() {
  const status = useApp((s) => s.scanStatus);
  const platform = useApp((s) => s.platform);
  const disks = useApp((s) => s.disks);
  const activeDisk = useApp((s) => s.activeDisk);
  const cancelScan = useApp((s) => s.cancelScan);
  const error = useApp((s) => s.scanError);
  const starting = useApp((s) => s.scanStarting);
  const startedAt = useApp((s) => s.scanStartedAt);
  const startScan = useApp((s) => s.startScan);
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    const update = () => setElapsed(startedAt ? Date.now() - startedAt : 0);
    update();
    if (!status.running) return;
    const t = setInterval(update, 1000);
    return () => clearInterval(t);
  }, [startedAt, status.running]);

  const disk = disks.find((d) => d.mountPoint === activeDisk);
  const diskLabel = disk?.name ?? activeDisk ?? "磁盘";

  return (
    <div>
      <div className="page-head">
        <div>
          <div className="page-title">{error ? "磁盘分析遇到问题" : status.running ? `正在分析 ${diskLabel}…` : "磁盘分析"}</div>
          <div className="page-sub">
            {error ?? "正在扫描可访问的文件；文件数量较多时可能需要几分钟。"}
          </div>
        </div>
      </div>

      <div className="card scan-body">
        <div className="scan-ring-wrap">
          <ProgressRing
            percent={status.running ? 0 : status.percent}
            indeterminate={status.running}
            label={
              <div className="scan-ring-label"><Illustration name="scan" className="scan-ring-art" /><strong>{status.running ? "扫描中" : error ? "未完成" : "待开始"}</strong></div>
            }
          />
          <div className="scan-meta">已统计{platform === "macos" || platform === "linux" ? "分配空间" : "逻辑大小"} {fmtBytes(status.scannedBytes)} · {status.scannedFiles.toLocaleString()} 个文件</div>
        </div>

        <div className="scan-list">
          <div className="scan-metrics"><div><strong>{fmtBytes(status.scannedBytes)}</strong><span>已统计文件体积</span></div><div><strong>{status.scannedFiles.toLocaleString()}</strong><span>已扫描文件</span></div><div><strong>{fmtDuration(elapsed)}</strong><span>已用时间</span></div></div>
          {status.recentFiles.slice(-5).map((f, i) => {
            const name = f.split(/[\\/]/).pop() ?? f;
            return (
              <div className="scan-row" key={`${f}-${i}`}>
                <FileIco name={name} size={22} />
                <span className="p" title={f}>{name}</span>
              </div>
            );
          })}
          {status.recentFiles.length === 0 && (
            <div className="empty-state">{error ? "扫描未完成" : status.running ? "正在读取目录…" : "点击开始分析以扫描磁盘"}</div>
          )}
        </div>
      </div>

      <div className="card scan-bottom">
        <div className="muted" style={{ marginBottom: 6 }}>当前扫描路径</div>
        <div className="path">{status.currentPath || "-"}</div>
        <div className={`bar ${status.running ? "scan-indeterminate" : ""}`} role="progressbar" aria-label="磁盘扫描" aria-valuetext={status.running ? `扫描中，已发现 ${status.scannedFiles} 个文件` : "未在扫描"}>
          <div style={{ width: status.running ? "30%" : "0%" }} />
        </div>
        <div className="muted" style={{ marginTop: 10 }}>
          总文件数未知，扫描结束前不显示完成百分比。统计仅覆盖可读取文件，不等于整盘已用空间。
          {!!status.skippedEntries && ` 已跳过 ${status.skippedEntries.toLocaleString()} 个无法读取的项目。`}
        </div>
        <div className="scan-actions">
          {status.running ? (
            <button className="btn btn-ghost" disabled={starting} onClick={() => cancelScan()}>取消分析</button>
          ) : (
            <button className="btn btn-primary" onClick={() => startScan()}>{error ? "重试分析" : "开始分析"}</button>
          )}
        </div>
      </div>
    </div>
  );
}

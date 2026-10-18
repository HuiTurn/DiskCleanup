import { useEffect, useState } from "react";
import { useApp } from "../stores/appStore";
import { fmtBytes, fmtNum, DonutChart, LegendColor, TabBar } from "../components/ui";

import { IconFolder } from "../components/Icons";

const TABS = [
  { id: "folder", label: "文件夹" },
  { id: "dir-name", label: "目录占用" },
  { id: "size-dist", label: "大小分布" },
  { id: "history", label: "历史变化" },
];

export default function Results() {
  const result = useApp((s) => s.scanResult);
  const startScan = useApp((s) => s.startScan);
  const disks = useApp((s) => s.disks);
  const activeDisk = useApp((s) => s.activeDisk);
  const allHistory = useApp((s) => s.history);
  const loadHistory = useApp((s) => s.loadHistory);
  const [historyError, setHistoryError] = useState("");
  const [tab, setTab] = useState("folder");

  useEffect(() => {
    loadHistory().catch(e => setHistoryError(`读取历史失败：${String(e)}`));
  }, [loadHistory]);

  const disk = disks.find((d) => d.mountPoint === activeDisk);
  const diskLabel = disk ? `${disk.name} (${fmtBytes(disk.totalBytes)})` : activeDisk ?? "磁盘";

  if (!result) {
    return <div className="empty-state">暂无分析结果，请先运行磁盘分析。</div>;
  }

  const history = allHistory.filter((h) => h.root === result.root && (h.sizeBasis ?? "") === (result.sizeBasis ?? ""));
  const sizeLabel = result.sizeBasis === "allocated" ? "文件分配空间" : "文件逻辑大小";
  const maxBucket = Math.max(1, result.totalBytes);
  const maxHist = Math.max(1, ...history.map((h) => h.totalBytes));

  return (
    <div>
      <div className="page-head">
        <div>
          <div className="page-title">磁盘分析结果</div>
          <div className="page-sub">已统计 {fmtBytes(result.totalBytes)} {sizeLabel}</div>
        </div>
        <div className="head-actions">
          <button className="btn btn-ghost" onClick={() => startScan()}>重新分析</button>
          <span className="tag">{diskLabel}</span>
        </div>
      </div>

      <p className="muted">{result.sizeBasis === "allocated" ? "按文件已分配空间汇总，已对硬链接去重；共享克隆块仍可能重复计入，不代表可释放空间。" : "按文件逻辑大小汇总，稀疏文件可能大于实际占用。"} 排除目录、未读取文件和系统元数据不在统计内。</p>
      <div className="result-grid">
        <div className="card result-summary">
          <DonutChart
            data={result.categories}
            center={
              <div>
                <div className="v">{fmtBytes(result.totalBytes)}</div>
                <div className="l">{sizeLabel}</div>
              </div>
            }
          />
          <div className="legend" style={{ width: "100%" }}>
            {result.categories.map((c, i) => (
              <div className="legend-row" key={c.name}>
                <span className="legend-dot" style={{ background: LegendColor(i) }} />
                <span className="n">{c.name}</span>
                <span className="s">{fmtBytes(c.bytes)}</span>
                <span className="p">{c.percent.toFixed(1)}%</span>
              </div>
            ))}
          </div>
        </div>

        <div className="result-details">
          <div style={{ marginBottom: 12 }}>
            <TabBar tabs={TABS} active={tab} onChange={setTab} />
          </div>

          <div className={`result-panel ${tab === "size-dist" || tab === "history" ? "chart-panel" : ""}`}>
          {tab === "folder" && (
            <table className="table">
              <thead>
                <tr>
                  <th style={{ width: 28 }}>#</th>
                  <th>名称</th>
                  <th className="num">大小</th>
                  <th className="num">占比</th>
                  <th className="num">文件数</th>
                  <th style={{ width: 60 }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {result.topFolders.map((f, i) => (
                  <tr key={f.path}>
                    <td style={{ color: "var(--text-3)" }}>{i + 1}</td>
                    <td>
                      <div style={{ display: "flex", alignItems: "center", gap: 9 }}>
                        <IconFolder size={20} color="#ffb627" />
                        <span style={{ fontWeight: 600 }}>{f.name}</span>
                      </div>
                    </td>
                    <td className="num">{fmtBytes(f.sizeBytes)}</td>
                    <td className="num">{f.percent.toFixed(1)}%</td>
                    <td className="num">{fmtNum(f.fileCount)}</td>
                    <td>
                      <button
                        className="link-btn"
                        onClick={() => useApp.setState({ activeDisk: f.path, page: "large-files" })}
                      >
                        查看
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {tab === "dir-name" && (
            <table className="table">
              <thead>
                <tr>
                  <th style={{ width: 28 }}>#</th>
                  <th>目录名</th>
                  <th className="num">合计大小</th>
                  <th className="num">占比</th>
                  <th className="num">出现次数</th>
                  <th className="num">文件数</th>
                </tr>
              </thead>
              <tbody>
                {result.dirNames.map((d, i) => (
                  <tr key={d.name}>
                    <td style={{ color: "var(--text-3)" }}>{i + 1}</td>
                    <td style={{ fontWeight: 600 }}>{d.name}</td>
                    <td className="num">{fmtBytes(d.sizeBytes)}</td>
                    <td className="num">{d.percent.toFixed(1)}%</td>
                    <td className="num">{fmtNum(d.occurrences)}</td>
                    <td className="num">{fmtNum(d.fileCount)}</td>
                  </tr>
                ))}
                {result.dirNames.length === 0 && (
                  <tr><td colSpan={6}><div className="empty-state">暂无目录聚合数据</div></td></tr>
                )}
              </tbody>
            </table>
          )}

          {tab === "size-dist" && (
            <div><p className="muted">按文件体积汇总，条形长度表示占本次扫描的比例。</p>
              {result.sizeBuckets.map((b) => (
                <div className="bucket-row" key={b.label}>
                  <span className="lb">{b.label}</span>
                  <span className="track">
                    <div style={{ width: `${(b.bytes / maxBucket) * 100}%` }} />
                  </span>
                  <span className="vv">
                    {fmtNum(b.count)} 个 · {fmtBytes(b.bytes)} · {b.percent.toFixed(1)}%
                  </span>
                </div>
              ))}
            </div>
          )}

          {tab === "history" && (
            <div>
              {historyError ? <p className="feedback error" role="alert">{historyError}</p> : history.length === 0 ? (
                <div className="empty-state">还没有历史记录，完成两次以上扫描后可查看趋势。</div>
              ) : (
                <>
                  <p className="muted">同一目录、同一统计方式 · 柱形从 0 开始</p><div className="hist-bars">
                    {history.slice(-12).map((h, i) => (
                      <div className="hist-col" key={`${h.ts}-${i}`}>
                        <span className="vl">{fmtBytes(h.totalBytes)}</span>
                        <span className="bar" style={{ height: `${(h.totalBytes / maxHist) * 110}px` }} />
                        <span className="lb">{new Date(h.ts * 1000).toLocaleDateString("zh-CN", { month: "numeric", day: "numeric" })}</span>
                      </div>
                    ))}
                  </div>
                  <div style={{ marginTop: 14, fontSize: 12, color: "var(--text-2)" }}>
                    最近一次相比上次：
                    {history.length >= 2
                      ? (() => {
                          const cur = history[history.length - 1].totalBytes;
                          const prev = history[history.length - 2].totalBytes;
                          const delta = cur - prev;
                          const sign = delta >= 0 ? "+" : "-";
                          return (
                            <b style={{ color: delta >= 0 ? "var(--danger)" : "var(--success)" }}>
                              {" "}{sign}{fmtBytes(Math.abs(delta))}
                            </b>
                          );
                        })()
                      : " 需要至少两条记录"}
                  </div>
                </>
              )}
            </div>
          )}
          </div>
        </div>
      </div>
    </div>
  );
}

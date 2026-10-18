import { useState } from 'react';
import { useApp } from '../stores/appStore';
import { fmtBytes, fmtNum, fmtDuration, BigCheck } from '../components/ui';
import Illustration from '../components/Illustration';
import { bridge } from '../services/bridge';

export default function Done() {
  const summary = useApp(s => s.cleanSummary);
  const requested = useApp(s => s.cleanRequested);
  const failed = summary?.failures?.length ?? Math.max(0, requested - (summary?.files ?? 0));
  const go = useApp(s => s.go);
  const platform = useApp(s => s.platform);
  const record = useApp(s => s.lastCleanup);
  const [msg, setMsg] = useState('');
  const [busy, setBusy] = useState(false);
  const [details, setDetails] = useState(false);
  const trash = platform === 'windows' ? '回收站' : '废纸篓';
  const restore = async () => {
    setBusy(true); setMsg('');
    try {
      if (platform === 'windows') setMsg(await useApp.getState().restoreLastCleanup());
      else { await bridge.openTrash(); setMsg(`已打开${trash}，请选中文件并选择「放回原处」。`); }
    } catch (e) { setMsg(`操作失败：${String(e)}`); }
    finally { setBusy(false); }
  };
  return <div className="done-wrap">
    <div className="done-art"><Illustration name="storage" />{failed ? <span className="done-warning" aria-label="部分项目未处理">!</span> : <BigCheck />}</div>
    <h1 className="done-title">{failed ? "清理已结束" : "已完成清理"}</h1>
    <p className="done-sub">已成功处理的项目已移入{trash}</p>
    {failed > 0 && <p className="feedback error" role="alert">{failed} 项需要核对，可能未移动或记录未完整保存；请查看下方具体原因。</p>}
    {!!summary?.failures?.length && <details className="card" style={{textAlign: "left", maxWidth: "100%"}}><summary>查看未处理文件与原因（{summary.failures.length}）</summary><ul style={{maxHeight: 280, overflow: "auto", overflowWrap: "anywhere"}}>{summary.failures.map((f, i) => <li key={i}><strong>{f.path}</strong><p>{f.reason}</p></li>)}</ul></details>}
    <div className="done-stats"><div className="done-stat"><div className="v">{fmtBytes(summary?.movedBytes ?? summary?.freedBytes ?? 0)}</div><div className="l">已处理文件体积</div></div><div className="done-stat"><div className="v">{fmtNum(summary?.files ?? 0)}</div><div className="l">已处理项目</div></div><div className="done-stat"><div className="v">{fmtDuration(summary?.durationMs ?? 0)}</div><div className="l">用时</div></div></div>
    <p className="muted" style={{ marginTop: 16 }}>{trash}中的文件仍占用磁盘空间，清空后才会释放。</p>
    <div className="done-actions"><button className="btn btn-ghost" disabled={!record} onClick={() => setDetails(v => !v)}>查看清理记录</button><button className="btn btn-ghost" disabled={busy || (platform === 'windows' && !record)} onClick={restore}>{busy ? '处理中…' : platform === 'windows' ? '撤销本次清理' : `打开${trash}`}</button><button className="btn btn-primary" onClick={() => go('home')}>返回首页</button></div>
    {details && record && <div className="done-record card"><strong>最近一次清理记录</strong><ul>{record.items.map(({ path }) => <li key={path}>{path}</li>)}</ul></div>}
    {msg && <p className="feedback" role="status">{msg}</p>}
    <div className="done-tip"><span>按你的习惯，定期整理安全缓存。</span><button className="link-btn" onClick={() => go('settings')}>设置自动清理 →</button></div>
  </div>;
}

import ScanTaskStatus from "../components/ScanTaskStatus";
import { useMemo, useState } from 'react';
import { useApp } from '../stores/appStore';
import { fmtBytes, FileIco } from '../components/ui';
import { duplicateCleanup, keepersFor, type KeeperChoices } from '../services/duplicateSelection';

export default function Duplicates() {
  const groups = useApp(s => s.dupGroups);
  const loading = useApp(s => s.dupLoading);
  const error = useApp(s => s.dupError);
  const busy = useApp(s => s.cleaning);
  const cleanError = useApp(s => s.cleanError);
  const platform = useApp(s => s.platform);
  const load = useApp(s => s.loadDuplicates);
  const [cat, setCat] = useState('全部');
  const [kept, setKept] = useState<KeeperChoices>({});
  const trash = platform === 'windows' ? '回收站' : '废纸篓';
  const categories = useMemo(() => ['全部', ...new Set(groups.map(g => g.category).filter(c => c !== '相似')), '相似'], [groups]);
  const shown = groups.filter(g => cat === '全部' || (cat === '相似' ? g.kind === 'similar' : g.category === cat));
  const cleanup = duplicateCleanup(groups, shown, kept);
  const bytes = cleanup.reduce((n, f) => n + f.sizeBytes, 0);
  const rescan = () => { setKept({}); setCat('全部'); void load(); };
  return <div>
    <header className="page-head"><div><h1 className="page-title">重复文件</h1><p className="page-sub">整理多余副本，让每一份保留都有依据</p></div><button className="btn btn-ghost" disabled={loading || busy} onClick={rescan}>{loading ? '扫描中…' : groups.length ? '重新扫描' : '开始扫描'}</button></header>
    {(error || cleanError) && <p className="feedback error" role="alert">{error || cleanError}</p>}
    <ScanTaskStatus kind="duplicates" loading={loading} />
    {loading ? <div className="card empty-state"><div className="spin" /><p>正在比对文件内容，可能需要一些时间…</p></div> : !groups.length ? <div className="card empty-state">{error ? '扫描未完成，请重试。' : '点击「开始扫描」查找文件副本。扫描完成后，结果会显示在这里。'}</div> : <>
      <div className="card duplicate-summary"><div><strong>{shown.length}</strong><span>文件组</span></div><div><strong>{cleanup.length}</strong><span>待整理副本</span></div><div><strong>{fmtBytes(bytes)}</strong><span>待处理文件体积</span></div></div>
      <div className="junk-tabs">{categories.map(c => <button className={`junk-tab ${cat === c ? 'active' : ''}`} aria-pressed={cat === c} key={c} onClick={() => setCat(c)}>{c}</button>)}</div>
      <p className="muted" style={{ marginTop: 10 }}>完全相同的文件默认保留最新一份；相似文件默认全部保留，请逐组核对。</p>
      <div className="duplicate-groups">{shown.map(g => {
        const keep = keepersFor(g, kept);
        return <section className="duplicate-group" key={g.id} aria-label={`${g.files[0]?.name} 文件组`}>
          <header><FileIco name={g.files[0]?.name ?? ''} /><strong>{g.files[0]?.name}</strong><span className={g.kind === 'similar' ? 'similarity-badge' : 'kind-badge'}>{g.kind === 'similar' ? `相似 ${Math.round(g.similarity * 100)}% · 需核对` : '内容相同'}</span><span className="muted">{g.files.length} 份 · {fmtBytes(g.sizeBytes)} / 份</span><button className="link-btn" disabled={busy} onClick={() => setKept(v => ({ ...v, [g.id]: g.files.map(f => f.path) }))}>本组全部保留</button></header>
          <div style={{ overflowX: 'auto' }}><table className="table"><thead><tr><th>文件位置</th><th className="num">大小</th><th>修改时间</th><th>保留副本</th></tr></thead><tbody>{g.files.map(f => <tr key={f.path} className={keep.has(f.path) ? 'selected' : ''}><td className="path-cell" title={f.path}>{f.path}{f.protectedReason && <span className="similarity-badge" title={f.protectedReason}>受保护</span>}</td><td className="num">{fmtBytes(f.sizeBytes)}</td><td className="muted">{f.modified}</td><td><button className="keep-button" aria-pressed={keep.has(f.path)} aria-label={`${keep.has(f.path) ? '已保留' : '设为保留'} ${f.path}`} disabled={busy} onClick={() => setKept(v => ({ ...v, [g.id]: [f.path] }))}>{keep.has(f.path) ? '✓ 保留' : '设为保留'}</button></td></tr>)}</tbody></table></div>
        </section>;
      })}</div>
      {!shown.length && <div className="empty-state">此分类下没有文件组</div>}
      <footer className="footer-bar"><div><strong>待整理 {cleanup.length} 份 · {fmtBytes(bytes)}</strong><p className="muted">每组至少保留一份，其余移入{trash}</p></div><div className="head-actions"><button className="btn btn-ghost" disabled={busy} onClick={() => setKept(v => ({ ...v, ...Object.fromEntries(shown.map(g => [g.id, g.files.map(f => f.path)])) }))}>全部保留</button><button className="btn btn-primary" disabled={busy || !cleanup.length} onClick={() => void useApp.getState().cleanSelected(cleanup.map(f => f.path))}>{busy ? '正在处理…' : `移入${trash}`}</button></div></footer>
    </>}
  </div>;
}

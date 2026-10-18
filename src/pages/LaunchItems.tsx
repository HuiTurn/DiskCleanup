import Illustration from "../components/Illustration";
import { useEffect, useState } from 'react';
import { useApp } from '../stores/appStore';
import { bridge } from '../services/bridge';
import type { LaunchItem } from '../types';
import { IconPower, IconRefresh } from '../components/Icons';

export default function LaunchItems() {
  const platform = useApp(s => s.platform);
  const [items, setItems] = useState<LaunchItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState(false);
  const [error, setError] = useState('');
  const windows = platform === 'windows';
  const reload = async () => {
    setLoading(true); setError('');
    try { setItems(await bridge.getLaunchItems()); }
    catch (e) { setError(`读取失败：${String(e)}`); }
    finally { setLoading(false); }
  };
  useEffect(() => { void reload(); }, []);
  const open = async () => {
    setOpening(true); setError('');
    try { await bridge.runSystemTool(windows ? 'startup' : 'login-items'); }
    catch (e) { setError(`打开失败：${String(e)}`); }
    finally { setOpening(false); }
  };
  return <div>
    <header className="page-head"><div><h1 className="page-title">启动项管理</h1><p className="page-sub">查看开机启动的程序，按需管理</p></div><div className="head-actions"><button className="btn btn-ghost" disabled={loading} onClick={reload}><IconRefresh size={15} />刷新</button><button className="btn btn-primary" disabled={opening} onClick={open}>{opening ? '打开中…' : windows ? '打开任务管理器' : '打开系统设置'}</button></div></header>
    <section className="feature-banner"><Illustration name="startup" /><div><h2>让开机更从容</h2><p>减少不必要的启动程序，让常用应用优先就绪。</p></div><div className="banner-meta"><b>{loading ? '—' : items.length}</b>已读取启动项</div></section>
    {error && <p className="feedback error" role="alert">{error}</p>}
    <div className="scroll-table">{loading ? <div className="empty-state"><div className="spin" /><p>正在读取启动项…</p></div> : !items.length ? <div className="empty-state">{error ? '请重试读取启动项' : '未读取到启动项'}<p className="muted">可在{windows ? '任务管理器的「启动应用」' : '系统设置 → 通用 → 登录项'}中查看完整列表。</p></div> : <table className="table"><thead><tr><th>应用名称</th><th>信息</th></tr></thead><tbody>{items.map((it, i) => <tr key={it.name + i}><td><div className="file-name"><span className="launch-icon"><IconPower size={19} /></span><strong>{it.name}</strong></div></td><td><span className="tag neutral">已读取</span></td></tr>)}</tbody></table>}</div>
    <p className="muted" style={{ marginTop: 16 }}>启用、停用与完整状态请在系统中管理。</p>
  </div>;
}

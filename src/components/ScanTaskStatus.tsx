import { useEffect, useState } from 'react';
import { bridge } from '../services/bridge';
import type { TaskStatus } from '../types';

export default function ScanTaskStatus({ kind, loading }: { kind: string; loading: boolean }) {
  const [status, setStatus] = useState<TaskStatus | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    let disposed = false;
    const refresh = () => bridge.getTaskStatus(kind).then(s => { if (!disposed) setStatus(s); }).catch(e => { if (!disposed) setError(String(e)); });
    void refresh();
    const timer = setInterval(refresh, 600);
    return () => { disposed = true; clearInterval(timer); };
  }, [kind, loading]);
  if ((!status || (!loading && !status.skipped && !status.notice)) && !error) return null;
  return <div className="feedback" style={{overflowWrap: "anywhere"}} role="status">
    {loading && <><span>已检查 {status?.visited ?? 0} 项 · {status?.currentPath || '正在准备扫描'}</span> <button className="btn btn-ghost" disabled={status?.cancelled} onClick={() => bridge.cancelTask(kind).catch(e => setError(String(e)))}>{status?.cancelled ? '正在取消…' : '取消扫描'}</button></>}
    {!!status?.skipped && <p>有 {status.skipped} 项因权限、文件变化或保护规则未计入结果。</p>}
    {status?.notice && <p>{status.notice}</p>}
    {error && <p role="alert">进度读取失败：{error}</p>}
  </div>;
}

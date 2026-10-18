import Illustration from '../components/Illustration';
import { useApp } from '../stores/appStore';
import { fmtBytes } from '../components/ui';
import { IconBroom, IconFile, IconCopy, IconMove, IconDrive, IconArrow } from '../components/Icons';

const ENTRIES = [
  { page: 'junk' as const, t: '垃圾清理', d: '整理缓存与临时文件', tone: 'blue', Icon: IconBroom },
  { page: 'large-files' as const, t: '大文件管理', d: '找出占空间的大文件', tone: 'amber', Icon: IconFile },
  { page: 'file-migration' as const, t: '文件迁移', d: '将常用目录移至其他盘', tone: 'teal', Icon: IconMove },
  { page: 'duplicates' as const, t: '重复文件', d: '整理多余的文件副本', tone: 'blue', Icon: IconCopy },
];
export default function Home() {
  const disks = useApp(s => s.disks);
  const platform = useApp(s => s.platform);
  const error = useApp(s => s.scanError);
  const go = useApp(s => s.go);
  const startScan = useApp(s => s.startScan);
  const sys = disks.find(d => d.isSystem) ?? disks[0];
  const pct = sys && sys.totalBytes > 0 ? Math.min(100, Math.max(0, sys.usedBytes / sys.totalBytes * 100)) : 0;
  return <div className="home-page">
    <div className="eyebrow">首页</div>
    <section className="home-hero">
      <div className="hero-copy">
        <h1>给电脑减减负<br />让<span className="accent">空间</span>更从容</h1>
        <p className="hero-desc">分析磁盘占用，整理文件与系统缓存。</p>
        <div className="home-cta"><button className="btn btn-primary btn-lg" onClick={() => void startScan()}>开始磁盘分析</button><span>先分析，再选择要清理的内容。</span></div>
      </div>
      <Illustration name="storage" className="hero-art" />
    </section>
    {error && <p className="feedback error" role="alert">{error}</p>}
    {sys ? <section className="disk-card" aria-label="系统磁盘空间">
      <span className="disk-symbol" aria-hidden="true"><IconDrive size={30} /></span>
      <div className="disk-info"><div className="disk-heading"><h2>{sys.name}{sys.isSystem && ' · 系统盘'}</h2><span className="tag neutral">{fmtBytes(sys.totalBytes)}</span><strong className="disk-pct">{Math.round(pct)}%</strong></div>
      <div className={`usage-bar ${pct >= 90 ? 'critical' : pct >= 70 ? 'high' : ''}`} role="meter" aria-label="磁盘已用空间" aria-valuenow={Math.round(pct)} aria-valuemin={0} aria-valuemax={100}><div style={{ width: `${pct}%` }} /></div>
      <div className="disk-meta"><span>已使用 <b>{fmtBytes(sys.usedBytes)}</b></span><span>可用 <b>{fmtBytes(Math.max(0, sys.totalBytes - sys.usedBytes))}</b></span></div></div>
    </section> : <div className="card empty-state">{platform === 'unknown' ? '正在读取磁盘信息…' : '未读取到磁盘信息'}</div>}
    <p className="home-note">空间信息来自系统。</p>
    <h2 className="section-heading">常用功能</h2>
    <div className="entry-grid">{ENTRIES.map(e => <button key={e.page} className="entry-card" onClick={() => go(e.page)}><span className={`entry-ico tone-${e.tone}`}><e.Icon size={27} /></span><span className="t">{e.t}</span><span className="d">{e.d}</span><span className="entry-arrow" aria-hidden="true"><IconArrow size={18} /></span></button>)}</div>
    <footer className="app-footer"><span>v0.1.0</span><span>{platform === 'macos' ? 'macOS' : platform === 'windows' ? 'Windows' : platform === 'unknown' ? '读取平台中…' : platform}</span></footer>
  </div>;
}

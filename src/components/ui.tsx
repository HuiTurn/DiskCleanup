import type { ReactNode } from "react";
import { IconCheck, IconFolder, IconFile, IconImage, IconVideo, IconAudio, IconArchive, IconApp, IconSettings } from "./Icons";

/** 字节 -> 人类可读 */
export function fmtBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  if (n >= 1024 * 1024 * 1024 * 1024) return `${(n / 1024 ** 4).toFixed(2)} TB`;
  if (n >= 1024 * 1024 * 1024) return `${(n / 1024 ** 3).toFixed(1)} GB`;
  if (n >= 1024 * 1024) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${n} B`;
}

export function fmtNum(n: number): string {
  return n.toLocaleString("en-US");
}

export function fmtDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} 秒`;
  return `${Math.floor(s / 60)} 分钟`;
}

export function Toggle({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <button className={`toggle ${on ? "on" : ""}`} onClick={() => onChange(!on)} role="switch" aria-checked={on} aria-label={label} />
  );
}

export function TabBar({
  tabs,
  active,
  onChange,
}: {
  tabs: { id: string; label: string }[];
  active: string;
  onChange: (id: string) => void;
}) {
  return (
    <div className="tabbar" role="group" aria-label="视图筛选">
      {tabs.map((t) => (
        <button
          key={t.id}
          className={`tab ${active === t.id ? "active" : ""}`}
          aria-pressed={active === t.id}
          onClick={() => onChange(t.id)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}

/** 环形进度（扫描中 68%） */
export function ProgressRing({
  percent,
  indeterminate = false,
  size = 190,
  stroke = 16,
  label,
}: {
  percent: number;
  indeterminate?: boolean;
  size?: number;
  stroke?: number;
  label?: ReactNode;
}) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const off = c * (1 - Math.min(percent, 100) / 100);
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
      <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--border)" strokeWidth={stroke} />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={r}
        fill="none"
        stroke="url(#ringGrad)"
        strokeWidth={stroke}
        strokeLinecap="round"
        className={indeterminate ? "scan-orbit" : undefined}
        strokeDasharray={indeterminate ? `${c * .24} ${c * .76}` : c}
        strokeDashoffset={indeterminate ? 0 : off}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
        style={{ transition: "stroke-dashoffset 0.2s" }}
      />
      <defs>
        <linearGradient id="ringGrad" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#4a90ff" />
          <stop offset="100%" stopColor="#2e7bf6" />
        </linearGradient>
      </defs>
      <foreignObject x={0} y={0} width={size} height={size}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
          {label}
        </div>
      </foreignObject>
    </svg>
  );
}

const CAT_COLORS = ["#2f7aff", "#ff655d", "#19c990", "#ffc13b", "#ff973e", "#9c77e8", "#a3a0cc"];

/** 分类甜甜圈图（分析结果页） */
export function DonutChart({
  data,
  size = 210,
  center,
}: {
  data: { name: string; percent: number }[];
  size?: number;
  center?: ReactNode;
}) {
  const stroke = 34;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  let acc = 0;
  return (
    <div className="donut-wrap" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--border)" strokeWidth={stroke} />
        {data.map((d, i) => {
          const frac = Math.max(0, Math.min(d.percent / 100, 1));
          const dash = `${Math.max(0, c * frac - 2)} ${c - Math.max(0, c * frac - 2)}`;
          const el = (
            <circle
              key={d.name}
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke={CAT_COLORS[i % CAT_COLORS.length]}
              strokeWidth={stroke}
              strokeDasharray={dash}
              strokeDashoffset={-c * acc}
              transform={`rotate(-90 ${size / 2} ${size / 2})`}
            />
          );
          acc += frac;
          return el;
        })}
      </svg>
      {center && <div className="donut-center">{center}</div>}
    </div>
  );
}

export function LegendColor(i: number): string {
  return CAT_COLORS[i % CAT_COLORS.length];
}

/** 完成页大对勾 */
export function BigCheck() {
  return (
    <svg className="done-check" viewBox="0 0 96 96">
      <circle cx="48" cy="48" r="44" fill="#22c55e" opacity="0.12" />
      <circle cx="48" cy="48" r="34" fill="#22c55e" />
      <path d="M34 49 45 59 63 39" fill="none" stroke="#fff" strokeWidth="6" strokeLinecap="round" strokeLinejoin="round" />
      {/* 彩色装饰点（截图 05 的彩带感） */}
      <circle cx="14" cy="22" r="3" fill="#f59e0b" />
      <circle cx="84" cy="30" r="2.5" fill="#2e7bf6" />
      <circle cx="78" cy="80" r="3" fill="#7b61ff" />
      <circle cx="20" cy="76" r="2.5" fill="#31c3ff" />
    </svg>
  );
}

/** 文件类型图标色块 */
export function FileIco({ name, size }: { name: string; size?: number }) {
  const ext = name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
  if (!ext) return <span className="folder-ico"><IconFolder size={size ?? 24} color="#ffb522" /></span>;
  let color = "#7185a5";
  let Icon = IconFile;
  if (["jpg", "png", "jpeg", "gif", "webp", "heic"].includes(ext)) { color = "#21a77c"; Icon = IconImage; }
  else if (["mp4", "mov", "avi", "mkv"].includes(ext)) { color = "#e76e74"; Icon = IconVideo; }
  else if (["mp3", "wav", "flac", "m4a"].includes(ext)) { color = "#8a6bd1"; Icon = IconAudio; }
  else if (["zip", "rar", "7z", "tar", "gz", "dmg"].includes(ext)) { color = "#ce922d"; Icon = IconArchive; }
  else if (["pdf", "doc", "docx", "xls", "xlsx", "pptx"].includes(ext)) { color = "#3b82eb"; }
  else if (["exe", "msi", "iso", "pkg"].includes(ext)) { color = "#5483d4"; Icon = IconApp; }
  else if (["dll", "so", "dylib", "bin", "app"].includes(ext)) { Icon = IconSettings; }
  return <span className="file-type-icon" style={{ color, width: size ?? 30, height: size ?? 30 }} title={ext.toUpperCase()}><Icon size={size ? Math.max(16, size - 2) : 22} /></span>;
}

export function SectionCheck({ ok }: { ok: boolean }) {
  return ok ? <IconCheck size={13} color="#22c55e" /> : null;
}

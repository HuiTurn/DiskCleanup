import ScanTaskStatus from "../components/ScanTaskStatus";
import Illustration from "../components/Illustration";
import { useEffect, useRef, useState } from "react";
import { bridge } from "../services/bridge";
import { IconCheck, IconFolder } from "../components/Icons";
import { fmtBytes } from "../components/ui";
import type {
  MigrationOptions,
  MigrationPlan,
  MigrationStatus,
} from "../types";
import "../styles/migration.css";

export default function FileMigration() {
  const [options, setOptions] = useState<MigrationOptions | null>(null);
  const [selected, setSelected] = useState("");
  const [drive, setDrive] = useState("");
  const [status, setStatus] = useState<MigrationStatus | null>(null);
  const [error, setError] = useState("");
  const [elapsed, setElapsed] = useState(0);
  const [checking, setChecking] = useState(false);
  const [starting, setStarting] = useState(false);
  const [plan, setPlan] = useState<MigrationPlan | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const checkRef = useRef<HTMLButtonElement>(null);
  const busy =
    starting || status?.phase === "checking" || status?.phase === "moving";
  useEffect(() => {
    if (!busy) { setElapsed(0); return; }
    const start=Date.now(); const timer=setInterval(() => setElapsed(Math.floor((Date.now()-start)/1000)), 1000);
    return () => clearInterval(timer);
  }, [busy]);
  const locked = busy || checking;
  const folder = options?.folders.find((item) => item.id === selected);
  const targets =
    options?.drives.filter(
      (item) =>
        item.path.slice(0, 2).toLowerCase() !==
        folder?.path.slice(0, 2).toLowerCase(),
    ) ?? [];
  const target = targets.find((item) => item.path === drive);

  const load = async () => {
    try {
      const value = await bridge.getMigrationOptions();
      setOptions(value);
    } catch (e) {
      setError(String(e));
    }
  };
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const current = await bridge.getMigrationStatus();
        if (!disposed) setStatus(current);
      } catch (e) {
        if (!disposed) setError(`无法获取迁移状态：${String(e)}`);
      }
      if (!disposed) timer = setTimeout(poll, 1200);
    };
    void load();
    void poll();
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, []);
  useEffect(() => {
    if (status?.phase === "complete" || status?.phase === "failed") void load();
  }, [status?.phase]);
  useEffect(() => {
    if (plan) confirmRef.current?.focus();
  }, [plan]);

  const check = async () => {
    if (!folder || !target) return;
    setChecking(true);
    setError("");
    setAcknowledged(false);
    try {
      setPlan(
        await bridge.prepareMigration({
          folderId: folder.id,
          targetDrive: target.path,
          expectedSource: folder.path,
        }),
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };
  const migrate = async () => {
    if (!plan || !acknowledged || busy) return;
    const request = plan.request;
    setStarting(true);
    setPlan(null);
    setError("");
    try {
      setStatus(await bridge.migrateFolder(request));
    } catch (e) {
      setError(String(e));
    } finally {
      setStarting(false);
      void load();
    }
  };
  const closeConfirm = () => {
    setPlan(null);
    checkRef.current?.focus();
  };

  return (
    <div className="migration-page">
      {checking && <ScanTaskStatus kind="migration-check" loading={checking} />}
      {busy && <p className="feedback">已运行 {elapsed} 秒。{elapsed >= 120 ? '若长时间未变化，请查看 Windows 迁移窗口和日志；不要强制退出或删除任一目录。' : '正在检查、移动或校验文件，请保持窗口打开。'}</p>}
      <header className="page-head">
        <div>
          <h1 className="page-title">文件迁移</h1>
          <p className="page-sub">把常用目录搬到其他盘，为系统盘腾出空间</p>
        </div>
        <span className="migration-platform">Windows 系统目录</span>
      </header>
      {error && (
        <div className="migration-feedback error" role="alert">
          {error}
          {!options && (
            <button className="link-btn" onClick={load}>
              重试
            </button>
          )}
        </div>
      )}
      {!options && !error && (
        <div className="empty-state">
          <div className="spin" />
          <p>正在读取系统目录…</p>
        </div>
      )}
      {options && !options.supported && (
        <div className="migration-unavailable card">
          <span className="migration-hero-icon">
            <Illustration name="migration" />
          </span>
          <h2>Windows 系统目录迁移</h2>
          <p>
            支持将桌面、文档、下载、图片、音乐和视频迁移到其他本地磁盘，并更新系统目录位置。
          </p>
          <div className="migration-folder-tags">
            {["桌面", "文档", "下载", "图片", "音乐", "视频"].map((name) => (
              <span key={name}>
                <IconFolder size={16} />
                {name}
              </span>
            ))}
          </div>
          <p className="migration-platform-note">
            当前平台暂不支持此功能，请在 Windows 版中使用。
          </p>
        </div>
      )}
      {options?.supported && (
        <>
          <div className="migration-intro">
            <Illustration name="migration" />
            <div>
              <strong>文件搬家，使用习惯不变</strong>
              <p>迁移完成后，Windows 和应用将使用新位置。每次迁移一个目录。</p>
            </div>
          </div>
          <ol className="migration-steps" aria-label="迁移步骤">
            <li className="active">
              <span>1</span>选择目录
            </li>
            <li className={folder ? "active" : ""}>
              <span>2</span>选择目标盘
            </li>
            <li className={plan || busy ? "active" : ""}>
              <span>3</span>检查并迁移
            </li>
          </ol>
          {status?.phase && (
            <div
              className={`migration-feedback ${status.phase === "failed" ? "error" : ""}`}
              role="status"
            >
              <strong>
                {busy
                  ? "迁移进行中"
                  : status.phase === "complete"
                    ? "迁移完成"
                    : "迁移未完成"}
              </strong>
              <p>{status.message}</p>
              {busy && (
                <>
                  <div className="migration-indeterminate">
                    <span />
                  </div>
                  <p>文件复制进度由 Windows 显示；完成前无法退出应用。</p>
                </>
              )}
              {status.source && (
                <p className="migration-path">原目录：{status.source}</p>
              )}
              {status.target && (
                <p className="migration-path">目标：{status.target}</p>
              )}
              {status.journal && (
                <p className="migration-path">迁移记录：{status.journal}</p>
              )}
            </div>
          )}
          <section className="migration-section">
            <h2>选择要迁移的目录</h2>
            <div
              className="migration-folders"
              role="group"
              aria-label="系统目录"
            >
              {options.folders.map((item) => (
                <button
                  key={item.id}
                  className={`migration-folder ${selected === item.id ? "selected" : ""}`}
                  aria-pressed={selected === item.id}
                  disabled={locked || !!item.blockedReason}
                  title={item.blockedReason || item.path}
                  onClick={() => {
                    setSelected(item.id);
                    setDrive("");
                    setPlan(null);
                    setError("");
                  }}
                >
                  <span className="migration-folder-heading">
                    <span className="migration-folder-icon">
                      <IconFolder size={24} />
                    </span>
                    <strong>{item.name}</strong>
                    <span className="migration-radio">
                      {selected === item.id && <IconCheck size={12} />}
                    </span>
                  </span>
                  <span className="migration-folder-path">
                    {item.path || "无法读取目录位置"}
                  </span>
                  {item.blockedReason && (
                    <span className="migration-blocked">
                      {item.blockedReason}
                    </span>
                  )}
                </button>
              ))}
            </div>
          </section>
          <section className="migration-section">
            <h2>选择目标盘</h2>
            <div className="card migration-destination">
              <div className="migration-target-row">
                <div>
                  <strong>迁移到其他本地磁盘</strong>
                  <p>自动建立独立目录，不合并或覆盖已有文件夹。</p>
                </div>
                <select
                  aria-label="目标盘"
                  value={target ? drive : ""}
                  disabled={!folder || locked}
                  onChange={(e) => {
                    setDrive(e.target.value);
                    setPlan(null);
                    setError("");
                  }}
                >
                  <option value="">选择目标盘</option>
                  {targets.map((disk) => (
                    <option key={disk.path} value={disk.path}>
                      {disk.path} · 可用 {fmtBytes(disk.availableBytes)}
                    </option>
                  ))}
                </select>
              </div>
              {folder && !targets.length && (
                <p className="migration-blocked">
                  未发现其他可用的本地固定磁盘。
                </p>
              )}
              <div className="migration-preview">
                <span>当前位置</span>
                <span className="migration-path">
                  {folder?.path || "请先选择一个目录"}
                </span>
                <span>迁移后位置</span>
                <span>检查后显示完整路径</span>
              </div>
            </div>
          </section>
          <div className="migration-bottom">
            <p>
              迁移前请关闭正在使用该目录的应用。
              <br />
              OneDrive 托管目录、网络盘和移动磁盘暂不支持。
            </p>
            <button
              ref={checkRef}
              className="btn btn-primary"
              disabled={!folder || !target || locked}
              onClick={check}
            >
              {checking
                ? "正在检查文件与空间…"
                : busy
                  ? "迁移中…"
                  : "检查并继续"}
            </button>
          </div>
        </>
      )}
      {plan && (
        <div className="modal-mask">
          <div
            className="modal migration-confirm"
            role="dialog"
            aria-modal="true"
            aria-labelledby="migration-confirm-title"
            onKeyDown={(e) => {
              if (e.key === "Escape") closeConfirm();
              if (e.key === "Tab") {
                const items = [
                  ...e.currentTarget.querySelectorAll<HTMLElement>(
                    "button:not(:disabled), input:not(:disabled)",
                  ),
                ];
                const first = items[0];
                const last = items[items.length - 1];
                if (e.shiftKey && document.activeElement === first) {
                  e.preventDefault();
                  last?.focus();
                } else if (!e.shiftKey && document.activeElement === last) {
                  e.preventDefault();
                  first?.focus();
                }
              }
            }}
          >
            <h2 id="migration-confirm-title">确认迁移「{plan.name}」</h2>
            <p className="migration-confirm-sub">
              检查通过 · {plan.fileCount.toLocaleString()} 个文件 ·{" "}
              {fmtBytes(plan.totalBytes)}
            </p>
            <dl className="migration-confirm-paths">
              <dt>原位置</dt>
              <dd>{plan.source}</dd>
              <dt>新位置</dt>
              <dd>{plan.target}</dd>
              <dt>目标盘可用</dt>
              <dd>{fmtBytes(plan.availableBytes)}</dd>
            </dl>
            <p>
              Windows
              将移动现有文件并更新系统目录位置。若中途失败，请按迁移记录核对两处文件。
            </p>
            <label className="migration-ack">
              <input
                type="checkbox"
                checked={acknowledged}
                onChange={(e) => setAcknowledged(e.target.checked)}
              />
              已关闭使用该目录的应用，确认迁移到上述位置
            </label>
            <div className="modal-actions">
              <button
                ref={confirmRef}
                className="btn btn-ghost"
                onClick={closeConfirm}
              >
                返回修改
              </button>
              <button
                className="btn btn-primary"
                disabled={!acknowledged || busy}
                onClick={migrate}
              >
                开始迁移
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

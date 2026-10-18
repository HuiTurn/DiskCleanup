import appLogo from "../assets/brand/app-logo-v2.png";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { useApp } from "../stores/appStore";
import { Toggle } from "../components/ui";
import { IconCheck, IconFolder, IconShield } from "../components/Icons";
import { bridge } from "../services/bridge";
import type { Settings } from "../types";
import "../styles/settings.css";

const tabs = [
  { id: "general", label: "常规" },
  { id: "rules", label: "清理规则" },
  { id: "exclusions", label: "排除目录" },
  { id: "about", label: "关于" },
] as const;
type Tab = (typeof tabs)[number]["id"];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="settings-section">
      <div className="settings-group"><h2 className="settings-group-title">{title}</h2>{children}</div>
    </section>
  );
}
function Row({
  title,
  description,
  children,
}: {
  title: string;
  description?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="settings-row">
      <div className="settings-description">
        <div className="settings-label">{title}</div>
        {description && <div className="settings-hint">{description}</div>}
      </div>
      {children}
    </div>
  );
}
function Notice({ children }: { children: ReactNode }) {
  return (
    <div className="settings-notice">
      <span aria-hidden="true" className="settings-info">
        i
      </span>
      <span>{children}</span>
    </div>
  );
}

export default function SettingsPage() {
  const settings = useApp((s) => s.settings);
  const platform = useApp((s) => s.platform);
  const systemDisk = useApp((s) => s.disks.find((disk) => disk.isSystem));
  const lastCleanup = useApp((s) => s.lastCleanup);
  const [tab, setTab] = useState<Tab>("general");
  const [excludeInput, setExcludeInput] = useState("");
  const [saveState, setSaveState] = useState("设置已加载");
  const [error, setError] = useState("");
  const [directoryError, setDirectoryError] = useState("");
  const [busy, setBusy] = useState(false);
  const [picking, setPicking] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [autoCleanMsg, setAutoCleanMsg] = useState("");
  const [restoreMsg, setRestoreMsg] = useState("");
  const saveQueue = useRef(Promise.resolve(true));
  const pendingSaves = useRef(0);

  useEffect(() => {
    if (!settings)
      useApp
        .getState()
        .init()
        .catch((e) => setError(String(e)));
  }, [settings]);

  // Serialize partial updates against the latest saved state so rapid edits cannot overwrite each other.
  const patch = (
    change: Partial<Settings> | ((current: Settings) => Partial<Settings>),
  ) => {
    pendingSaves.current += 1;
    setSaveState("正在保存…");
    const task = saveQueue.current.then(async () => {
      const current = useApp.getState().settings;
      if (!current) {
        pendingSaves.current -= 1;
        setSaveState("未保存");
        return false;
      }
      const result = await useApp
        .getState()
        .saveSettings({
          ...current,
          ...(typeof change === "function" ? change(current) : change),
        });
      pendingSaves.current -= 1;
      if (result) {
        setError(`保存失败：${result}`);
        setSaveState("未保存");
        return false;
      }
      setError("");
      if (!pendingSaves.current) setSaveState("已自动保存");
      return true;
    });
    saveQueue.current = task;
    return task;
  };

  const openTrash = async () => {
    setRestoreMsg("");
    try {
      await bridge.openTrash();
    } catch (e) {
      setRestoreMsg(`打开失败：${String(e)}`);
    }
  };
  const runAutoClean = async () => {
    setBusy(true);
    setAutoCleanMsg("");
    try {
      if (!(await saveQueue.current)) {
        setAutoCleanMsg("请先成功保存设置，再执行清理。");
        return;
      }
      const summary = await useApp.getState().runAutoCleanNow();
      setAutoCleanMsg(`已移入回收站／废纸篓 ${((summary.movedBytes ?? 0) / 1024 ** 3).toFixed(2)} GB（不代表磁盘空间已释放）。${summary.failures?.length ? ` ${summary.failures.length} 项需要核对：${summary.failures.map(f => `${f.path}：${f.reason}`).join("；")}` : ""}`);
    } catch (e) {
      setAutoCleanMsg(`执行失败：${String(e)}`);
    } finally {
      setBusy(false);
    }
  };
  const addDirectory = async () => {
    const path = excludeInput.trim();
    if (!path) return;
    if (!(
      path.startsWith("/") ||
      /^[A-Za-z]:[\\/]/.test(path) ||
      path.startsWith("\\\\")
    )) {
      setDirectoryError("请输入完整的绝对路径");
      return;
    }
    if (useApp.getState().settings?.excludeDirs.includes(path)) {
      setDirectoryError("该目录已在排除列表中");
      return;
    }
    setDirectoryError("");
    if (
      await patch((current) => ({
        excludeDirs: [...new Set([...current.excludeDirs, path])],
      }))
    ) {
      setExcludeInput((value) => (value.trim() === path ? "" : value));
    }
  };
  const pickDirectory = async () => {
    setPicking(true);
    setDirectoryError("");
    try {
      const path = await bridge.pickDirectory();
      if (path) setExcludeInput(path);
    } catch (e) {
      setDirectoryError(`选择失败：${String(e)}`);
    } finally {
      setPicking(false);
    }
  };

  if (!settings)
    return (
      <div className="empty-state">{error || <div className="spin" />}</div>
    );
  const trash = platform === "windows" ? "回收站" : "废纸篓";
  const platformName =
    platform === "macos"
      ? "macOS"
      : platform === "windows"
        ? "Windows"
        : platform;
  const restoreButton = (
    <button className="btn btn-ghost" onClick={openTrash}>
      打开{trash}
    </button>
  );

  return (
    <div className="settings-page">
      <header className="settings-header">
        <h1>设置</h1>
        <p>管理应用偏好与清理规则</p>
      </header>
      <div className="settings-tabs" role="tablist" aria-label="设置分类">
        {tabs.map((item, index) => (
          <button
            key={item.id}
            id={`settings-tab-${item.id}`}
            role="tab"
            aria-selected={tab === item.id}
            aria-controls={`settings-panel-${item.id}`}
            tabIndex={tab === item.id ? 0 : -1}
            onClick={() => setTab(item.id)}
            onKeyDown={(e) => {
              let next = index;
              if (e.key === "ArrowRight") next = (index + 1) % tabs.length;
              else if (e.key === "ArrowLeft")
                next = (index + tabs.length - 1) % tabs.length;
              else if (e.key === "Home") next = 0;
              else if (e.key === "End") next = tabs.length - 1;
              else return;
              e.preventDefault();
              setTab(tabs[next].id);
              document.getElementById(`settings-tab-${tabs[next].id}`)?.focus();
            }}
          >
            {item.label}
          </button>
        ))}
      </div>
      {error && (
        <div className="settings-error" role="alert">
          {error}
        </div>
      )}
      <div
        className="settings-panel"
        role="tabpanel"
        id={`settings-panel-${tab}`}
        aria-labelledby={`settings-tab-${tab}`}
        tabIndex={0}
      >
        {tab === "general" && (
          <>
            <Section title="启动与退出">
              <Row title="开机自动启动" description="登录系统后自动运行工具">
                <Toggle
                  label="开机自动启动"
                  on={settings.launchAtLogin}
                  onChange={(value) => {
                    void patch({ launchAtLogin: value });
                  }}
                />
              </Row>
              <Row title="退出前确认" description="关闭窗口前提醒确认">
                <Toggle
                  label="退出前确认"
                  on={settings.closeConfirm}
                  onChange={(value) => {
                    void patch({ closeConfirm: value });
                  }}
                />
              </Row>
            </Section>
            <Section title="语言与外观">
              <Row title="界面语言">
                <select
                  aria-label="界面语言"
                  value={settings.language}
                  onChange={(e) => {
                    void patch({ language: e.target.value });
                  }}
                >
                  <option value="zh-CN">简体中文</option>
                </select>
              </Row>
              <Row title="外观" description="选择你喜欢的界面风格">
                <div
                  className="settings-appearances"
                  role="group"
                  aria-label="外观"
                >
                  {(
                    [
                      ["light", "浅色"],
                      ["dark", "深色"],
                      ["system", "跟随系统"],
                    ] as const
                  ).map(([value, label]) => (
                    <button
                      key={value}
                      className="appearance-option"
                      aria-pressed={settings.theme === value}
                      onClick={() => {
                        void patch({ theme: value });
                      }}
                    >
                      <span
                        className={`appearance-preview preview-${value}`}
                        aria-hidden="true"
                      >
                        <span className="preview-sidebar">
                          <i />
                          <i />
                          <i />
                          <i />
                        </span>
                        <span className="preview-content">
                          <i />
                          <i />
                          <i />
                        </span>
                        {settings.theme === value && (
                          <span className="appearance-check">
                            <IconCheck size={12} />
                          </span>
                        )}
                      </span>
                      <span>{label}</span>
                    </button>
                  ))}
                </div>
              </Row>
            </Section>
            <Section title="清理恢复">
              <Row
                title={`${trash}恢复`}
                description={`已清理的文件可在${trash}中找回`}
              >
                <div className="settings-actions">
                  {restoreButton}
                  {platform === "windows" && (
                    <button
                      className="btn btn-primary"
                      disabled={!lastCleanup || restoring}
                      onClick={async () => {
                        setRestoring(true);
                        try {
                          setRestoreMsg(
                            await useApp.getState().restoreLastCleanup(),
                          );
                        } finally {
                          setRestoring(false);
                        }
                      }}
                    >
                      {restoring ? "恢复中…" : "撤销清理"}
                    </button>
                  )}
                </div>
              </Row>
            </Section>
          </>
        )}
        {tab === "rules" && (
          <>
            <Section title="文件筛选">
              <Row
                title="大文件阈值"
                description="大文件管理中默认显示的最小文件大小"
              >
                <select
                  aria-label="大文件阈值"
                  value={settings.largeFileMinMb}
                  onChange={(e) => {
                    void patch({ largeFileMinMb: Number(e.target.value) });
                  }}
                >
                  {[
                    [10, "10 MB"],
                    [100, "100 MB"],
                    [500, "500 MB"],
                    [1024, "1 GB"],
                  ].map(([value, label]) => (
                    <option key={value} value={value}>
                      大于 {label}
                    </option>
                  ))}
                </select>
              </Row>
            </Section>
            <Section title="自动清理">
              <Row title="启用自动清理" description="仅处理至少 48 小时未修改的常见缓存文件">
                <Toggle
                  label="启用自动清理"
                  on={settings.autoCleanEnabled}
                  onChange={(value) => {
                    void patch({ autoCleanEnabled: value });
                  }}
                />
              </Row>
              <Row title="清理频率">
                <select
                  aria-label="清理频率"
                  disabled={!settings.autoCleanEnabled}
                  value={settings.autoCleanIntervalDays}
                  onChange={(e) => {
                    void patch({
                      autoCleanIntervalDays: Number(e.target.value),
                    });
                  }}
                >
                  <option value={1}>每天</option>
                  <option value={7}>每周</option>
                  <option value={30}>每月</option>
                </select>
              </Row>
              <div className="settings-group-note">
                自动清理会遵守排除目录，跳过最近 48 小时修改的文件、开发项目和受保护数据。
                {settings.lastAutoCleanTs && (
                  <span>
                    {" "}
                    上次执行：
                    {new Date(settings.lastAutoCleanTs * 1000).toLocaleString(
                      "zh-CN",
                    )}
                  </span>
                )}
              </div>
            </Section>
            <Section title="手动执行">
              <Row
                title="立即清理"
                description="按当前规则执行一次安全缓存清理"
              >
                <button
                  className="btn btn-primary"
                  disabled={busy}
                  onClick={runAutoClean}
                >
                  {busy ? "清理中…" : "立即执行"}
                </button>
              </Row>
              {autoCleanMsg && (
                <p className="settings-group-note" role="status">
                  {autoCleanMsg}
                </p>
              )}
            </Section>
            <Notice>清理后的文件将移入{trash}，可在需要时恢复。</Notice>
          </>
        )}
        {tab === "exclusions" && (
          <section className="settings-section">
            <div className="settings-group directory-group">
              <h2 className="settings-group-title">扫描排除目录 <span className="muted">· 以下目录不会被扫描和清理</span></h2>
              <table className="directory-table">
                <thead>
                  <tr>
                    <th scope="col">目录路径</th>
                    <th scope="col">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {settings.excludeDirs.map((path) => (
                    <tr key={path}>
                      <td>
                        <span className="directory-path">
                          <IconFolder size={22} />
                          <span title={path}>{path}</span>
                        </span>
                      </td>
                      <td>
                        <button
                          className="btn directory-remove"
                          aria-label={`移除 ${path}`}
                          onClick={() => {
                            void patch((current) => ({
                              excludeDirs: current.excludeDirs.filter(
                                (item) => item !== path,
                              ),
                            }));
                          }}
                        >
                          移除
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {!settings.excludeDirs.length && (
                <p className="settings-empty">
                  暂无排除目录，添加不需要扫描的文件夹。
                </p>
              )}
              <form
                className="directory-form"
                onSubmit={(e) => {
                  e.preventDefault();
                  void addDirectory();
                }}
              >
                <input
                  aria-label="目录路径"
                  placeholder={
                    platform === "windows"
                      ? "输入目录路径，如 C:\\Users\\x\\Downloads"
                      : "输入目录路径，如 /Users/x/Downloads"
                  }
                  value={excludeInput}
                  onChange={(e) => {
                    setExcludeInput(e.target.value);
                    setDirectoryError("");
                  }}
                />
                <button
                  type="button"
                  className="btn btn-ghost"
                  disabled={picking}
                  onClick={pickDirectory}
                >
                  {picking ? "选择中…" : "选择文件夹"}
                </button>
                <button
                  className="btn btn-primary"
                  type="submit"
                  disabled={!excludeInput.trim()}
                >
                  添加
                </button>
              </form>
              {directoryError && (
                <p className="settings-error" role="alert">
                  {directoryError}
                </p>
              )}
            </div>
            <Notice>排除目录及其子目录将自动跳过。</Notice>
          </section>
        )}
        {tab === "about" && (
          <>
            <div className="settings-about-brand settings-group">
              <span className="about-logo">
                <img src={appLogo} alt="" draggable={false} />
              </span>
              <div>
                <h2>DiskCleanup</h2>
                <p>让电脑清爽更快捷</p>
                <span className="settings-version">版本 0.1.0</span>
              </div>
            </div>
            <Section title="运行信息">
              <Row title="当前平台">
                <span className="settings-value">{platformName}</span>
              </Row>
              <Row title="系统磁盘">
                <span className="settings-value">
                  {systemDisk?.name ||
                    (platform === "macos" ? "Macintosh HD" : "C盘")}
                </span>
              </Row>
            </Section>
            <Section title="清理与恢复">
              <div className="settings-recovery">
                <IconShield size={34} />
                <div>
                  <h3>文件可恢复</h3>
                  <p>清理后的文件将移入{trash}，可在需要时恢复。</p>
                  {restoreButton}
                </div>
              </div>
            </Section>
          </>
        )}
        {(tab === "general" || tab === "about") && restoreMsg && (
          <p className="settings-error" role="status">
            {restoreMsg}
          </p>
        )}
      </div>
      <footer className="settings-footer">
        <span>DiskCleanup · v0.1.0 · {platformName}</span>
        {tab !== "about" && (
          <span role="status">
            <IconCheck size={14} />
            {saveState}
          </span>
        )}
      </footer>
    </div>
  );
}

// 启动 vite dev server，用无头浏览器在 mock 模式下截下每个界面。
// 用法：node scripts/screenshot.mjs  （会自动起停 dev server）
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import { chromium } from "playwright";

// 本地若有 HTTP 代理，localhost 会被误转发；此处对回环地址强制直连。
for (const k of ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"]) delete process.env[k];
process.env.NO_PROXY = "localhost,127.0.0.1";
process.env.no_proxy = "localhost,127.0.0.1";

const viteBin = fileURLToPath(new URL("../node_modules/.bin/vite", import.meta.url));
const SKIP_SERVER = process.env.SKIP_SERVER === "1";

const PORT = 5173;
const BASE = `http://localhost:${PORT}`;
const OUT_DIR = fileURLToPath(new URL("../docs/screenshots/", import.meta.url));

// [文件名, ?page 值, 等待选择器]
const PAGES = [
  ["home", "home", "main.page-home"],
  ["file-migration", "file-migration", "main.page-file-migration"],
  ["analyze", "analyze", "main.page-analyze"],
  ["results", "results", "main.page-results"],
  ["junk", "junk", "main.page-junk"],
  ["done", "done", "main.page-done"],
  ["large-files", "large-files", "main.page-large-files"],
  ["duplicates", "duplicates", "main.page-duplicates"],
  ["launch-items", "launch-items", "main.page-launch-items"],
  ["system-tools", "system-tools", "main.page-system-tools"],
  ["settings", "settings", "main.page-settings"],
];

function waitForServer(url, timeoutMs = 60000) {
  const start = Date.now();
  return new Promise((resolve, reject) => {
    const tick = async () => {
      try {
        const r = await fetch(url);
        if (r.ok) return resolve();
      } catch {}
      if (Date.now() - start > timeoutMs) return reject(new Error("dev server timeout"));
      setTimeout(tick, 500);
    };
    tick();
  });
}

const server = SKIP_SERVER
  ? null
  : spawn(process.execPath, [viteBin], {
      cwd: fileURLToPath(new URL("..", import.meta.url)),
      stdio: "ignore",
      env: { ...process.env, BROWSER: "none" },
    });

let browser;
try {
  await waitForServer(BASE);
  await mkdir(OUT_DIR, { recursive: true });
  browser = await chromium.launch({
    env: { ...process.env, no_proxy: "localhost,127.0.0.1", NO_PROXY: "localhost,127.0.0.1" },
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });

  const errors = [];
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  page.on("pageerror", (e) => errors.push(String(e)));

  for (const [file, id, sel] of PAGES) {
    console.log(`screenshot: ${id}`);
    await page.goto(`${BASE}/?mock=1&page=${id}`, { waitUntil: "networkidle" });
    try {
      await page.waitForSelector(sel, { timeout: 8000 });
    } catch {
      console.warn(`  ! selector ${sel} not found for ${id}`);
    }
    await page.waitForTimeout(700); // 等待图表/列表渲染
    await page.screenshot({ path: join(OUT_DIR, `${file}.png`), fullPage: false });
  }

  const manifest = PAGES.map(([file, id]) => ({ file: `docs/screenshots/${file}.png`, page: id }));
  await writeFile(join(OUT_DIR, "manifest.json"), JSON.stringify(manifest, null, 2));
  console.log("screenshots written to docs/screenshots/");
  if (errors.length) console.log("console/page errors during capture:\n" + errors.join("\n"));
} finally {
  if (browser) await browser.close();
  if (server) server.kill("SIGTERM");
}

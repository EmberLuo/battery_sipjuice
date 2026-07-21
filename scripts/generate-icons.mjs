import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "..");
const source = resolve(projectRoot, "src", "app-icon.svg");
const traySource = resolve(projectRoot, "src", "tray-icon.svg");
const output = resolve(projectRoot, "src-tauri", "icons");
const tauriBin = resolve(
  projectRoot,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri",
);

if (!existsSync(tauriBin)) {
  console.error("找不到 Tauri CLI，请先运行 npm install。");
  process.exit(1);
}

mkdirSync(output, { recursive: true });
function generatePngs(iconSource, iconOutput, sizes) {
  const result = spawnSync(
    tauriBin,
    [
      "icon",
      iconSource,
      "--output",
      iconOutput,
      ...sizes.flatMap((size) => ["--png", String(size)]),
    ],
    {
      cwd: projectRoot,
      stdio: "inherit",
    },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

generatePngs(source, output, [16, 24, 32, 48, 64, 96, 128, 256, 512]);

copyFileSync(resolve(output, "256x256.png"), resolve(output, "128x128@2x.png"));
copyFileSync(resolve(output, "512x512.png"), resolve(output, "icon.png"));
rmSync(resolve(output, "512x512.png"));

const trayTemp = mkdtempSync(join(tmpdir(), "battery-sipjuice-tray-"));
try {
  generatePngs(traySource, trayTemp, [64]);
  copyFileSync(resolve(trayTemp, "64x64.png"), resolve(output, "tray-icon.png"));
} finally {
  rmSync(trayTemp, { recursive: true, force: true });
}

console.log(`应用图标已从 ${source} 生成到 ${output}`);
console.log(`托盘图标已从 ${traySource} 生成到 ${resolve(output, "tray-icon.png")}`);

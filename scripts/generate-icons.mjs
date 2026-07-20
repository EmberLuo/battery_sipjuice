import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "..");
const source = resolve(projectRoot, "src", "app-icon.svg");
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
const sizes = [16, 24, 32, 48, 64, 96, 128, 256, 512];
const args = [
  "icon",
  source,
  "--output",
  output,
  ...sizes.flatMap((size) => ["--png", String(size)]),
];
const result = spawnSync(tauriBin, args, {
  cwd: projectRoot,
  stdio: "inherit",
});

if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);

copyFileSync(resolve(output, "256x256.png"), resolve(output, "128x128@2x.png"));
copyFileSync(resolve(output, "512x512.png"), resolve(output, "icon.png"));
rmSync(resolve(output, "512x512.png"));

console.log(`图标已从 ${source} 生成到 ${output}`);

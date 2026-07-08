#!/usr/bin/env node
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const files = {
  packageJson: resolve(rootDir, "package.json"),
  packageLock: resolve(rootDir, "package-lock.json"),
  cargoToml: resolve(rootDir, "src-tauri", "Cargo.toml"),
  cargoLock: resolve(rootDir, "src-tauri", "Cargo.lock"),
  tauriConfig: resolve(rootDir, "src-tauri", "tauri.conf.json"),
};

const packageName = "battery-sipjuice";
const targetVersion = process.argv[2] ?? readCargoVersion();

if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(targetVersion)) {
  throw new Error(`Invalid version: ${targetVersion}`);
}

updateJson(files.packageJson, (json) => {
  json.version = targetVersion;
});

updateJson(files.packageLock, (json) => {
  json.version = targetVersion;
  if (json.packages?.[""]) {
    json.packages[""].version = targetVersion;
  }
});

writeFileSync(
  files.cargoToml,
  replacePackageVersion(readFileSync(files.cargoToml, "utf8"), targetVersion),
);

writeFileSync(
  files.tauriConfig,
  JSON.stringify(
    withJson(files.tauriConfig, (json) => {
      json.version = targetVersion;
      return json;
    }),
    null,
    2,
  ) + "\n",
);

writeFileSync(
  files.cargoLock,
  replaceCargoLockPackageVersion(readFileSync(files.cargoLock, "utf8"), packageName, targetVersion),
);

console.log(`Battery SipJuice version set to ${targetVersion}`);

function readCargoVersion() {
  const cargoToml = readFileSync(files.cargoToml, "utf8");
  const packageSection = findTomlSection(cargoToml, "package");
  const match = packageSection.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error("Could not find package.version in src-tauri/Cargo.toml");
  }
  return match[1];
}

function updateJson(path, mutate) {
  const json = withJson(path, (value) => {
    mutate(value);
    return value;
  });
  writeFileSync(path, JSON.stringify(json, null, 2) + "\n");
}

function withJson(path, mutate) {
  return mutate(JSON.parse(readFileSync(path, "utf8")));
}

function replacePackageVersion(toml, version) {
  return replaceTomlSection(toml, "package", (section) => {
    if (!/^version\s*=\s*"[^"]+"/m.test(section)) {
      throw new Error("Could not find package.version in src-tauri/Cargo.toml");
    }
    return section.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
  });
}

function replaceCargoLockPackageVersion(lock, name, version) {
  const blocks = lock.split(/(?=^\[\[package\]\]$)/m);
  return blocks
    .map((block) => {
      if (!new RegExp(`^name\\s*=\\s*"${escapeRegExp(name)}"$`, "m").test(block)) {
        return block;
      }
      if (!/^version\s*=\s*"[^"]+"/m.test(block)) {
        throw new Error(`Could not find version for ${name} in src-tauri/Cargo.lock`);
      }
      return block.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
    })
    .join("");
}

function findTomlSection(toml, sectionName) {
  const startMatch = new RegExp(`^\\[${escapeRegExp(sectionName)}\\]\\s*$`, "m").exec(toml);
  if (!startMatch) {
    throw new Error(`Could not find [${sectionName}] in TOML`);
  }
  const start = startMatch.index;
  const rest = toml.slice(start + startMatch[0].length);
  const nextSection = /\n\[/.exec(rest);
  return nextSection ? toml.slice(start, start + startMatch[0].length + nextSection.index) : toml.slice(start);
}

function replaceTomlSection(toml, sectionName, replaceSection) {
  const startMatch = new RegExp(`^\\[${escapeRegExp(sectionName)}\\]\\s*$`, "m").exec(toml);
  if (!startMatch) {
    throw new Error(`Could not find [${sectionName}] in TOML`);
  }
  const start = startMatch.index;
  const restStart = start + startMatch[0].length;
  const rest = toml.slice(restStart);
  const nextSection = /\n\[/.exec(rest);
  const end = nextSection ? restStart + nextSection.index : toml.length;
  return toml.slice(0, start) + replaceSection(toml.slice(start, end)) + toml.slice(end);
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

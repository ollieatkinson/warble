#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const WINDOWS_PORTABLE_RUNTIME_FILES = [
  "onnxruntime.dll",
  "DirectML.dll",
  "dxcompiler.dll",
  "dxil.dll",
];
const PLATFORM_BUNDLE_SPECS = {
  linux: [
    { directory: "appimage", fileSuffixes: [".AppImage", ".sig"] },
    { directory: "deb", fileSuffixes: [".deb", ".sig"] },
    { directory: "rpm", fileSuffixes: [".rpm", ".sig"] },
  ],
  macos: [
    {
      allowAppBundles: true,
      directory: "macos",
      fileSuffixes: [".app.tar.gz", ".pkg", ".sig", ".zip"],
    },
    { directory: "dmg", fileSuffixes: [".dmg", ".sig"] },
  ],
  windows: [
    { directory: "msi", fileSuffixes: [".msi", ".sig"] },
    { directory: "nsis", fileSuffixes: [".exe", ".sig", ".zip"] },
  ],
};

const args = parseArgs(process.argv.slice(2));
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const tauriDir = path.join(repoRoot, "src-tauri");
const targetReleaseDir = path.join(tauriDir, "target", "release");
const bundleDir = path.join(targetReleaseDir, "bundle");

const packageJson = readJson(path.join(repoRoot, "package.json"));
const tauriConfig = readJson(path.join(tauriDir, "tauri.conf.json"));

const platform = args.platform ?? detectPlatform(process.platform);
const arch = normalizeArch(process.arch);
const productName = tauriConfig.productName || packageJson.name;
const version = tauriConfig.version || packageJson.version;
const safeName = productName.replace(/\s+/g, "-");
const artifactsDir = path.resolve(
  args.outputDir ?? path.join(repoRoot, "artifacts", platform),
);

if (!args.skipTauriBuild) {
  runChecked(resolveCommand("pnpm"), ["tauri", "build"], { cwd: repoRoot });
}

removeDirectory(artifactsDir);
fs.mkdirSync(artifactsDir, { recursive: true });

const copiedFiles = [];

if (platform === "windows") {
  createWindowsPortableZip({
    arch,
    artifactsDir,
    packageName: packageJson.name,
    safeName,
    targetReleaseDir,
    version,
  });
  copiedFiles.push(
    normalizeRelativePath(
      `${safeName}_${version}_windows_${arch}_portable.zip`,
    ),
  );
}

copyBundleArtifacts(bundleDir, artifactsDir, copiedFiles, platform);

if (copiedFiles.length === 0) {
  throw new Error(`No release artifacts were found in ${bundleDir}`);
}

const manifestPath = path.join(artifactsDir, "manifest.json");
fs.writeFileSync(
  manifestPath,
  JSON.stringify(
    {
      productName,
      version,
      platform,
      arch,
      generatedAt: new Date().toISOString(),
      files: copiedFiles.slice().sort(),
    },
    null,
    2,
  ),
);

console.log("");
console.log("Release artifacts are ready:");
for (const relativePath of copiedFiles.slice().sort()) {
  console.log(` - ${path.join(artifactsDir, relativePath)}`);
}
console.log(` - ${manifestPath}`);

function parseArgs(argv) {
  const parsed = {
    outputDir: null,
    platform: null,
    skipTauriBuild: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];

    if (value === "--skip-tauri-build") {
      parsed.skipTauriBuild = true;
      continue;
    }

    if (value === "--output-dir") {
      parsed.outputDir = argv[index + 1];
      index += 1;
      continue;
    }

    if (value === "--platform") {
      parsed.platform = argv[index + 1];
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${value}`);
  }

  return parsed;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function detectPlatform(nodePlatform) {
  switch (nodePlatform) {
    case "win32":
      return "windows";
    case "darwin":
      return "macos";
    case "linux":
      return "linux";
    default:
      throw new Error(`Unsupported platform: ${nodePlatform}`);
  }
}

function normalizeArch(nodeArch) {
  switch (nodeArch) {
    case "x64":
      return "x64";
    case "arm64":
      return "arm64";
    default:
      return nodeArch;
  }
}

function resolveCommand(command) {
  if (process.platform === "win32" && !command.endsWith(".cmd")) {
    return `${command}.cmd`;
  }

  return command;
}

function runChecked(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    cwd: options.cwd,
    env: process.env,
    shell: false,
    stdio: "inherit",
  });

  if (typeof result.status === "number" && result.status !== 0) {
    throw new Error(
      `Command failed (${result.status}): ${command} ${commandArgs.join(" ")}`,
    );
  }

  if (result.error) {
    throw result.error;
  }
}

function removeDirectory(directoryPath) {
  if (fs.existsSync(directoryPath)) {
    fs.rmSync(directoryPath, { force: true, recursive: true });
  }
}

function createWindowsPortableZip({
  arch,
  artifactsDir,
  packageName,
  safeName,
  targetReleaseDir,
  version,
}) {
  const portableStem = `${safeName}_${version}_windows_${arch}_portable`;
  const stagingRoot = path.join(artifactsDir, ".portable-staging");
  const portableDir = path.join(stagingRoot, portableStem);
  const portableZip = path.join(artifactsDir, `${portableStem}.zip`);
  const executableName = `${packageName}.exe`;
  const portableFiles = [executableName, ...WINDOWS_PORTABLE_RUNTIME_FILES];

  removeDirectory(stagingRoot);
  fs.mkdirSync(portableDir, { recursive: true });

  const missingFiles = portableFiles.filter(
    (fileName) => !fs.existsSync(path.join(targetReleaseDir, fileName)),
  );
  if (missingFiles.length > 0) {
    throw new Error(
      `Windows portable runtime is incomplete. Missing: ${missingFiles.join(", ")}`,
    );
  }

  for (const fileName of portableFiles) {
    fs.copyFileSync(
      path.join(targetReleaseDir, fileName),
      path.join(portableDir, fileName),
    );
  }

  runChecked("powershell.exe", [
    "-NoProfile",
    "-Command",
    `Compress-Archive -Path '${portableDir}\\*' -DestinationPath '${portableZip}' -Force`,
  ]);

  removeDirectory(stagingRoot);
}

function copyBundleArtifacts(sourceDir, artifactsDir, copiedFiles, platformName) {
  const bundleSpecs = PLATFORM_BUNDLE_SPECS[platformName];
  if (!fs.existsSync(sourceDir) || !bundleSpecs) {
    return;
  }

  for (const bundleSpec of bundleSpecs) {
    const bundleSourceDir = path.join(sourceDir, bundleSpec.directory);
    if (!fs.existsSync(bundleSourceDir)) {
      continue;
    }

    const entries = fs.readdirSync(bundleSourceDir, { withFileTypes: true });
    for (const entry of entries) {
      const sourcePath = path.join(bundleSourceDir, entry.name);

      if (entry.isDirectory()) {
        if (!bundleSpec.allowAppBundles || !entry.name.endsWith(".app")) {
          continue;
        }

        const existingArchivePath = `${sourcePath}.tar.gz`;
        if (fs.existsSync(existingArchivePath)) {
          continue;
        }

        const archiveRelativePath = path.join(
          "bundle",
          bundleSpec.directory,
          `${entry.name}.tar.gz`,
        );
        const archivePath = path.join(artifactsDir, archiveRelativePath);
        fs.mkdirSync(path.dirname(archivePath), { recursive: true });
        runChecked("tar", [
          "-czf",
          archivePath,
          "-C",
          bundleSourceDir,
          entry.name,
        ]);
        copiedFiles.push(normalizeRelativePath(archiveRelativePath));
        continue;
      }

      if (!bundleSpec.fileSuffixes.some((suffix) => entry.name.endsWith(suffix))) {
        continue;
      }

      const destinationRelativePath = path.join(
        "bundle",
        bundleSpec.directory,
        entry.name,
      );
      const destinationPath = path.join(artifactsDir, destinationRelativePath);
      fs.mkdirSync(path.dirname(destinationPath), { recursive: true });
      fs.copyFileSync(sourcePath, destinationPath);
      copiedFiles.push(normalizeRelativePath(destinationRelativePath));
    }
  }
}

function normalizeRelativePath(filePath) {
  return filePath.split(path.sep).join(path.posix.sep);
}

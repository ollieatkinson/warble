param(
  [switch]$SkipTauriBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tauriDir = Join-Path $repoRoot "src-tauri"
$targetReleaseDir = Join-Path $tauriDir "target\release"
$bundleDir = Join-Path $targetReleaseDir "bundle"
$artifactsDir = Join-Path $repoRoot "artifacts\windows"

$tauriConfig = Get-Content (Join-Path $tauriDir "tauri.conf.json") -Raw | ConvertFrom-Json
$packageJson = Get-Content (Join-Path $repoRoot "package.json") -Raw | ConvertFrom-Json

$productName = if ($tauriConfig.productName) { $tauriConfig.productName } else { $packageJson.name }
$version = if ($tauriConfig.version) { $tauriConfig.version } else { $packageJson.version }
$safeName = ($productName -replace "\s+", "-")
$portableStem = "{0}_{1}_windows_x64_portable" -f $safeName, $version

$portableFiles = @(
  "transcribed.exe",
  "onnxruntime.dll",
  "DirectML.dll",
  "dxcompiler.dll",
  "dxil.dll"
)

function Invoke-CmdChecked {
  param([string]$Command)

  $wrapped = "cd /d `"$repoRoot`" && $Command"
  & cmd.exe /c $wrapped
  $exitCode = if (Test-Path variable:global:LASTEXITCODE) {
    $global:LASTEXITCODE
  } else {
    0
  }

  if ($exitCode -ne 0) {
    throw "Command failed: $Command"
  }
}

Invoke-CmdChecked "where pnpm"
Invoke-CmdChecked "where cargo"

Push-Location $repoRoot
try {
  if (-not $SkipTauriBuild) {
    Write-Host "Building Windows release bundle..." -ForegroundColor Cyan
    Invoke-CmdChecked "pnpm tauri build"
  }
} finally {
  Pop-Location
}

if (Test-Path $artifactsDir) {
  Remove-Item $artifactsDir -Recurse -Force
}
New-Item -ItemType Directory -Path $artifactsDir | Out-Null

$portableDir = Join-Path $artifactsDir $portableStem
New-Item -ItemType Directory -Path $portableDir | Out-Null

$missingPortableFiles = @(
  $portableFiles | Where-Object {
    -not (Test-Path (Join-Path $targetReleaseDir $_))
  }
)

if ($missingPortableFiles.Count -gt 0) {
  $missingList = $missingPortableFiles -join ", "
  throw "Release runtime is incomplete. Missing: $missingList"
}

foreach ($fileName in $portableFiles) {
  Copy-Item (Join-Path $targetReleaseDir $fileName) -Destination (Join-Path $portableDir $fileName) -Force
}

$portableZip = Join-Path $artifactsDir ($portableStem + ".zip")
Compress-Archive -Path (Join-Path $portableDir "*") -DestinationPath $portableZip -Force

$copiedBundles = New-Object System.Collections.Generic.List[string]
foreach ($bundleSpec in @(
  @{ Directory = (Join-Path $bundleDir "nsis"); Filter = "*.exe" },
  @{ Directory = (Join-Path $bundleDir "msi"); Filter = "*.msi" }
)) {
  if (-not (Test-Path $bundleSpec.Directory)) {
    continue
  }

  foreach ($artifact in Get-ChildItem $bundleSpec.Directory -File -Filter $bundleSpec.Filter) {
    $destination = Join-Path $artifactsDir $artifact.Name
    Copy-Item $artifact.FullName -Destination $destination -Force
    $copiedBundles.Add($destination)
  }
}

Write-Host ""
Write-Host "Release artifacts are ready:" -ForegroundColor Green
Write-Host (" - {0}" -f $portableZip)
foreach ($artifact in $copiedBundles | Sort-Object -Unique) {
  Write-Host (" - {0}" -f $artifact)
}

if ($copiedBundles.Count -eq 0) {
  Write-Warning "No installer bundles were copied from src-tauri\\target\\release\\bundle. The portable zip is still ready."
}

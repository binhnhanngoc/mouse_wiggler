# build-portable.ps1
# Build Mouse Wiggler release binary and pack it into a portable folder + zip.
#
# Usage (from project root):
#   .\build-portable.ps1            # build + bundle
#   .\build-portable.ps1 -SkipBuild # bundle existing target\release\mouse_wiggler.exe

[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

# Resolve project root from script location so the script works from any CWD.
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ProjectRoot

# --- Read version from Cargo.toml -------------------------------------------
$cargoToml = Get-Content (Join-Path $ProjectRoot 'Cargo.toml') -Raw
if ($cargoToml -notmatch '(?m)^\s*version\s*=\s*"([^"]+)"') {
    throw "Could not parse version from Cargo.toml"
}
$Version = $Matches[1]
Write-Host "Mouse Wiggler v$Version" -ForegroundColor Cyan

# --- Build release binary ---------------------------------------------------
$ExePath = Join-Path $ProjectRoot 'target\release\mouse_wiggler.exe'
if (-not $SkipBuild) {
    Write-Host "Running: cargo build --release" -ForegroundColor Yellow
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed (exit $LASTEXITCODE)" }
}
if (-not (Test-Path $ExePath)) { throw "Missing build output: $ExePath" }

# --- Prepare dist folder ----------------------------------------------------
$BundleName = "MouseWiggler-v$Version"
$DistRoot   = Join-Path $ProjectRoot 'dist'
$BundleDir  = Join-Path $DistRoot $BundleName
$ZipPath    = Join-Path $DistRoot "$BundleName.zip"

# Retry helper: AV / OneDrive scans can briefly lock files just-written.
function Remove-WithRetry {
    param([string]$Path, [int]$Tries = 5, [int]$DelayMs = 400)
    for ($i = 1; $i -le $Tries; $i++) {
        if (-not (Test-Path $Path)) { return }
        try { Remove-Item $Path -Recurse -Force -ErrorAction Stop; return }
        catch { if ($i -eq $Tries) { throw } ; Start-Sleep -Milliseconds $DelayMs }
    }
}

Remove-WithRetry $BundleDir
Remove-WithRetry $ZipPath
New-Item -ItemType Directory -Path $BundleDir -Force | Out-Null

# --- Copy executable (renamed to MouseWiggler.exe for friendlier display) ---
Copy-Item $ExePath (Join-Path $BundleDir 'MouseWiggler.exe') -Force

# --- Generate README.txt with current version -------------------------------
$readme = @"
Mouse Wiggler v$Version - Portable
================================

Tiny Windows mouse mover that prevents idle/lock states.
Single .exe, no installer, no admin required.

QUICK START
-----------
1. Extract this folder anywhere (Desktop, USB drive, OneDrive, etc.).
2. Double-click MouseWiggler.exe.
3. On first launch, settings.json is created next to the .exe.

If Windows SmartScreen warns "Unknown Publisher":
  -> Click "More info" -> "Run anyway".

USAGE
-----
- Click "Start" to begin wiggling. "Stop" to halt.
- Minimize sends the app to the system tray. Right-click the tray
  icon for Show / Start-Stop / Exit. Left/double-click to restore.

MODES
-----
- Zen     : Invisible movement (dx=0, dy=0). Resets idle timer
            without moving the cursor. Best for Teams/Slack.
- Visible : Alternates +/-1 px. Cursor stays in place (net drift = 0).

SETTINGS (settings.json)
------------------------
{
  "mode": "zen",            // "zen" or "visible"
  "interval_sec": 60,       // 1 to 86400
  "autostart": false,       // launch on Windows sign-in
  "start_minimized": false  // start in tray
}

Changes save automatically (debounced 250 ms).

PORTABILITY
-----------
- The whole app is self-contained in this folder.
- Move the folder anywhere; settings.json travels with it.
- Autostart: if enabled, the registry entry self-heals to the
  current .exe path the next time you launch from a new location.

UNINSTALL
---------
1. Close the app (right-click tray -> Exit).
2. If autostart was enabled, uncheck it first OR manually delete:
   HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MouseWiggler
3. Delete this folder.

REQUIREMENTS
------------
Windows 10 or 11, x64. No runtime, no .NET, no admin.

License: MIT
"@
Set-Content -Path (Join-Path $BundleDir 'README.txt') -Value $readme -Encoding UTF8

# --- Zip the bundle ---------------------------------------------------------
Compress-Archive -Path $BundleDir -DestinationPath $ZipPath -Force

# --- Report ----------------------------------------------------------------
$exeSize = (Get-Item (Join-Path $BundleDir 'MouseWiggler.exe')).Length
$zipSize = (Get-Item $ZipPath).Length
Write-Host ""
Write-Host "Bundle ready" -ForegroundColor Green
Write-Host ("  Folder : {0}" -f $BundleDir)
Write-Host ("  Zip    : {0}  ({1:N0} bytes)" -f $ZipPath, $zipSize)
Write-Host ("  Exe    : {0:N0} bytes" -f $exeSize)

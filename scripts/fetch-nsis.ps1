<#
.SYNOPSIS
  Pre-fetch the NSIS toolchain Tauri v2 needs for `--bundles nsis`, via GitHub mirrors,
  to avoid the direct-download timeout ("timeout: global") that aborts the release.

.DESCRIPTION
  `tauri build --bundles nsis` downloads two things from GitHub on first run; a direct
  connection from CN often times out and aborts the whole release at the last step:
    1. nsis-3.13.zip        (tauri-apps/binary-releases)
    2. nsis_tauri_utils.dll (tauri-apps/nsis-tauri-utils v0.5.3, matches CLI 2.12.4)

  This script downloads them through GitHub mirrors and places them where Tauri expects:
    %LOCALAPPDATA%\tauri\NSIS\makensis.exe
    %LOCALAPPDATA%\tauri\NSIS\Plugins\x86-unicode\nsis_tauri_utils.dll
  Once present, Tauri skips the download and bundles offline.

  Idempotent: existing files are skipped; safe to re-run.

  NOTE: This file is intentionally ASCII-only. Windows PowerShell 5.1 reads a .ps1 using the
  system ANSI code page unless a UTF-8 BOM is present, and the IDE keeps stripping the BOM,
  which corrupts non-ASCII strings and breaks parsing. Keeping it ASCII makes it encoding-proof.

.PARAMETER Mirror
  Optional single mirror prefix (e.g. https://ghproxy.net/). If omitted, the built-in list
  is tried in order, falling back to a direct GitHub connection last.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/fetch-nsis.ps1
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/fetch-nsis.ps1 -Mirror https://ghproxy.net/
#>
[CmdletBinding()]
param(
  [string]$Mirror = ''
)

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# URLs match the bundler inside @tauri-apps/cli 2.12.4. After upgrading the CLI the
# nsis_tauri_utils version may change: grep the CLI binary for nsis_tauri_utils-vX.Y.Z,
# or read the URL from the failing bundle log.
$NsisZipUrl      = 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip'
$UtilsDllUrl     = 'https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll'
$NsisInnerFolder = 'nsis-3.13'   # top-level folder inside the zip

# Prefix proxies, ordered by measured speed (ghproxy.net fastest). Each is prepended to the
# full GitHub URL. These sites come and go; on failure the next one is tried automatically.
# Note: api.gitproxy.dev uses a "bare host" form (strip https://) and is NOT compatible here.
$Mirrors = @(
  'https://ghproxy.net/',
  'https://ghfast.top/',
  'https://ghproxy.link/',
  'https://ghclone.com/',
  'https://gh-proxy.com/',
  'https://mirror.ghproxy.com/'
)
if ($Mirror) { $Mirrors = @($Mirror.TrimEnd('/') + '/') }
$Mirrors += ''   # always fall back to a direct GitHub connection last

$CacheRoot  = Join-Path $env:LOCALAPPDATA 'tauri'
$NsisDir    = Join-Path $CacheRoot 'NSIS'
$PluginsDir = Join-Path $NsisDir 'Plugins\x86-unicode'
$DllPath    = Join-Path $PluginsDir 'nsis_tauri_utils.dll'
$Makensis   = Join-Path $NsisDir 'makensis.exe'

function Write-Step($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }
function Write-Ok($m)   { Write-Host "  [ok] $m" -ForegroundColor Green }
function Write-Note($m) { Write-Host "  [!] $m" -ForegroundColor Yellow }

function Get-FromMirrors {
  param([string]$Url, [string]$OutFile, [string]$Label)
  foreach ($m in $Mirrors) {
    $full = if ($m) { $m + $Url } else { $Url }
    $via  = if ($m) { $m } else { 'GitHub direct' }
    Write-Host "  trying: $via" -ForegroundColor DarkGray
    try {
      Invoke-WebRequest -Uri $full -OutFile $OutFile -UseBasicParsing -TimeoutSec 180
      $len = (Get-Item $OutFile).Length
      if ($len -le 0) { throw 'downloaded file is empty' }
      Write-Ok "$Label downloaded ($([math]::Round($len/1KB,1)) KB via $via)"
      return
    } catch {
      Write-Note "failed: $($_.Exception.Message)"
      # only remove the temp file this script just created for a failed download
      if (Test-Path $OutFile) { Remove-Item $OutFile -Force -ErrorAction SilentlyContinue }
    }
  }
  throw "all sources failed: $Url"
}

# ---- 1. NSIS core ----
Write-Step "Checking NSIS toolchain: $NsisDir"
if (Test-Path $Makensis) {
  Write-Ok 'makensis.exe exists, skipping NSIS core download'
} else {
  New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null
  $tmpZip     = Join-Path $env:TEMP ("nsis-3.13-" + [guid]::NewGuid().ToString('N') + '.zip')
  $tmpExtract = Join-Path $env:TEMP ("nsis-extract-" + [guid]::NewGuid().ToString('N'))
  try {
    Get-FromMirrors -Url $NsisZipUrl -OutFile $tmpZip -Label 'nsis-3.13.zip'
    Write-Host '  extracting...' -ForegroundColor DarkGray
    Expand-Archive -Path $tmpZip -DestinationPath $tmpExtract -Force
    $inner = Join-Path $tmpExtract $NsisInnerFolder
    if (-not (Test-Path $inner)) { $inner = $tmpExtract }   # some mirrors repack without the top folder
    # reaching here means makensis.exe was missing (likely a broken partial run); rebuild it
    if (Test-Path $NsisDir) { Remove-Item $NsisDir -Recurse -Force }
    Move-Item -Path $inner -Destination $NsisDir
    if (-not (Test-Path $Makensis)) { throw "makensis.exe still missing after extract ($Makensis)" }
    Write-Ok "NSIS ready: $NsisDir"
  } finally {
    Remove-Item $tmpZip     -Force          -ErrorAction SilentlyContinue
    Remove-Item $tmpExtract -Recurse -Force -ErrorAction SilentlyContinue
  }
}

# ---- 2. nsis_tauri_utils.dll plugin ----
Write-Step "Checking Tauri NSIS plugin: $DllPath"
if (Test-Path $DllPath) {
  Write-Ok 'nsis_tauri_utils.dll exists, skipping'
} else {
  New-Item -ItemType Directory -Force -Path $PluginsDir | Out-Null
  Get-FromMirrors -Url $UtilsDllUrl -OutFile $DllPath -Label 'nsis_tauri_utils.dll'
}

# ---- 3. Verify ----
Write-Step 'Verifying key files'
$missing = @()
foreach ($f in @($Makensis, $DllPath)) {
  if (Test-Path $f) { Write-Ok (Split-Path $f -Leaf) } else { $missing += $f }
}
if ($missing.Count) { throw ("missing files:`n  " + ($missing -join "`n  ")) }

Write-Host "`nNSIS toolchain ready. You can now re-run the release, e.g.: pnpm release --bump patch" -ForegroundColor Green
Write-Host '(or: node scripts/release.mjs build)' -ForegroundColor DarkGray

<#
.SYNOPSIS
  Run web lint checks plus perf regression tests (Windows version).

.DESCRIPTION
  Runs from web/:
    1. npm run lint
    2. npm run build-only (unless -SkipBuild)
    3. Start `vite preview` on the perf URL (unless it already answers,
       or -UseExistingPreview is given), wait until it is reachable.
    4. node tests/perf/run.mjs (== npm run perf)
    5. Stop the preview server it started (unless -KeepPreview).

  The preview server proxies /api to the local Rust backend on
  127.0.0.1:5272, so local postgres + backend must be running
  (e.g. `cargo run --release`). A TCP preflight check fails fast with
  instructions when the backend is unreachable (bypass: -SkipBackendCheck).
  See web/tests/perf/README.md.

.EXAMPLE
  # Full run (lint + build + preview + perf, 5 headed runs)
  .\scripts\lint-and-perf.ps1

.EXAMPLE
  # Lint only
  .\scripts\lint-and-perf.ps1 -SkipPerf

.EXAMPLE
  # Perf only against an already-running preview server (no build)
  .\scripts\lint-and-perf.ps1 -SkipLint -SkipBuild -UseExistingPreview

.EXAMPLE
  # Perf only, headless, custom run count / URL
  .\scripts\lint-and-perf.ps1 -SkipLint -Runs 3 -Url http://localhost:4173 -Headless
#>
[CmdletBinding()]
param(
  # Skip `npm run lint`.
  [switch]$SkipLint,

  # Skip perf tests.
  [switch]$SkipPerf,

  # Skip `npm run build-only` before serving preview.
  # Requires a fresh dist/ from a previous build.
  [switch]$SkipBuild,

  # Skip the TCP preflight check for the Rust backend on 127.0.0.1:5272.
  [switch]$SkipBackendCheck,

  # Do not start a preview server; fail if -Url is not already serving.
  [switch]$UseExistingPreview,

  # Leave a started preview server running after perf finishes.
  [switch]$KeepPreview,

  # Seconds to wait for the preview server to become reachable.
  [int]$PreviewTimeoutSec = 90,

  # Number of measured perf runs (plus warmup). Passed as --runs.
  [int]$Runs = 5,

  # App URL under test (preview server). Passed as --url.
  [string]$Url = 'http://localhost:4173',

  # Run perf browser headless. Headed (default) gives trustworthy FPS;
  # headless inflates FPS (no vsync) - spikes are still visible.
  [switch]$Headless,

  # Optional explicit output path, passed as --out to run.mjs.
  # Defaults to tests/perf/results/YYYY-MM-DD_<branch>_<sha>.json.
  [string]$OutFile = '',

  # Extra raw args forwarded to tests/perf/run.mjs (e.g. '-ExtraPerfArgs @("--foo","bar")').
  [string[]]$ExtraPerfArgs = @()
)

$ErrorActionPreference = 'Stop'

if ($PSVersionTable.PSVersion.Major -lt 5) {
  Write-Error 'This script requires PowerShell 5.1 or later.'
  exit 1
}

# Always run from web/ regardless of caller CWD.
$WebRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $WebRoot
try {
  function Test-CommandExists {
    param([string]$Name)
    $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
  }

  function Test-UrlReady {
    param([string]$TargetUrl)
    try {
      $resp = Invoke-WebRequest -Uri $TargetUrl -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop
      return ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 500)
    }
    catch {
      return $false
    }
  }

  function Test-BackendUp {
    param([string]$BackendHost = '127.0.0.1', [int]$BackendPort = 5272)
    $client = New-Object Net.Sockets.TcpClient
    try {
      $iar = $client.BeginConnect($BackendHost, $BackendPort, $null, $null)
      if (-not $iar.AsyncWaitHandle.WaitOne(3000)) { return $false }
      return $client.Connected
    }
    catch {
      return $false
    }
    finally {
      $client.Close()
    }
  }

  function Stop-PreviewServer {
    param($Proc)
    if ($null -eq $Proc) { return }
    try {
      if (-not $Proc.HasExited) {
        Write-Host 'Stopping preview server...' -ForegroundColor DarkGray
        # /T kills the whole tree (npm shim -> vite), so no orphan holds the port.
        & taskkill /PID $Proc.Id /T /F 2>$null | Out-Null
      }
    }
    catch {
      # Already gone - nothing to do.
    }
  }

  if (-not (Test-CommandExists 'node')) {
    Write-Error "Command 'node' is required but not found. Install Node.js 20.19+ / 22.12+."
    exit 1
  }
  if (-not (Test-CommandExists 'npm')) {
    Write-Error "Command 'npm' is required but not found. Install Node.js 20.19+ / 22.12+."
    exit 1
  }

  if (-not (Test-Path (Join-Path $WebRoot 'package.json'))) {
    Write-Error "package.json not found in $WebRoot. Is WebRoot wrong?"
    exit 1
  }

  $failed = $false

  if (-not $SkipLint) {
    Write-Host "`n=== 1. Lint (npm run lint) ===" -ForegroundColor Cyan
    npm run lint
    if ($LASTEXITCODE -ne 0) {
      Write-Host "`nLint failed (exit $LASTEXITCODE)." -ForegroundColor Red
      $failed = $true
    }
  }
  else {
    Write-Host "`n=== 1. Lint skipped (-SkipLint) ===" -ForegroundColor Yellow
  }

  if ($failed) {
    Write-Host "`n=== Some checks failed (lint) - skipping perf ===" -ForegroundColor Red
    exit 1
  }

  if (-not $SkipPerf) {
    Write-Host "`n=== 2. Perf tests ===" -ForegroundColor Cyan

    # --- 2a. Production build (perf numbers must come from `vite preview`). ---
    if (-not $SkipBuild) {
      Write-Host "`n--- Building production bundle (npm run build-only) ---" -ForegroundColor Cyan
      npm run build-only
      if ($LASTEXITCODE -ne 0) {
        Write-Host "`nBuild failed (exit $LASTEXITCODE) - skipping perf." -ForegroundColor Red
        exit 1
      }
    }
    elseif (-not (Test-Path (Join-Path $WebRoot 'dist/index.html'))) {
      Write-Error 'dist/index.html not found and -SkipBuild was given. Run without -SkipBuild once to build.'
      exit 1
    }

    # --- 2b. Backend preflight: preview proxies /api to 127.0.0.1:5272. ---
    if (-not $SkipBackendCheck) {
      Write-Host 'Checking Rust backend on 127.0.0.1:5272...' -ForegroundColor DarkGray
      if (-not (Test-BackendUp)) {
        Write-Host @'

Backend is unreachable on 127.0.0.1:5272. The preview server proxies
/api there, so perf login would fail. Start postgres + backend first,
e.g. `cargo run --release` (see web/tests/perf/README.md), or bypass
this check with -SkipBackendCheck.
'@ -ForegroundColor Red
        exit 1
      }
    }

    # --- 2c. Preview server: reuse if $Url answers, else start our own. ---
    $previewProc = $null
    try {
      if (Test-UrlReady $Url) {
        Write-Host "Reusing existing server at $Url (already reachable)." -ForegroundColor Yellow
        Write-Host 'Numbers reflect whatever that server serves - may be stale.' -ForegroundColor DarkGray
      }
      else {
        if ($UseExistingPreview) {
          Write-Error "No server reachable at $Url and -UseExistingPreview was given. Start it or drop the flag."
          exit 1
        }
        $parsedUri = $Url -as [uri]
        $port = if ($parsedUri -and $parsedUri.Port -gt 0) { $parsedUri.Port } else { 4173 }
        $previewLog = Join-Path ([System.IO.Path]::GetTempPath()) 'ruurd-photos-preview.log'
        if (Test-Path $previewLog) { Remove-Item $previewLog -Force }
        Write-Host "Starting preview server on port $port (log: $previewLog)..." -ForegroundColor Cyan
        # --strictPort: fail loudly instead of drifting to another port perf would miss.
        $cmdLine = "npm run preview -- --port $port --strictPort > `"$previewLog`" 2>&1"
        $previewProc = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', $cmdLine `
          -WorkingDirectory $WebRoot -PassThru -WindowStyle Hidden

        $deadline = (Get-Date).AddSeconds($PreviewTimeoutSec)
        $ready = $false
        while ((Get-Date) -lt $deadline) {
          if (Test-UrlReady $Url) { $ready = $true; break }
          if ($previewProc.HasExited) { break }
          Start-Sleep -Seconds 2
        }
        if (-not $ready) {
          Write-Host "`nPreview server did not become ready at $Url." -ForegroundColor Red
          if (Test-Path $previewLog) {
            Write-Host '--- preview log (tail) ---' -ForegroundColor DarkGray
            Get-Content $previewLog -Tail 30 | ForEach-Object { Write-Host $_ -ForegroundColor DarkGray }
            Write-Host '--------------------------' -ForegroundColor DarkGray
          }
          Write-Error "Preview server failed to start (exit code inquiry skipped). See log: $previewLog"
          exit 1
        }
        Write-Host "Preview server ready at $Url." -ForegroundColor Green
      }

      # --- 2d. Perf run (auto-compares against newest previous results/*.json). ---
      $perfArgs = @('tests/perf/run.mjs', '--runs', "$Runs", '--url', $Url)
      if ($Headless) { $perfArgs += '--headless' }
      if ($OutFile -ne '') { $perfArgs += @('--out', $OutFile) }
      if ($ExtraPerfArgs.Count -gt 0) { $perfArgs += $ExtraPerfArgs }

      Write-Host ("node " + ($perfArgs -join ' ')) -ForegroundColor DarkGray
      node @perfArgs
      if ($LASTEXITCODE -ne 0) {
        Write-Host "`nPerf run failed (exit $LASTEXITCODE)." -ForegroundColor Red
        exit 1
      }
      Write-Host "`nTip: compare two result files manually with:" -ForegroundColor DarkGray
      Write-Host '  node tests/perf/check-perf-regression.mjs --current <new.json> --baseline <main.json>' -ForegroundColor DarkGray
    }
    finally {
      if ($null -ne $previewProc -and -not $KeepPreview) {
        Stop-PreviewServer $previewProc
      }
      elseif ($null -ne $previewProc -and $KeepPreview) {
        Write-Host "Preview server left running at $Url (PID $($previewProc.Id))." -ForegroundColor Yellow
      }
    }
  }
  else {
    Write-Host "`n=== 2. Perf skipped (-SkipPerf) ===" -ForegroundColor Yellow
  }

  Write-Host "`n=== Success: lint + perf passed! ===" -ForegroundColor Green
}
finally {
  Pop-Location
}

[CmdletBinding()]
param(
# Optional path to local app_data folder. If omitted, it will automatically search relative to the script.
    [Parameter(Position = 0)]
    [string]$AppDataPath,

# Optional name of the Podman volume. If omitted, it auto-detects the app_data volume.
    [Parameter(Position = 1)]
    [string]$VolumeName
)

$ErrorActionPreference = "Stop"

# 1. Determine script and project directory paths
$ScriptDir = $PSScriptRoot
# Look up to find project root containing app_data
$ProjectRoot = (Get-Item $ScriptDir).Parent.FullName

# 2. Auto-detect host app_data path if not provided
if (-not $AppDataPath) {
    if (Test-Path "$ProjectRoot\app_data") {
        $AppDataPath = "$ProjectRoot\app_data"
    } elseif (Test-Path "$ScriptDir\app_data") {
        $AppDataPath = "$ScriptDir\app_data"
    } else {
        Write-Error "Could not automatically locate your host 'app_data' folder. Please specify it using: .\seed_app_data.ps1 -AppDataPath 'C:\path\to\app_data'"
    }
}

$ResolvedAppDataPath = (Get-Item $AppDataPath).FullName
Write-Host "[1/3] Host app_data directory: " -NoNewline
Write-Host "$ResolvedAppDataPath" -ForegroundColor Cyan

# 3. Auto-detect Podman volume name if not provided
if (-not $VolumeName) {
    Write-Host "[2/3] Searching for Podman app_data volume..." -ForegroundColor Yellow
    $matchingVolumes = podman volume ls --format "{{.Name}}" | Select-String "app_data"

    if ($matchingVolumes) {
        $VolumeName = ($matchingVolumes | Select-Object -First 1).ToString().Trim()
        Write-Host "      Found matching volume: " -NoNewline
        Write-Host "$VolumeName" -ForegroundColor Green
    } else {
        Write-Host "      No volume found. Initializing Podman Compose volumes..." -ForegroundColor Yellow
        podman compose create database | Out-Null

        $VolumeName = (podman volume ls --format "{{.Name}}" | Select-String "app_data" | Select-Object -First 1).ToString().Trim()
        if (-not $VolumeName) {
            Write-Error "Could not find or create Podman volume containing 'app_data'. Please ensure podman-compose is installed and run 'podman compose up' first."
        }
    }
} else {
    Write-Host "[2/3] Using specified volume: $VolumeName" -ForegroundColor Green
}

# 4. Copy files using temporary helper container
Write-Host "[3/3] Copying cache files to Podman volume (this may take a moment)..." -ForegroundColor Yellow

$podmanArgs = @(
    "run", "--rm",
    "-v", "${ResolvedAppDataPath}:/source:ro",
    "-v", "${VolumeName}:/target",
    "alpine",
    "sh", "-c", "cp -a /source/. /target/"
)

podman @podmanArgs

if ($LASTEXITCODE -eq 0) {
    Write-Host "Successfully populated Podman volume '$VolumeName' from host!" -ForegroundColor Green
} else {
    Write-Error "Failed to copy files into Podman volume (Exit code $LASTEXITCODE)."
}
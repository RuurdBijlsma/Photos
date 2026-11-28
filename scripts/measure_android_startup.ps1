param(
    [int]$Runs = 10,
    [int]$Warmup = 1,
    [string]$Package = "dev.ruurd.photos",
    [string]$Activity = ".MainActivity",
    [int]$FullyDrawnTimeoutMs = 5000,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$adbState = (& adb get-state 2>$null)
if ($LASTEXITCODE -ne 0 -or $adbState.Trim() -ne "device") {
    throw "No ADB device connected (adb get-state: '$adbState'). Connect wireless ADB first."
}

if (-not $SkipBuild) {
    $repoRoot = Split-Path $PSScriptRoot -Parent
    $androidDir = Join-Path $repoRoot "android"
    Push-Location $androidDir
    try {
        Write-Host "Building :app:assembleBenchmark..." -ForegroundColor Cyan
        & .\gradlew.bat :app:assembleBenchmark
        if ($LASTEXITCODE -ne 0) { throw "Gradle build failed." }

        $apk = Join-Path $androidDir "app\build\outputs\apk\benchmark\app-benchmark.apk"
        Write-Host "Installing $apk (preserving app data)..." -ForegroundColor Cyan
        & adb install -r -t $apk
        if ($LASTEXITCODE -ne 0) { throw "adb install failed." }
    }
    finally {
        Pop-Location
    }
}

function ConvertFrom-AdbDurationMs {
    param([string]$Line)
    if ($Line -match '\+(?:(?<s>\d+)s)?(?<ms>\d+)ms') {
        $sec = if ($Matches['s']) { [int]$Matches['s'] } else { 0 }
        $ms = [int]$Matches['ms']
        return ($sec * 1000) + $ms
    }
    return $null
}

function Get-Stats {
    param([double[]]$Values)
    if (-not $Values -or $Values.Count -eq 0) { return $null }
    $sorted = $Values | Sort-Object
    $n = $sorted.Count
    $median = if ($n % 2 -eq 1) {
        $sorted[[int][math]::Floor($n / 2)]
    } else {
        ($sorted[($n / 2) - 1] + $sorted[$n / 2]) / 2.0
    }
    $mean = ($sorted | Measure-Object -Average).Average
    return [PSCustomObject]@{
        Count  = $n
        Min    = [math]::Round($sorted[0], 1)
        Median = [math]::Round($median, 1)
        Mean   = [math]::Round($mean, 1)
        Max    = [math]::Round($sorted[$n - 1], 1)
    }
}

$totalIterations = $Warmup + $Runs
$results = @()

Write-Host "Measuring cold start for $Package/$Activity ($Warmup warmup + $Runs measured runs)..." -ForegroundColor Cyan

for ($i = 1; $i -le $totalIterations; $i++) {
    $isWarmup = $i -le $Warmup
    $label = if ($isWarmup) { "Warmup $i/$Warmup" } else { "Run $($i - $Warmup)/$Runs" }

    & adb shell am force-stop $Package | Out-Null
    Start-Sleep -Milliseconds 350
    & adb logcat -c | Out-Null

    $startOut = (& adb shell am start -W -n "$Package/$Activity" 2>&1) -join "`n"
    $amTotalMs = if ($startOut -match 'TotalTime:\s*(\d+)') { [int]$Matches[1] } else { $null }

    $displayedMs = $null
    $fullyDrawnMs = $null
    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    while ($sw.ElapsedMilliseconds -lt $FullyDrawnTimeoutMs) {
        $log = & adb logcat -d -s ActivityTaskManager 2>$null
        foreach ($line in $log) {
            if ($null -eq $displayedMs -and $line -match "Displayed\s+$([regex]::Escape($Package))/") {
                $displayedMs = ConvertFrom-AdbDurationMs $line
            }
            if ($null -eq $fullyDrawnMs -and $line -match "Fully drawn\s+$([regex]::Escape($Package))/") {
                $fullyDrawnMs = ConvertFrom-AdbDurationMs $line
            }
        }
        if ($null -ne $displayedMs -and $null -ne $fullyDrawnMs) {
            break
        }
        Start-Sleep -Milliseconds 100
    }

    Write-Host ("[{0}] amTotal={1}ms  Displayed(TTID)={2}ms  FullyDrawn(TTFD)={3}ms" -f `
        $label,
    $(if ($null -ne $amTotalMs) { $amTotalMs } else { "-" }),
    $(if ($null -ne $displayedMs) { $displayedMs } else { "-" }),
    $(if ($null -ne $fullyDrawnMs) { $fullyDrawnMs } else { "-" }))

    if (-not $isWarmup) {
        $results += [PSCustomObject]@{
            AmTotalMs    = $amTotalMs
            DisplayedMs  = $displayedMs
            FullyDrawnMs = $fullyDrawnMs
        }
    }
}

Write-Host "`n--- Summary ($Runs measured cold starts) ---" -ForegroundColor Green

foreach ($metric in @(
    @{ Name = "am start TotalTime (ms)"; Prop = "AmTotalMs" },
    @{ Name = "Displayed / TTID (ms)   "; Prop = "DisplayedMs" },
    @{ Name = "Fully drawn / TTFD (ms) "; Prop = "FullyDrawnMs" }
)) {
    $vals = @($results | ForEach-Object { $_.$($metric.Prop) } | Where-Object { $null -ne $_ } | ForEach-Object { [double]$_ })
    $stats = Get-Stats $vals
    if ($stats) {
        Write-Host ("{0}: median={1}  min={2}  mean={3}  max={4}  (n={5})" -f `
            $metric.Name, $stats.Median, $stats.Min, $stats.Mean, $stats.Max, $stats.Count)
    } else {
        Write-Host ("{0}: no samples captured" -f $metric.Name) -ForegroundColor Yellow
    }
}
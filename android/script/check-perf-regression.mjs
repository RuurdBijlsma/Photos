import fs from 'node:fs'
import path from 'node:path'

const BENCHMARK_OUTPUT_ROOT = path.resolve(
  'benchmark/build/outputs/connected_android_test_additional_output/benchmark/connected',
)
const BASELINE_PATH = path.resolve('perf/baseline.json')

// Allow +15% on median CPU frame time and +20% on P95/P99 tail frames (ignoring <0.5ms jitter).
const MAX_MEDIAN_REGRESSION = 0.15
const MAX_TAIL_REGRESSION = 0.20
const MIN_ABS_DELTA_MS = 0.5
// Fail if P95 frameOverrunMs exceeds 4ms past VSync (0ms = zero dropped frames at P95).
const MAX_P95_OVERRUN_MS = 4.0

function collectBenchmarkJsons(dir) {
  if (!fs.existsSync(dir)) {
    throw new Error(`Benchmark output directory not found: ${dir}`)
  }
  const files = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      files.push(...collectBenchmarkJsons(full))
    } else if (entry.name.endsWith('-benchmarkData.json')) {
      files.push({ path: full, mtime: fs.statSync(full).mtimeMs })
    }
  }
  return files
}

function findLatestBenchmarkJson(dir) {
  const files = collectBenchmarkJsons(dir)
  files.sort((a, b) => b.mtime - a.mtime)
  return files[0]?.path
}

function nearestRank(sorted, p) {
  if (sorted.length === 0) return 0
  const idx = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1))
  return sorted[idx]
}

function summarizeSampledMetric(metric) {
  if (metric.P50 !== undefined && metric.P95 !== undefined && metric.P99 !== undefined) {
    return {
      p50: metric.P50,
      p90: metric.P90 ?? metric.P95,
      p95: metric.P95,
      p99: metric.P99,
    }
  }
  const all = (metric.runs ?? []).flat().sort((a, b) => a - b)
  return {
    p50: nearestRank(all, 50),
    p90: nearestRank(all, 90),
    p95: nearestRank(all, 95),
    p99: nearestRank(all, 99),
  }
}

function extractSummary(jsonPath) {
  const raw = JSON.parse(fs.readFileSync(jsonPath, 'utf8'))
  const results = {}
  for (const b of raw.benchmarks ?? []) {
    const sampled = b.sampledMetrics ?? {}
    results[b.name] = {
      frameDurationCpuMs: sampled.frameDurationCpuMs
        ? summarizeSampledMetric(sampled.frameDurationCpuMs)
        : null,
      frameOverrunMs: sampled.frameOverrunMs
        ? summarizeSampledMetric(sampled.frameOverrunMs)
        : null,
    }
  }
  return {
    context: raw.context?.build ?? {},
    benchmarks: results,
  }
}

const args = new Set(process.argv.slice(2))
const jsonPath = findLatestBenchmarkJson(BENCHMARK_OUTPUT_ROOT)
if (!jsonPath) {
  throw new Error(`No *-benchmarkData.json found under ${BENCHMARK_OUTPUT_ROOT}`)
}

const current = extractSummary(jsonPath)
console.log(`Loaded benchmark results from: ${jsonPath}`)
console.log(JSON.stringify(current.benchmarks, null, 2))

if (args.has('--update-baseline')) {
  fs.mkdirSync(path.dirname(BASELINE_PATH), { recursive: true })
  fs.writeFileSync(BASELINE_PATH, JSON.stringify(current, null, 2) + '\n')
  console.log(`Updated baseline: ${BASELINE_PATH}`)
  process.exit(0)
}

const failures = []

for (const [name, metrics] of Object.entries(current.benchmarks)) {
  if (metrics.frameOverrunMs && metrics.frameOverrunMs.p95 > MAX_P95_OVERRUN_MS) {
    failures.push(
      `${name}: frameOverrunMs P95 = ${metrics.frameOverrunMs.p95.toFixed(2)}ms > ${MAX_P95_OVERRUN_MS}ms budget`,
    )
  }
}

if (fs.existsSync(BASELINE_PATH)) {
  const baseline = JSON.parse(fs.readFileSync(BASELINE_PATH, 'utf8'))
  for (const [name, metrics] of Object.entries(current.benchmarks)) {
    const base = baseline.benchmarks?.[name]?.frameDurationCpuMs
    const cur = metrics.frameDurationCpuMs
    if (!base || !cur) continue

    for (const [pct, limit] of [
      ['p50', MAX_MEDIAN_REGRESSION],
      ['p95', MAX_TAIL_REGRESSION],
      ['p99', MAX_TAIL_REGRESSION],
    ]) {
      const delta = cur[pct] - base[pct]
      const ratio = base[pct] > 0 ? delta / base[pct] : 0
      if (delta > MIN_ABS_DELTA_MS && ratio > limit) {
        failures.push(
          `${name}: frameDurationCpuMs ${pct} regressed from ${base[pct].toFixed(2)}ms to ${cur[pct].toFixed(2)}ms (+${(ratio * 100).toFixed(1)}%)`,
        )
      }
    }
  }
} else {
  console.warn(`No baseline found at ${BASELINE_PATH}; checked absolute P95 overrun budget only.`)
}

if (failures.length > 0) {
  console.error('\nPerformance regression gate FAILED:')
  for (const f of failures) console.error(`  - ${f}`)
  process.exit(1)
}

console.log('\nPerformance regression gate PASSED.')

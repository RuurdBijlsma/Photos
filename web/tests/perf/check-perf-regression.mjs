#!/usr/bin/env node
/**
 * check-perf-regression.mjs — compare a PR result against a baseline.
 *
 * Usage:
 *   node tests/perf/check-perf-regression.mjs --current <pr.json> --baseline <main.json>
 *
 * Exit 0 = same or better (within tolerance), 1 = regression, 2 = invalid input.
 * Multiple new files in a PR are fine: the caller passes the latest one
 * (by meta.timestamp). This script only ever compares two files.
 *
 * Schema version 2 (run.mjs). Version 1 results predate the photo-open and
 * navigation sampler fixes and cannot be compared — regenerate the baseline.
 */

import fs from 'node:fs'

function arg(name, fallback = null) {
  const i = process.argv.indexOf(`--${name}`)
  return i === -1 ? fallback : (process.argv[i + 1] ?? fallback)
}

const currentPath = arg('current')
const baselinePath = arg('baseline')
if (!currentPath || !baselinePath) {
  console.error('Usage: check-perf-regression.mjs --current <file> --baseline <file>')
  process.exit(2)
}

function load(p) {
  try {
    return JSON.parse(fs.readFileSync(p, 'utf8'))
  } catch (e) {
    console.error(`Cannot read ${p}: ${e.message}`)
    process.exit(2)
  }
}

const current = load(currentPath)
const baseline = load(baselinePath)

for (const key of ['version', 'meta', 'metrics']) {
  if (current[key] === undefined) {
    console.error(`Current file missing key: ${key}`)
    process.exit(2)
  }
}
if (current.version !== 2 || baseline.version !== 2) {
  console.error(
    `Schema v2 required (got current=${current.version}, baseline=${baseline.version}). ` +
      'Re-run tests/perf/run.mjs to regenerate results.',
  )
  process.exit(2)
}

// Lower-is-better (ms / frame times / spike counts) and higher-is-better (fps).
const checks = [
  {
    label: 'timeline LCP (median ms)',
    base: baseline.metrics?.timelineLcpMs?.median,
    curr: current.metrics?.timelineLcpMs?.median,
    relTol: 0.1,
    absTol: 200,
    lowerBetter: true,
  },
  {
    label: 'explore LCP (median ms)',
    base: baseline.metrics?.exploreLcpMs?.median,
    curr: current.metrics?.exploreLcpMs?.median,
    relTol: 0.1,
    absTol: 200,
    lowerBetter: true,
  },
  {
    label: 'photo-open click->image (median ms)',
    base: baseline.metrics?.photoOpen?.medianOpenMs,
    curr: current.metrics?.photoOpen?.medianOpenMs,
    relTol: 0.15,
    absTol: 300,
    lowerBetter: true,
  },
  {
    label: 'scroll p95 frame (median ms)',
    base: baseline.metrics?.scroll?.medianP95Frame,
    curr: current.metrics?.scroll?.medianP95Frame,
    relTol: 0.15,
    absTol: 5,
    lowerBetter: true,
  },
  {
    label: 'scroll spikes>100ms (median)',
    base: baseline.metrics?.scroll?.medianSpikes100,
    curr: current.metrics?.scroll?.medianSpikes100,
    relTol: 0,
    absTol: 0,
    lowerBetter: true,
  },
  {
    label: 'scroll FPS (median)',
    base: baseline.metrics?.scroll?.medianFps,
    curr: current.metrics?.scroll?.medianFps,
    relTol: 0.05,
    absTol: 2,
    lowerBetter: false,
  },
  {
    label: 'navigation p95 frame (median ms)',
    base: baseline.metrics?.navigation?.medianP95Frame,
    curr: current.metrics?.navigation?.medianP95Frame,
    relTol: 0.15,
    absTol: 5,
    lowerBetter: true,
  },
]

let failed = false
const rows = []
for (const c of checks) {
  if (typeof c.base !== 'number' || typeof c.curr !== 'number' || c.base < 0 || c.curr < 0) {
    console.error(`Missing metric for check: ${c.label} (base=${c.base}, curr=${c.curr})`)
    process.exit(2)
  }
  // -1 means the metric was never recorded (e.g. viewer never opened):
  // environment/test failure, not a pass.
  if (c.base === -1 || c.curr === -1) {
    console.error(`Metric not recorded for: ${c.label} (base=${c.base}, curr=${c.curr})`)
    process.exit(2)
  }
  const allowedDelta = c.base * c.relTol + c.absTol
  const isReg = c.lowerBetter ? c.curr > c.base + allowedDelta : c.curr < c.base - allowedDelta
  if (isReg) failed = true
  rows.push(`| ${c.label} | ${c.base} | ${c.curr} | ${isReg ? 'REGRESSION' : 'ok'} |`)
}

// Sanity: warn (don't fail) when the scroll test barely moved or saw nothing.
for (const [label, file] of [['baseline', baseline], ['current', current]]) {
  const d = file.metrics?.scroll?.medianDistancePx
  const n = file.metrics?.scroll?.medianItemsSeen
  if (typeof d === 'number' && d < 10000) {
    console.log(`WARNING: ${label} scroll distance only ${d}px — window may have been idle.`)
  }
  if (typeof n === 'number' && n === 0) {
    console.log(`WARNING: ${label} scroll saw 0 items — selector may be broken.`)
  }
}

console.log(`Baseline: ${baselinePath} (${baseline.meta?.commit ?? '?'}, ${baseline.meta?.timestamp ?? '?'})`)
console.log(`Current:  ${currentPath} (${current.meta?.commit ?? '?'}, ${current.meta?.timestamp ?? '?'})`)
console.log('')
console.log('| metric | baseline | current | status |')
console.log('|---|---|---|---|')
for (const r of rows) console.log(r)

if (failed) {
  console.error('\nPerf regression detected (outside tolerance).')
  process.exit(1)
}
console.log('\nPerf check passed: same or better than baseline.')

#!/usr/bin/env node
/**
 * run.mjs — local perf regression runner.
 *
 * Runs against a production build served via `vite preview`
 * (proxies /api -> local Rust backend on 127.0.0.1:5272),
 * with a local postgres. Do NOT use `vite dev` for numbers.
 *
 * Usage:
 *   PERF_USER_EMAIL=ruurd@example.com PERF_USER_PASSWORD=kibbeling \
 *     node tests/perf/run.mjs --url http://localhost:4173 --runs 5 --out results/<name>.json
 *
 * From web/:  npm run perf
 *
 * Output: one JSON file with meta + median/p95 per metric (version 2).
 * Commit that file with your MR. CI compares the latest file
 * in the MR against the newest file on main.
 *
 * NOTE: headed mode is the default. Headless Chromium does not vsync-lock
 * requestAnimationFrame, so FPS reads ~3x too high there. Spikes are still
 * visible headless, but use headed numbers for the committed baseline.
 */

import { execSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

function arg(name, fallback) {
  const i = process.argv.indexOf(`--${name}`)
  if (i === -1) return fallback
  return process.argv[i + 1] ?? fallback
}
function flag(name) {
  return process.argv.includes(`--${name}`)
}

const URL = arg('url', process.env.PERF_URL ?? 'http://localhost:4173')
const RUNS = Number(arg('runs', process.env.PERF_RUNS ?? '5'))
const HEADLESS = flag('headless') || process.env.PERF_HEADLESS === '1'
const VIEWPORT = { width: 1920, height: 1080 }
const EMAIL = process.env.PERF_USER_EMAIL ?? 'ruurd@example.com'
const PASSWORD = process.env.PERF_USER_PASSWORD ?? 'kibbeling'
const SCROLL_PX_PER_SEC = 33000
const SCROLL_DURATION_MS = 5000

const outArg = arg('out', null)
const outFile =
  outArg ??
  (() => {
    const d = new Date().toISOString().slice(0, 10)
    const branch = git('rev-parse --abbrev-ref HEAD').replace(/[^a-zA-Z0-9-_]+/g, '-')
    const sha = git('rev-parse --short HEAD')
    return path.join(__dirname, 'results', `${d}_${branch}_${sha}.json`)
  })()

function git(cmd) {
  try {
    return execSync(`git ${cmd}`, { encoding: 'utf8' }).trim()
  } catch {
    return 'unknown'
  }
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const median = (a) => {
  if (!a.length) return 0
  const s = [...a].sort((x, y) => x - y)
  const m = Math.floor(s.length / 2)
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2
}
const p95 = (a) => {
  if (!a.length) return 0
  const s = [...a].sort((x, y) => x - y)
  return s[Math.min(s.length - 1, Math.floor(s.length * 0.95))]
}
const round1 = (n) => Math.round(n * 10) / 10

async function launch() {
  // Prefer real Chrome (user tests in Chrome); fall back to bundled Chromium.
  for (const opts of [{ channel: 'chrome' }, {}]) {
    try {
      return await chromium.launch({
        headless: HEADLESS,
        args: [
          '--disable-extensions',
          '--disable-background-timer-throttling',
          '--disable-backgrounding-occluded-windows',
          '--disable-renderer-backgrounding',
        ],
        ...opts,
      })
    } catch {
      continue
    }
  }
  throw new Error('Could not launch Chrome/Chromium. Run: npx playwright install chromium')
}

async function login(page) {
  await page.goto(`${URL}/login`, { waitUntil: 'networkidle', timeout: 20000 })
  const quick = page.locator('button', { hasText: 'Login Ruurd' })
  if (await quick.isVisible({ timeout: 3000 }).catch(() => false)) {
    await quick.click()
  } else {
    await page.locator('input[autocomplete="email"]').fill(EMAIL)
    await page.locator('input[autocomplete="password"]').fill(PASSWORD)
    await page.locator('button[type="submit"]').click()
  }
  await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 20000 })
}

/** LCP observed from page load until settle. Returns ms, or -1 if not recorded. */
async function measurePageLCP(page, targetPath, settleSelector) {
  await page.goto(`${URL}${targetPath}`, { waitUntil: 'domcontentloaded' })
  const lcp = await page.evaluate(
    async (sel) => {
      const lcpEntries = []
      const obs = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) lcpEntries.push(e.startTime)
      })
      try {
        obs.observe({ type: 'largest-contentful-paint', buffered: true })
      } catch {
        return -1
      }
      // Wait for app content to settle (max 15s).
      const t0 = Date.now()
      while (Date.now() - t0 < 15000) {
        if (sel && document.querySelector(sel)) {
          // Give LCP one more frame to fire.
          await new Promise((r) => setTimeout(r, 800))
          break
        }
        await new Promise((r) => setTimeout(r, 250))
      }
      obs.disconnect()
      return lcpEntries.length ? Math.max(...lcpEntries) : -1
    },
    settleSelector,
  )
  return Math.round(lcp)
}

function frameStats(frames) {
  const spikes100 = frames.filter((d) => d >= 100).length
  const spikes50 = frames.filter((d) => d >= 50).length
  return {
    frames: frames.length,
    meanFps: frames.length ? Math.round(1000 / (frames.reduce((a, b) => a + b, 0) / frames.length)) : 0,
    p50Frame: round1(median(frames)),
    p95Frame: round1(p95(frames)),
    maxFrame: round1(Math.max(...frames, 0)),
    dropped50ms: spikes50,
    spikes100ms: spikes100,
  }
}

async function startSampler(page) {
  await page.evaluate(() => {
    window.__frames = []
    window.__rafOn = true
    let last = performance.now()
    const tick = () => {
      if (!window.__rafOn) return
      const now = performance.now()
      window.__frames.push(now - last)
      last = now
      requestAnimationFrame(tick)
    }
    requestAnimationFrame(tick)
  })
}

async function stopSampler(page) {
  return page.evaluate(() => {
    window.__rafOn = false
    return window.__frames ?? []
  })
}

/**
 * Scroll test at ~33k px/s on the timeline scroll container.
 * Wraps back to the top when the bottom is reached so small libraries
 * don't leave most of the window idle, and counts distinct items seen
 * mid-scroll (the virtualizer unmounts rows, so end-state DOM is useless).
 */
async function scrollTest(page) {
  await page.goto(`${URL}/`, { waitUntil: 'networkidle' })
  await page.waitForSelector('.scroll-container', { timeout: 20000 })
  await sleep(1500)
  await startSampler(page)
  const detail = await page.evaluate(
    async ({ step, durationMs }) => {
      const loafs = []
      let obs = null
      try {
        obs = new PerformanceObserver((list) => {
          for (const e of list.getEntries()) if (e.duration >= 100) loafs.push(e.duration)
        })
        obs.observe({ type: 'long-animation-frame', buffered: false })
      } catch {
        obs = null
      }
      const el = document.querySelector('.scroll-container')
      if (!el) return { error: 'no .scroll-container' }
      el.scrollTop = 0
      await new Promise((r) => setTimeout(r, 400))
      const seen = new Set()
      const t0 = performance.now()
      let lastTop = el.scrollTop
      let distancePx = 0
      let wraps = 0
      let tick = 0
      while (performance.now() - t0 < durationMs) {
        el.scrollBy({ top: step, behavior: 'instant' })
        await new Promise((r) => setTimeout(r, 16))
        distancePx += Math.abs(el.scrollTop - lastTop)
        lastTop = el.scrollTop
        if (el.scrollTop + el.clientHeight >= el.scrollHeight - 50) {
          wraps++
          el.scrollTop = 0
          lastTop = 0
        }
        if (tick++ % 10 === 0) {
          for (const item of document.querySelectorAll('.scroll-container .virtual-scroll-item[data-id]')) {
            seen.add(item.getAttribute('data-id'))
          }
        }
      }
      if (obs) obs.disconnect()
      return { loafSpikes: loafs.length, itemsSeen: seen.size, distancePx: Math.round(distancePx), wraps }
    },
    { step: Math.round(SCROLL_PX_PER_SEC / 60), durationMs: SCROLL_DURATION_MS },
  )
  const frames = await stopSampler(page)
  return { ...frameStats(frames), detail }
}

/**
 * Click first timeline photo -> fullscreen viewer.
 * Clicks the inner router-link (GridItem renders a .view-link anchor;
 * the outer .virtual-scroll-item div itself is not clickable) and
 * measures click -> viewer image loaded latency plus viewer LCP.
 * Skips videos: a video item has no LCP <img> and the loaded-wait would
 * stall while it plays (GridItem marks videos with .video-info).
 */
async function photoOpenTest(page) {
  await page.goto(`${URL}/`, { waitUntil: 'networkidle' })
  await page.waitForSelector('.scroll-container .virtual-scroll-item .view-link', { timeout: 20000 })
  await sleep(1000)
  const photoLink = page.locator(
    '.scroll-container .virtual-scroll-item:not(:has(.video-info)) .view-link',
  )
  const photoOnly = (await photoLink.count()) > 0
  const link = photoOnly ? photoLink.first() : page.locator('.scroll-container .virtual-scroll-item .view-link').first()
  const mediaId = await link.evaluate(
    (el) => el.closest('.virtual-scroll-item')?.getAttribute('data-id') ?? null,
  )
  // Arm a buffered LCP observer before the SPA navigation happens.
  const installMark = await page.evaluate(() => {
    window.__lcpEntries = []
    const obs = new PerformanceObserver((list) => {
      for (const e of list.getEntries()) window.__lcpEntries.push(e.startTime)
    })
    try {
      obs.observe({ type: 'largest-contentful-paint', buffered: true })
    } catch {
      return -1
    }
    window.__lcpObs = obs
    return performance.now()
  })
  const t0 = Date.now()
  await link.click()
  await page.waitForURL(/\/view\//, { timeout: 15000 })
  await page.waitForSelector('.view-container', { timeout: 15000 })
  await page
    .waitForFunction(
      () =>
        [...document.querySelectorAll('.view-container img')].some(
          (img) => img.complete && img.naturalWidth > 0,
        ) || !!document.querySelector('.view-container video'),
      { timeout: 15000 },
    )
    .catch(() => {})
  const openMs = Date.now() - t0
  const lcpMs = await page.evaluate((mark) => {
    const entries = (window.__lcpEntries ?? []).filter((t) => mark < 0 || t >= mark)
    if (window.__lcpObs) window.__lcpObs.disconnect()
    if (entries.length) return Math.round(Math.max(...entries))
    const buffered = performance
      .getEntriesByType('largest-contentful-paint')
      .map((e) => e.startTime)
      .filter((t) => mark < 0 || t >= mark)
    return buffered.length ? Math.round(Math.max(...buffered)) : -1
  }, installMark)
  // Close viewer for the next test.
  await page.keyboard.press('Escape').catch(() => {})
  await sleep(500)
  return { openMs, lcpMs, photoOnly, mediaId }
}

/**
 * Page-to-page FPS while navigating.
 * Samples rAF per route AFTER navigation: a page.goto() destroys the
 * execution context, so a single sampler across navigations records nothing.
 */
async function navigationTest(page) {
  const routes = [
    { path: '/', selector: '.scroll-container' },
    { path: '/explore', selector: '.explore-content' },
    { path: '/map', selector: 'canvas' },
    { path: '/search', selector: 'input' },
    { path: '/', selector: '.scroll-container' },
  ]
  const allFrames = []
  const perRoute = {}
  for (const { path: routePath, selector } of routes) {
    await page.goto(`${URL}${routePath}`, { waitUntil: 'networkidle' })
    if (selector) await page.waitForSelector(selector, { timeout: 15000 }).catch(() => {})
    await startSampler(page)
    await sleep(1500)
    const frames = await stopSampler(page)
    allFrames.push(...frames)
    const s = frameStats(frames)
    perRoute[routePath] = { fps: s.meanFps, p95Frame: s.p95Frame, maxFrame: s.maxFrame }
  }
  return { ...frameStats(allFrames), perRoute }
}

async function main() {
  console.log(`Perf run: ${URL} | runs=${RUNS} | headed=${!HEADLESS} | out=${outFile}`)
  if (HEADLESS) {
    console.log('WARNING: headless mode inflates FPS (no vsync). Prefer headed for committed baselines.')
  }
  const browser = await launch()
  const context = await browser.newContext({ viewport: VIEWPORT })
  const page = await context.newPage()
  await login(page)
  // Warmup: settle backend caches, thumbnails, route chunks.
  await page.goto(`${URL}/`, { waitUntil: 'networkidle' })
  await sleep(1500)

  const collect = { timelineLcp: [], photoOpen: [], exploreLcp: [], scroll: [], nav: [] }
  for (let i = 0; i < RUNS; i++) {
    console.log(`— run ${i + 1}/${RUNS} —`)
    collect.timelineLcp.push(await measurePageLCP(page, '/', '.scroll-container'))
    collect.exploreLcp.push(await measurePageLCP(page, '/explore', '.explore-content'))
    collect.photoOpen.push(await photoOpenTest(page))
    collect.scroll.push(await scrollTest(page))
    collect.nav.push(await navigationTest(page))
  }
  await browser.close()

  const summarizeFrames = (runs) => ({
    medianFps: median(runs.map((r) => r.meanFps)),
    medianP95Frame: median(runs.map((r) => r.p95Frame)),
    medianMaxFrame: median(runs.map((r) => r.maxFrame)),
    medianSpikes100: median(runs.map((r) => r.spikes100ms)),
    values: runs,
  })

  const result = {
    version: 2,
    meta: {
      timestamp: new Date().toISOString(),
      commit: git('rev-parse HEAD'),
      branch: git('rev-parse --abbrev-ref HEAD'),
      url: URL,
      viewport: `${VIEWPORT.width}x${VIEWPORT.height}`,
      headed: !HEADLESS,
      runs: RUNS,
      scrollPxPerSec: SCROLL_PX_PER_SEC,
      chrome: process.env.PERF_CHROME_VERSION ?? 'channel=chrome-or-chromium',
      user: EMAIL,
    },
    metrics: {
      timelineLcpMs: { median: median(collect.timelineLcp), values: collect.timelineLcp },
      exploreLcpMs: { median: median(collect.exploreLcp), values: collect.exploreLcp },
      photoOpen: {
        medianOpenMs: median(collect.photoOpen.map((r) => r.openMs)),
        medianLcpMs: median(collect.photoOpen.map((r) => r.lcpMs)),
        values: collect.photoOpen,
      },
      scroll: {
        ...summarizeFrames(collect.scroll),
        medianDistancePx: median(collect.scroll.map((r) => r.detail?.distancePx ?? 0)),
        medianItemsSeen: median(collect.scroll.map((r) => r.detail?.itemsSeen ?? 0)),
      },
      navigation: summarizeFrames(collect.nav),
    },
  }
  fs.mkdirSync(path.dirname(outFile), { recursive: true })
  fs.writeFileSync(outFile, JSON.stringify(result, null, 2))
  console.log(`Wrote ${outFile}`)
  console.log(JSON.stringify(result.metrics, null, 2))
}

main().catch((e) => {
  console.error(e)
  process.exit(2)
})

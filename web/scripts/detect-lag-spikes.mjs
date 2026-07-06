#!/usr/bin/env node
/**
 * detect-lag-spikes.mjs
 *
 * Automated lag-spike detector for the timeline scroll.
 * Designed to be used with `git bisect run`.
 *
 * What it does:
 *   1. Launches a headed Chromium browser via Playwright
 *   2. Logs in with dev credentials
 *   3. Waits for the timeline to load
 *   4. Installs a PerformanceObserver for "long-animation-frame" entries
 *   5. Scrolls the timeline rapidly for ~5 seconds (simulating MX Master infinite scroll)
 *   6. Collects all long animation frames and reports them
 *   7. Exits with code 0 (good / no spikes) or 1 (bad / spikes detected)
 *
 * Usage:
 *   node scripts/detect-lag-spikes.mjs [options]
 *
 * Options:
 *   --threshold <ms>     Frame duration threshold in ms (default: 100)
 *   --min-spikes <n>     Minimum number of spikes to consider "bad" (default: 3)
 *   --scroll-duration <ms>  How long to scroll in ms (default: 5000)
 *   --url <url>          App URL (default: http://localhost:5173)
 *   --headless           Run in headless mode (default: headed for debugging)
 *   --runs <n>           Number of scroll runs to average (default: 1)
 *
 * Exit codes:
 *   0   = No significant lag spikes (good commit)
 *   1   = Lag spikes detected (bad commit)
 *   125 = Could not run test (skip this commit in bisect)
 */

import { chromium } from 'playwright'

// ─── Parse CLI args ────────────────────────────────────────
function parseArgs() {
  const args = process.argv.slice(2)
  const opts = {
    threshold: 100,
    minSpikes: 3,
    scrollDuration: 5000,
    url: 'http://localhost:5173',
    headless: false,
    runs: 1,
  }

  for (let i = 0; i < args.length; i++) {
    switch (args[i]) {
      case '--threshold':
        opts.threshold = Number(args[++i])
        break
      case '--min-spikes':
        opts.minSpikes = Number(args[++i])
        break
      case '--scroll-duration':
        opts.scrollDuration = Number(args[++i])
        break
      case '--url':
        opts.url = args[++i]
        break
      case '--headless':
        opts.headless = true
        break
      case '--runs':
        opts.runs = Number(args[++i])
        break
    }
  }
  return opts
}

const opts = parseArgs()

// ─── Helpers ───────────────────────────────────────────────
function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

function color(code, text) {
  return `\x1b[${code}m${text}\x1b[0m`
}

const log = {
  info: (msg) => console.log(color('36', `[INFO]  ${msg}`)),
  good: (msg) => console.log(color('32', `[GOOD]  ${msg}`)),
  bad: (msg) => console.log(color('31', `[BAD]   ${msg}`)),
  warn: (msg) => console.log(color('33', `[WARN]  ${msg}`)),
  dim: (msg) => console.log(color('2', `        ${msg}`)),
}

// ─── Main ──────────────────────────────────────────────────
async function main() {
  log.info(`Lag spike detector`)
  log.dim(`Threshold: ${opts.threshold}ms | Min spikes to fail: ${opts.minSpikes}`)
  log.dim(`Scroll duration: ${opts.scrollDuration}ms | Runs: ${opts.runs}`)
  log.dim(`URL: ${opts.url}`)
  console.log()

  let browser
  try {
    browser = await chromium.launch({
      headless: opts.headless,
      // Chrome flags for consistent rendering perf measurement
      args: [
        '--disable-extensions',
        '--disable-background-timer-throttling',
        '--disable-backgrounding-occluded-windows',
        '--disable-renderer-backgrounding',
      ],
    })
  } catch (e) {
    log.warn(`Could not launch browser: ${e.message}`)
    log.warn(`Make sure Playwright is installed: npx playwright install chromium`)
    process.exit(125)
  }

  const context = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
  })
  const page = await context.newPage()

  try {
    // ── Step 1: Login ──────────────────────────────────────
    log.info('Navigating to login page...')
    await page.goto(`${opts.url}/login`, { waitUntil: 'networkidle', timeout: 15000 })

    // Use the dev quick-login button if available, otherwise fill the form
    const quickLoginBtn = page.locator('button', { hasText: 'Login Ruurd' })
    if (await quickLoginBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
      log.info('Found dev quick-login button, clicking...')
      await quickLoginBtn.click()
    } else {
      log.info('Filling login form...')
      await page.locator('input[autocomplete="email"]').fill('ruurd@example.com')
      await page.locator('input[autocomplete="password"]').fill('kibbeling')
      await page.locator('button[type="submit"]').click()
    }

    // Wait for navigation to timeline
    log.info('Waiting for timeline to load...')
    await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 15000 })
    // Wait for the scroll container to appear (timeline is rendered)
    await page.waitForSelector('.scroll-container', { timeout: 15000 })
    // Give Vue/virtualizer a moment to settle
    await sleep(1500)

    log.info('Timeline loaded. Starting scroll test...')
    console.log()

    // ── Step 2: Run scroll tests ───────────────────────────
    let allSpikes = []

    for (let run = 0; run < opts.runs; run++) {
      if (opts.runs > 1) log.info(`── Run ${run + 1}/${opts.runs} ──`)

      // Scroll back to top before each run
      await page.evaluate(() => {
        const el = document.querySelector('.scroll-container')
        if (el) el.scrollTop = 0
      })
      await sleep(500)

      // Install the PerformanceObserver to collect long animation frames
      const spikes = await page.evaluate(async ({ threshold, scrollDuration }) => {
        return new Promise((resolve) => {
          const longFrames = []

          // Try Long Animation Frames API (Chrome 123+), fall back to longtask
          let observerType = 'long-animation-frame'
          let observer
          try {
            observer = new PerformanceObserver((list) => {
              for (const entry of list.getEntries()) {
                const duration = entry.duration
                if (duration >= threshold) {
                  longFrames.push({
                    duration: Math.round(duration * 100) / 100,
                    startTime: Math.round(entry.startTime),
                    type: observerType,
                  })
                }
              }
            })
            observer.observe({ type: observerType, buffered: false })
          } catch {
            // Fallback: use longtask API
            observerType = 'longtask'
            try {
              observer = new PerformanceObserver((list) => {
                for (const entry of list.getEntries()) {
                  const duration = entry.duration
                  if (duration >= threshold) {
                    longFrames.push({
                      duration: Math.round(duration * 100) / 100,
                      startTime: Math.round(entry.startTime),
                      type: observerType,
                    })
                  }
                }
              })
              observer.observe({ type: observerType, buffered: false })
            } catch {
              // Neither API available
              observer = null
            }
          }

          // Also measure with requestAnimationFrame as a backup detector
          const frameTimes = []
          let lastFrameTime = performance.now()
          let rafRunning = true

          function measureFrame() {
            if (!rafRunning) return
            const now = performance.now()
            const delta = now - lastFrameTime
            if (delta >= threshold) {
              frameTimes.push({
                duration: Math.round(delta * 100) / 100,
                startTime: Math.round(now),
                type: 'raf-gap',
              })
            }
            lastFrameTime = now
            requestAnimationFrame(measureFrame)
          }
          requestAnimationFrame(measureFrame)

          // Simulate rapid scrolling on the scroll container
          const scrollEl = document.querySelector('.scroll-container')
          if (!scrollEl) {
            resolve([{ error: 'No .scroll-container found' }])
            return
          }

          const scrollStep = 150 // pixels per tick
          const scrollInterval = 16 // ~60fps tick rate
          const totalTicks = Math.floor(scrollDuration / scrollInterval)
          let tick = 0

          const interval = setInterval(() => {
            scrollEl.scrollBy({ top: scrollStep, behavior: 'instant' })
            tick++
            if (tick >= totalTicks) {
              clearInterval(interval)
              // Wait a bit for any trailing long frames to be reported
              setTimeout(() => {
                rafRunning = false
                if (observer) observer.disconnect()

                // Prefer LoAF/longtask entries (most accurate).
                // Only fall back to raf-gap if the observer didn't capture anything
                // (e.g. API not available, or observer was null).
                const allDetected = longFrames.length > 0 ? [...longFrames] : [...frameTimes]
                // Sort by time
                allDetected.sort((a, b) => a.startTime - b.startTime)
                resolve(allDetected)
              }, 500)
            }
          }, scrollInterval)
        })
      }, { threshold: opts.threshold, scrollDuration: opts.scrollDuration })

      allSpikes.push(...spikes)

      if (spikes.length > 0) {
        for (const spike of spikes) {
          if (spike.error) {
            log.warn(spike.error)
          } else {
            const bar = '█'.repeat(Math.min(40, Math.round(spike.duration / 10)))
            const durStr = `${spike.duration}ms`.padEnd(10)
            const typeStr = spike.type.padEnd(22)
            log.bad(`${durStr} ${typeStr} ${color('31', bar)}`)
          }
        }
      } else {
        log.good('No spikes detected in this run.')
      }

      if (opts.runs > 1 && run < opts.runs - 1) {
        await sleep(1000)
      }
    }

    // ── Step 3: Verdict ────────────────────────────────────
    console.log()
    console.log('─'.repeat(60))

    const errorSpikes = allSpikes.filter((s) => s.error)
    const realSpikes = allSpikes.filter((s) => !s.error)

    if (errorSpikes.length > 0) {
      log.warn('Errors occurred during measurement, result may be unreliable')
      process.exit(125)
    }

    const avgDuration = realSpikes.length > 0
      ? realSpikes.reduce((sum, s) => sum + s.duration, 0) / realSpikes.length
      : 0
    const maxDuration = realSpikes.length > 0
      ? Math.max(...realSpikes.map((s) => s.duration))
      : 0

    log.info(`Total spikes (>${opts.threshold}ms): ${realSpikes.length}`)
    if (realSpikes.length > 0) {
      log.info(`Average spike duration: ${Math.round(avgDuration)}ms`)
      log.info(`Worst spike: ${maxDuration}ms`)
    }
    console.log()

    if (realSpikes.length >= opts.minSpikes) {
      log.bad(`VERDICT: BAD — ${realSpikes.length} lag spikes detected (threshold: ${opts.minSpikes})`)
      await browser.close()
      process.exit(1)
    } else {
      log.good(`VERDICT: GOOD — ${realSpikes.length < 1 ? 'no' : 'only ' + realSpikes.length} spike(s), below threshold of ${opts.minSpikes}`)
      await browser.close()
      process.exit(0)
    }
  } catch (e) {
    log.warn(`Test failed with error: ${e.message}`)
    log.dim(e.stack)
    log.warn('Exiting with 125 (skip) for git bisect')
    await browser.close()
    process.exit(125)
  }
}

main()

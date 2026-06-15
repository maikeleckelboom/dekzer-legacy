import {
  _electron as electron,
  type ConsoleMessage,
  type ElectronApplication,
  type Page,
  type TestInfo
} from '@playwright/test'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { test as base, expect } from './filesystem.fixture'

export type ElectronAppHarness = {
  readonly userDataPath: string
  readonly app: () => ElectronApplication
  readonly firstWindow: () => Promise<Page>
  readonly window: () => Page
  readonly relaunch: () => Promise<Page>
  readonly close: () => Promise<void>
}

export type ElectronAppFixtures = {
  readonly electronApp: ElectronAppHarness
  readonly mainWindow: Page
}

type CapturedLogs = {
  readonly appProcess: string[]
  readonly mainConsole: string[]
  readonly rendererConsole: string[]
  readonly pageErrors: string[]
}

const desktopRoot = resolve(process.cwd())
const builtMainPath = join(desktopRoot, 'out', 'main', 'index.js')
const libraryUserDataPathEnvVar = 'DESKTOP_LIBRARY_USER_DATA_PATH'

export const test = base.extend<ElectronAppFixtures>({
  electronApp: async ({ libraryFilesystem }, use, testInfo) => {
    assertBuiltAppExists()

    const logs: CapturedLogs = {
      appProcess: [],
      mainConsole: [],
      rendererConsole: [],
      pageErrors: []
    }
    const trackedPages: Page[] = []
    const trackedPageSet = new WeakSet<Page>()
    let currentApp: ElectronApplication | undefined
    let currentWindow: Page | undefined
    let launchCount = 0

    async function launch(): Promise<Page> {
      launchCount += 1
      const app = await electron.launch({
        executablePath: await resolveElectronExecutablePath(),
        args: [desktopRoot],
        cwd: desktopRoot,
        env: {
          ...process.env,
          [libraryUserDataPathEnvVar]: libraryFilesystem.userDataPath,
          ELECTRON_DISABLE_SECURITY_WARNINGS: 'true'
        },
        timeout: 30_000
      })

      currentApp = app
      wireApp(app, logs, trackedPages, trackedPageSet, launchCount)
      currentWindow = await app.firstWindow({ timeout: 30_000 })
      trackPage(currentWindow, trackedPages, trackedPageSet, logs, launchCount)
      return currentWindow
    }

    async function close(): Promise<void> {
      const app = currentApp
      currentApp = undefined
      currentWindow = undefined

      if (app === undefined) {
        return
      }

      const childProcess = app.process()
      const closed = app.waitForEvent('close', { timeout: 10_000 }).catch(() => undefined)
      const closeRequested = app.close().catch(() => undefined)

      await Promise.race([closed, closeRequested, delay(10_000)])

      if (!childProcess.killed && childProcess.exitCode === null) {
        childProcess.kill()
      }
    }

    const harness: ElectronAppHarness = {
      userDataPath: libraryFilesystem.userDataPath,
      app: () => {
        if (currentApp === undefined) {
          throw new Error('Electron app is not running.')
        }

        return currentApp
      },
      firstWindow: async () => {
        if (currentWindow !== undefined && !currentWindow.isClosed()) {
          return currentWindow
        }

        const app = harness.app()
        currentWindow = await app.firstWindow({ timeout: 30_000 })
        trackPage(currentWindow, trackedPages, trackedPageSet, logs, launchCount)
        return currentWindow
      },
      window: () => {
        if (currentWindow === undefined || currentWindow.isClosed()) {
          throw new Error('Electron main window is not available.')
        }

        return currentWindow
      },
      relaunch: async () => {
        await close()
        return launch()
      },
      close
    }

    await launch()
    await use(harness)
    await close()
    await attachFailureLogs(testInfo, logs)
  },

  mainWindow: async ({ electronApp }, use, testInfo) => {
    const page = await electronApp.firstWindow()
    const tracePath = testInfo.outputPath('trace.zip')
    const shouldTrace = testInfo.retry > 0

    if (shouldTrace) {
      await page.context().tracing.start({ screenshots: true, snapshots: true, sources: true })
    }

    await use(page)

    const failed = didFail(testInfo)

    if (failed && !page.isClosed()) {
      await page
        .screenshot({ fullPage: true, timeout: 5_000 })
        .then((body) =>
          testInfo.attach('renderer-screenshot', {
            body,
            contentType: 'image/png'
          })
        )
        .catch(() => undefined)
    }

    if (shouldTrace) {
      if (failed) {
        await page.context().tracing.stop({ path: tracePath })
        await testInfo.attach('playwright-trace', {
          path: tracePath,
          contentType: 'application/zip'
        })
      } else {
        await page.context().tracing.stop()
      }
    }
  }
})

export { expect }

function assertBuiltAppExists(): void {
  if (!existsSync(builtMainPath)) {
    throw new Error(
      `Built Electron main entry was not found at ${builtMainPath}. Run pnpm run build before pnpm run test:e2e, or use pnpm run verify:e2e.`
    )
  }
}

async function resolveElectronExecutablePath(): Promise<string> {
  const electronModule = (await import('electron')) as unknown as {
    readonly default: unknown
  }
  const electronPath = electronModule.default

  if (typeof electronPath !== 'string' || electronPath.length === 0) {
    throw new Error('The electron package did not resolve to an executable path.')
  }

  return electronPath
}

function wireApp(
  app: ElectronApplication,
  logs: CapturedLogs,
  trackedPages: Page[],
  trackedPageSet: WeakSet<Page>,
  launchIndex: number
): void {
  const childProcess = app.process()

  childProcess.stdout?.on('data', (chunk: Buffer) => {
    logs.appProcess.push(`[app:${launchIndex}:stdout] ${chunk.toString()}`)
  })
  childProcess.stderr?.on('data', (chunk: Buffer) => {
    logs.appProcess.push(`[app:${launchIndex}:stderr] ${chunk.toString()}`)
  })
  app.on('console', (message: ConsoleMessage) => {
    logs.mainConsole.push(`[main:${launchIndex}:${message.type()}] ${message.text()}`)
  })
  app.on('window', (page: Page) => {
    trackPage(page, trackedPages, trackedPageSet, logs, launchIndex)
  })
}

function trackPage(
  page: Page,
  trackedPages: Page[],
  trackedPageSet: WeakSet<Page>,
  logs: CapturedLogs,
  launchIndex: number
): void {
  if (trackedPageSet.has(page)) {
    return
  }

  trackedPageSet.add(page)
  trackedPages.push(page)

  page.on('console', (message) => {
    logs.rendererConsole.push(`[renderer:${launchIndex}:${message.type()}] ${message.text()}`)
  })
  page.on('pageerror', (error) => {
    logs.pageErrors.push(`[renderer:${launchIndex}:pageerror] ${error.stack ?? error.message}`)
  })
}

async function attachFailureLogs(testInfo: TestInfo, logs: CapturedLogs): Promise<void> {
  if (!didFail(testInfo)) {
    return
  }

  await attachText(testInfo, 'electron-process.log', logs.appProcess)
  await attachText(testInfo, 'electron-main-console.log', logs.mainConsole)
  await attachText(testInfo, 'renderer-console.log', logs.rendererConsole)
  await attachText(testInfo, 'renderer-page-errors.log', logs.pageErrors)
}

async function attachText(
  testInfo: TestInfo,
  name: string,
  lines: readonly string[]
): Promise<void> {
  if (lines.length === 0) {
    return
  }

  await testInfo.attach(name, {
    body: lines.join('\n'),
    contentType: 'text/plain'
  })
}

function didFail(testInfo: TestInfo): boolean {
  return testInfo.status !== undefined && testInfo.status !== testInfo.expectedStatus
}

function delay(ms: number): Promise<void> {
  return new Promise((resolveDelay) => {
    setTimeout(resolveDelay, ms)
  })
}

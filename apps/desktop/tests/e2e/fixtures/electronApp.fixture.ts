import {
  _electron as electron,
  type ConsoleMessage,
  type ElectronApplication,
  type Page,
  type TestInfo
} from '@playwright/test'
import { execFile, type ChildProcess } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { test as base, expect } from './filesystem.fixture'

export type ElectronAppCloseOptions = {
  readonly allowKillFallback?: boolean
  readonly reason?: 'test-request' | 'relaunch' | 'fixture-teardown'
  readonly gracefulTimeoutMs?: number
  readonly killTimeoutMs?: number
}

export type ElectronAppCloseResult = {
  readonly launchIndex: number | null
  readonly graceful: boolean
  readonly killed: boolean
  readonly timedOut: boolean
  readonly exitCode: number | null
  readonly signalCode: NodeJS.Signals | null
}

export type ElectronAppHarness = {
  readonly userDataPath: string
  readonly app: () => ElectronApplication
  readonly firstWindow: () => Promise<Page>
  readonly window: () => Page
  readonly relaunch: () => Promise<Page>
  readonly close: (options?: ElectronAppCloseOptions) => Promise<ElectronAppCloseResult>
}

export type ElectronAppFixtures = {
  readonly electronApp: ElectronAppHarness
  readonly mainWindow: Page
}

type CapturedLogs = {
  readonly stdout: string[]
  readonly stderr: string[]
  readonly mainConsole: string[]
  readonly rendererConsole: string[]
  readonly pageErrors: string[]
  readonly shutdown: string[]
}

type RunningElectronApp = {
  readonly app: ElectronApplication
  readonly cleanup: () => void
  readonly launchIndex: number
}

const desktopRoot = resolve(process.cwd())
const builtMainPath = join(desktopRoot, 'out', 'main', 'index.js')
const libraryUserDataPathEnvVar = 'DESKTOP_LIBRARY_USER_DATA_PATH'
const gracefulCloseTimeoutMs = 10_000
const killExitTimeoutMs = 5_000
const taskKillTimeoutMs = 5_000

export const test = base.extend<ElectronAppFixtures>({
  electronApp: async ({ libraryFilesystem }, use, testInfo) => {
    assertBuiltAppExists()

    const logs: CapturedLogs = {
      stdout: [],
      stderr: [],
      mainConsole: [],
      rendererConsole: [],
      pageErrors: [],
      shutdown: []
    }
    const trackedPageSet = new WeakSet<Page>()
    let currentApp: RunningElectronApp | undefined
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

      currentApp = {
        app,
        cleanup: wireApp(app, logs, trackedPageSet, launchCount),
        launchIndex: launchCount
      }
      currentWindow = await app.firstWindow({ timeout: 30_000 })
      trackPage(currentWindow, trackedPageSet, logs, launchCount)
      return currentWindow
    }

    async function close(options: ElectronAppCloseOptions = {}): Promise<ElectronAppCloseResult> {
      const runningApp = currentApp

      if (runningApp === undefined) {
        return {
          launchIndex: null,
          graceful: true,
          killed: false,
          timedOut: false,
          exitCode: null,
          signalCode: null
        }
      }

      const result = await closeRunningApp(runningApp, logs, options)

      if (result.graceful || result.killed || hasProcessExited(runningApp.app.process())) {
        clearCurrentApp(runningApp)
      }

      if (result.timedOut && options.allowKillFallback !== true) {
        throw new Error(
          `Electron app ${runningApp.launchIndex} did not close gracefully within ${
            options.gracefulTimeoutMs ?? gracefulCloseTimeoutMs
          }ms. The process was left for fixture teardown so the failure remains observable.`
        )
      }

      return result
    }

    function clearCurrentApp(runningApp: RunningElectronApp): void {
      runningApp.cleanup()

      if (currentApp === runningApp) {
        currentApp = undefined
        currentWindow = undefined
      }
    }

    const harness: ElectronAppHarness = {
      userDataPath: libraryFilesystem.userDataPath,
      app: () => {
        if (currentApp === undefined) {
          throw new Error('Electron app is not running.')
        }

        return currentApp.app
      },
      firstWindow: async () => {
        if (currentWindow !== undefined && !currentWindow.isClosed()) {
          return currentWindow
        }

        const app = harness.app()
        currentWindow = await app.firstWindow({ timeout: 30_000 })
        trackPage(currentWindow, trackedPageSet, logs, launchCount)
        return currentWindow
      },
      window: () => {
        if (currentWindow === undefined || currentWindow.isClosed()) {
          throw new Error('Electron main window is not available.')
        }

        return currentWindow
      },
      relaunch: async () => {
        await close({ reason: 'relaunch' })
        return launch()
      },
      close
    }

    let testError: unknown
    let teardownError: unknown

    try {
      await launch()
      await use(harness)
    } catch (error) {
      testError = error
    } finally {
      try {
        const teardownResult = await close({
          allowKillFallback: true,
          reason: 'fixture-teardown'
        })

        if (!teardownResult.graceful && teardownResult.launchIndex !== null) {
          teardownError = new Error(
            `Electron app ${teardownResult.launchIndex} did not close gracefully during teardown.`
          )
        }
      } catch (error) {
        teardownError = error
      }

      await attachFailureLogs(
        testInfo,
        logs,
        testError !== undefined || teardownError !== undefined
      )
    }

    if (testError !== undefined) {
      throw testError
    }

    if (teardownError !== undefined) {
      throw teardownError
    }
  },

  mainWindow: async ({ electronApp }, use, testInfo) => {
    const page = await electronApp.firstWindow()
    const tracePath = testInfo.outputPath('trace.zip')
    let traceStarted = false

    traceStarted = await startTracing(page, testInfo)

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

    await stopTracing(page, testInfo, traceStarted, failed, tracePath)
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
  trackedPageSet: WeakSet<Page>,
  launchIndex: number
): () => void {
  const childProcess = app.process()
  const onStdout = (chunk: Buffer): void => {
    logs.stdout.push(`[app:${launchIndex}:stdout] ${chunk.toString()}`)
  }
  const onStderr = (chunk: Buffer): void => {
    logs.stderr.push(`[app:${launchIndex}:stderr] ${chunk.toString()}`)
  }
  const onMainConsole = (message: ConsoleMessage): void => {
    logs.mainConsole.push(`[main:${launchIndex}:${message.type()}] ${message.text()}`)
  }
  const onWindow = (page: Page): void => {
    trackPage(page, trackedPageSet, logs, launchIndex)
  }

  childProcess.stdout?.on('data', onStdout)
  childProcess.stderr?.on('data', onStderr)
  app.on('console', onMainConsole)
  app.on('window', onWindow)

  return () => {
    childProcess.stdout?.off('data', onStdout)
    childProcess.stderr?.off('data', onStderr)
    app.off('console', onMainConsole)
    app.off('window', onWindow)
  }
}

function trackPage(
  page: Page,
  trackedPageSet: WeakSet<Page>,
  logs: CapturedLogs,
  launchIndex: number
): void {
  if (trackedPageSet.has(page)) {
    return
  }

  trackedPageSet.add(page)
  const onConsole = (message: ConsoleMessage): void => {
    logs.rendererConsole.push(`[renderer:${launchIndex}:${message.type()}] ${message.text()}`)
  }
  const onPageError = (error: Error): void => {
    logs.pageErrors.push(`[renderer:${launchIndex}:pageerror] ${error.stack ?? error.message}`)
  }

  page.on('console', onConsole)
  page.on('pageerror', onPageError)
  page.once('close', () => {
    page.off('console', onConsole)
    page.off('pageerror', onPageError)
  })
}

async function closeRunningApp(
  runningApp: RunningElectronApp,
  logs: CapturedLogs,
  options: ElectronAppCloseOptions
): Promise<ElectronAppCloseResult> {
  const app = runningApp.app
  const childProcess = app.process()
  const gracefulTimeout = options.gracefulTimeoutMs ?? gracefulCloseTimeoutMs
  const killTimeout = options.killTimeoutMs ?? killExitTimeoutMs
  const reason = options.reason ?? 'test-request'

  logs.shutdown.push(
    `[shutdown:${runningApp.launchIndex}] close requested; reason=${reason}; ` +
      `allowKillFallback=${String(options.allowKillFallback === true)}; ` +
      formatProcessState(childProcess)
  )

  const closeEvent = app.waitForEvent('close', { timeout: gracefulTimeout }).then(
    () => true,
    (error: unknown) => {
      logs.shutdown.push(
        `[shutdown:${runningApp.launchIndex}] graceful close was not observed within ` +
          `${gracefulTimeout}ms: ${formatUnknownError(error)}`
      )
      return false
    }
  )

  void app.close().catch((error: unknown) => {
    logs.shutdown.push(
      `[shutdown:${runningApp.launchIndex}] app.close() rejected: ${formatUnknownError(error)}`
    )
  })

  const graceful = await closeEvent

  if (graceful) {
    logs.shutdown.push(
      `[shutdown:${runningApp.launchIndex}] graceful close observed; ${formatProcessState(
        childProcess
      )}`
    )
    return closeResult(runningApp.launchIndex, true, false, false, childProcess)
  }

  if (options.allowKillFallback !== true) {
    logs.shutdown.push(
      `[shutdown:${runningApp.launchIndex}] leaving process running for teardown fallback; ` +
        formatProcessState(childProcess)
    )
    return closeResult(runningApp.launchIndex, false, false, true, childProcess)
  }

  const killed = await forceKillProcessTree(childProcess, logs, runningApp.launchIndex)
  const exited = await waitForProcessExit(childProcess, killTimeout)

  logs.shutdown.push(
    `[shutdown:${runningApp.launchIndex}] forced teardown fallback completed; ` +
      `killRequested=${String(killed)}; exited=${String(exited)}; ${formatProcessState(
        childProcess
      )}`
  )

  return closeResult(runningApp.launchIndex, false, killed, true, childProcess)
}

async function forceKillProcessTree(
  childProcess: ChildProcess,
  logs: CapturedLogs,
  launchIndex: number
): Promise<boolean> {
  if (hasProcessExited(childProcess)) {
    logs.shutdown.push(`[shutdown:${launchIndex}] process already exited before forced kill.`)
    return false
  }

  if (process.platform === 'win32' && childProcess.pid !== undefined) {
    try {
      const result = await execFileWithTimeout(
        'taskkill',
        ['/pid', childProcess.pid.toString(), '/T', '/F'],
        taskKillTimeoutMs
      )

      if (result.stdout.trim().length > 0) {
        logs.shutdown.push(`[shutdown:${launchIndex}] taskkill stdout: ${result.stdout.trim()}`)
      }
      if (result.stderr.trim().length > 0) {
        logs.shutdown.push(`[shutdown:${launchIndex}] taskkill stderr: ${result.stderr.trim()}`)
      }

      return true
    } catch (error) {
      logs.shutdown.push(
        `[shutdown:${launchIndex}] taskkill fallback failed: ${formatUnknownError(error)}`
      )

      if (hasProcessExited(childProcess)) {
        return false
      }
    }
  }

  if (process.platform !== 'win32' && childProcess.pid !== undefined) {
    const descendantPids = await collectDescendantPids(childProcess.pid)

    for (const pid of descendantPids) {
      try {
        process.kill(pid, 'SIGKILL')
        logs.shutdown.push(`[shutdown:${launchIndex}] sent SIGKILL to descendant pid=${pid}.`)
      } catch (error) {
        logs.shutdown.push(
          `[shutdown:${launchIndex}] failed to kill descendant pid=${pid}: ${formatUnknownError(
            error
          )}`
        )
      }
    }
  }

  const signalSent = childProcess.kill('SIGKILL')
  logs.shutdown.push(
    `[shutdown:${launchIndex}] child_process.kill(SIGKILL) fallback sent=${String(signalSent)}`
  )
  return signalSent
}

async function collectDescendantPids(parentPid: number): Promise<number[]> {
  const directChildren = await findDirectChildPids(parentPid)
  const descendants: number[] = []

  for (const childPid of directChildren) {
    descendants.push(...(await collectDescendantPids(childPid)), childPid)
  }

  return descendants
}

async function findDirectChildPids(parentPid: number): Promise<number[]> {
  try {
    const result = await execFileWithTimeout('pgrep', ['-P', parentPid.toString()], 1_000)

    return result.stdout
      .split(/\s+/)
      .map((value) => Number(value))
      .filter((pid) => Number.isInteger(pid) && pid > 0)
  } catch {
    return []
  }
}

function execFileWithTimeout(
  file: string,
  args: readonly string[],
  timeoutMs: number
): Promise<{ readonly stdout: string; readonly stderr: string }> {
  return new Promise((resolveExec, rejectExec) => {
    execFile(file, [...args], { timeout: timeoutMs }, (error, stdout, stderr) => {
      if (error !== null) {
        rejectExec(error)
        return
      }

      resolveExec({ stdout, stderr })
    })
  })
}

function waitForProcessExit(childProcess: ChildProcess, timeoutMs: number): Promise<boolean> {
  if (hasProcessExited(childProcess)) {
    return Promise.resolve(true)
  }

  return new Promise((resolveExit) => {
    const timeoutId = setTimeout(() => {
      childProcess.off('exit', onExit)
      childProcess.off('error', onExit)
      resolveExit(false)
    }, timeoutMs)

    const onExit = (): void => {
      clearTimeout(timeoutId)
      childProcess.off('exit', onExit)
      childProcess.off('error', onExit)
      resolveExit(true)
    }

    childProcess.once('exit', onExit)
    childProcess.once('error', onExit)
  })
}

function closeResult(
  launchIndex: number,
  graceful: boolean,
  killed: boolean,
  timedOut: boolean,
  childProcess: ChildProcess
): ElectronAppCloseResult {
  return {
    launchIndex,
    graceful,
    killed,
    timedOut,
    exitCode: childProcess.exitCode,
    signalCode: childProcess.signalCode
  }
}

function hasProcessExited(childProcess: ChildProcess): boolean {
  return childProcess.exitCode !== null || childProcess.signalCode !== null
}

function formatProcessState(childProcess: ChildProcess): string {
  return `pid=${childProcess.pid ?? 'unknown'}; exitCode=${
    childProcess.exitCode ?? 'null'
  }; signalCode=${childProcess.signalCode ?? 'null'}; killed=${String(childProcess.killed)}`
}

async function startTracing(page: Page, testInfo: TestInfo): Promise<boolean> {
  try {
    await page.context().tracing.start({ screenshots: true, snapshots: true, sources: true })
    return true
  } catch (error) {
    await testInfo.attach('playwright-trace-start-error.log', {
      body: formatUnknownError(error),
      contentType: 'text/plain'
    })
    return false
  }
}

async function stopTracing(
  page: Page,
  testInfo: TestInfo,
  traceStarted: boolean,
  failed: boolean,
  tracePath: string
): Promise<void> {
  if (!traceStarted) {
    return
  }

  try {
    if (failed) {
      await page.context().tracing.stop({ path: tracePath })
      await testInfo.attach('playwright-trace', {
        path: tracePath,
        contentType: 'application/zip'
      })
      return
    }

    await page.context().tracing.stop()
  } catch (error) {
    if (failed) {
      await testInfo.attach('playwright-trace-stop-error.log', {
        body: formatUnknownError(error),
        contentType: 'text/plain'
      })
    }
  }
}

async function attachFailureLogs(
  testInfo: TestInfo,
  logs: CapturedLogs,
  force = false
): Promise<void> {
  if (!force && !didFail(testInfo)) {
    return
  }

  await attachText(testInfo, 'electron-stdout.log', logs.stdout)
  await attachText(testInfo, 'electron-stderr.log', logs.stderr)
  await attachText(testInfo, 'electron-main-console.log', logs.mainConsole)
  await attachText(testInfo, 'renderer-console.log', logs.rendererConsole)
  await attachText(testInfo, 'renderer-page-errors.log', logs.pageErrors)
  await attachText(testInfo, 'electron-shutdown.log', logs.shutdown)
}

async function attachText(
  testInfo: TestInfo,
  name: string,
  lines: readonly string[]
): Promise<void> {
  await testInfo.attach(name, {
    body: lines.length === 0 ? '(no entries)' : lines.join('\n'),
    contentType: 'text/plain'
  })
}

function didFail(testInfo: TestInfo): boolean {
  return testInfo.status !== undefined && testInfo.status !== testInfo.expectedStatus
}

function formatUnknownError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? error.message
  }

  return String(error)
}

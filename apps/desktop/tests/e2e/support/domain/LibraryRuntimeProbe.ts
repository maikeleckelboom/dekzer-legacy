import { expect, type Page } from '@playwright/test'

import type {
  ContentsReadRequest,
  ContentsReadResult
} from '../../../../src/shared/library/contents/read'
import type { ReadLocalRootsOutcome } from '../../../../src/shared/library/roots/read'
import type { LibraryViewStateReadResult } from '../../../../src/shared/library/viewState/persistence'
import type { RendererApi } from '../../../../src/shared/rendererApi'

export type LibraryRuntimeProbeFailure = {
  readonly state: 'probeFailed'
  readonly error: string
}

export type LibraryRuntimeSourceSnapshot = {
  readonly sourceId: string
  readonly lifecycle: unknown
  readonly integrity: unknown
  readonly activity: unknown
  readonly maintenance: unknown
}

export class LibraryRuntimeProbe {
  constructor(private readonly page: Page) {}

  async hasPreloadApi(): Promise<boolean> {
    return this.page
      .evaluate(() => {
        const api = (window as unknown as { readonly dekzer?: RendererApi }).dekzer
        return api?.library !== undefined
      })
      .catch(() => false)
  }

  async waitForPreloadApi(): Promise<void> {
    await this.page.waitForFunction(
      () => {
        const api = (window as unknown as { readonly dekzer?: RendererApi }).dekzer
        return api?.library !== undefined
      },
      undefined,
      { timeout: 30_000 }
    )
  }

  async readHostStatus(): Promise<unknown | LibraryRuntimeProbeFailure> {
    return this.safeEvaluate('read library host status', async () => {
      const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
      return api.library.host.getStatus()
    })
  }

  async readLocalRoots(): Promise<ReadLocalRootsOutcome | LibraryRuntimeProbeFailure> {
    return this.safeEvaluate('read local roots', async () => {
      const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
      return api.library.roots.readLocalRoots()
    })
  }

  async readViewState(): Promise<LibraryViewStateReadResult | LibraryRuntimeProbeFailure> {
    return this.safeEvaluate('read library view state', async () => {
      const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
      return api.library.viewState.readViewState()
    })
  }

  async readContents(
    request: ContentsReadRequest
  ): Promise<ContentsReadResult | LibraryRuntimeProbeFailure> {
    return this.safeEvaluateWithArg(
      'read library contents',
      async (contentsRequest) => {
        const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
        return api.library.contents.read(contentsRequest)
      },
      request
    )
  }

  async readSourceSnapshot(
    sourceId: string
  ): Promise<LibraryRuntimeSourceSnapshot | LibraryRuntimeProbeFailure> {
    return this.safeEvaluateWithArg(
      'read source status snapshots',
      async (id) => {
        const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
        const [lifecycle, integrity, activity, maintenance] = await Promise.all([
          api.library.sourceLifecycle.readSourceLifecycle({ sourceId: id }),
          api.library.sourceIntegrity.readSourceIntegrity({ sourceId: id }),
          api.library.sourceActivity.readSourceActivity({ sourceId: id }),
          api.library.sourceMaintenance.readSourceMaintenance({ sourceId: id })
        ])

        return {
          sourceId: id,
          lifecycle,
          integrity,
          activity,
          maintenance
        }
      },
      sourceId
    )
  }

  async sourceIdForAdmittedPath(sourcePath: string): Promise<string | undefined> {
    const admittedPathKey = normalizeLocalSourcePath(sourcePath)
    const result = await this.readLocalRoots()

    if (result.state === 'probeFailed' || result.state !== 'read') {
      return undefined
    }

    return result.roots.find(
      (root) => normalizeLocalSourcePath(root.admittedRootPath) === admittedPathKey
    )?.rootId
  }

  async admittedPathCount(sourcePath: string): Promise<number> {
    const admittedPathKey = normalizeLocalSourcePath(sourcePath)
    const result = await this.readLocalRoots()

    if (result.state === 'probeFailed' || result.state !== 'read') {
      return 0
    }

    return result.roots.filter(
      (root) => normalizeLocalSourcePath(root.admittedRootPath) === admittedPathKey
    ).length
  }

  async waitForAdmittedPath(sourcePath: string): Promise<string> {
    await expect
      .poll(() => this.sourceIdForAdmittedPath(sourcePath), { timeout: 30_000 })
      .not.toBeUndefined()

    const sourceId = await this.sourceIdForAdmittedPath(sourcePath)

    if (sourceId === undefined) {
      throw new Error(`Admitted source was not found for path: ${sourcePath}`)
    }

    return sourceId
  }

  async waitForSourcePathForgotten(sourcePath: string): Promise<void> {
    await expect
      .poll(() => this.sourceIdForAdmittedPath(sourcePath), { timeout: 30_000 })
      .toBeUndefined()
  }

  async waitForSingleAdmittedPath(sourcePath: string): Promise<void> {
    await expect.poll(() => this.admittedPathCount(sourcePath), { timeout: 30_000 }).toBe(1)
  }

  async waitForUnavailableSource(sourceId: string): Promise<void> {
    await expect.poll(() => this.isSourceUnavailable(sourceId), { timeout: 30_000 }).toBe(true)
  }

  async waitForTerminalSourceReadiness(sourceId: string): Promise<void> {
    await this.page.waitForFunction(
      async (id) => {
        const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
        const [lifecycle, integrity, activity] = await Promise.all([
          api.library.sourceLifecycle.readSourceLifecycle({ sourceId: id }),
          api.library.sourceIntegrity.readSourceIntegrity({ sourceId: id }),
          api.library.sourceActivity.readSourceActivity({ sourceId: id })
        ])

        if (
          lifecycle.state !== 'ready' ||
          integrity.state !== 'ready' ||
          activity.state !== 'ready'
        ) {
          return false
        }

        return (
          lifecycle.lifecycle.scanPhase === 'complete' &&
          integrity.integrity.coverageIntegrity.state === 'complete' &&
          integrity.integrity.coverageIntegrity.subtreeCoverageComplete &&
          activity.activity.scan.state !== 'running'
        )
      },
      sourceId,
      { timeout: 30_000 }
    )
  }

  async isSourceUnavailable(sourceId: string): Promise<boolean> {
    const [roots, snapshot] = await Promise.all([
      this.readLocalRoots(),
      this.readSourceSnapshot(sourceId)
    ])

    if (roots.state !== 'probeFailed' && roots.state === 'read') {
      const root = roots.roots.find((candidate) => candidate.rootId === sourceId)
      if (root?.availability === 'unavailable') {
        return true
      }
    }

    if (isProbeFailure(snapshot)) {
      return false
    }

    return sourceSnapshotLooksUnavailable(snapshot)
  }

  private async safeEvaluate<T>(
    description: string,
    callback: () => Promise<T> | T
  ): Promise<T | LibraryRuntimeProbeFailure> {
    try {
      return await this.page.evaluate(callback)
    } catch (error) {
      return {
        state: 'probeFailed',
        error: `${description} failed: ${formatUnknownError(error)}`
      }
    }
  }

  private async safeEvaluateWithArg<T, Arg>(
    description: string,
    callback: (arg: Arg) => Promise<T> | T,
    arg: Arg
  ): Promise<T | LibraryRuntimeProbeFailure> {
    try {
      return await this.page.evaluate(
        callback as (browserArg: unknown) => Promise<T> | T,
        arg as unknown
      )
    } catch (error) {
      return {
        state: 'probeFailed',
        error: `${description} failed: ${formatUnknownError(error)}`
      }
    }
  }
}

export function isProbeFailure(value: unknown): value is LibraryRuntimeProbeFailure {
  return isRecord(value) && value.state === 'probeFailed' && typeof value.error === 'string'
}

export function sourceSnapshotLooksUnavailable(snapshot: LibraryRuntimeSourceSnapshot): boolean {
  const lifecycle = snapshot.lifecycle
  if (isRecord(lifecycle) && lifecycle.state === 'ready' && isRecord(lifecycle.lifecycle)) {
    if (
      lifecycle.lifecycle.accessState === 'missing' ||
      lifecycle.lifecycle.accessState === 'blocked'
    ) {
      return true
    }
    if (
      lifecycle.lifecycle.sourceClass !== 'internal' &&
      lifecycle.lifecycle.mountStatus !== 'mounted' &&
      lifecycle.lifecycle.mountStatus !== 'unknown'
    ) {
      return true
    }
  }

  const integrity = snapshot.integrity
  if (isRecord(integrity) && integrity.state === 'ready' && isRecord(integrity.integrity)) {
    const coverageIntegrity = integrity.integrity.coverageIntegrity
    if (
      isRecord(coverageIntegrity) &&
      (coverageIntegrity.state === 'sourceUnavailable' ||
        coverageIntegrity.state === 'locationMissing' ||
        coverageIntegrity.state === 'blocked')
    ) {
      return true
    }

    const sourceAvailability = integrity.integrity.sourceAvailability
    if (
      isRecord(sourceAvailability) &&
      (sourceAvailability.state === 'missing' ||
        sourceAvailability.state === 'blocked' ||
        sourceAvailability.state === 'unavailable' ||
        sourceAvailability.state === 'notFound')
    ) {
      return true
    }
  }

  const activity = snapshot.activity
  if (isRecord(activity) && activity.state === 'ready' && isRecord(activity.activity)) {
    const browse = activity.activity.browse
    if (
      isRecord(browse) &&
      (browse.state === 'missing' || browse.state === 'blocked' || browse.state === 'unavailable')
    ) {
      return true
    }
  }

  return false
}

export function normalizeLocalSourcePath(path: string): string {
  return process.platform === 'win32' ? windowsLocalPathKey(path) : posixLocalPathKey(path)
}

function windowsLocalPathKey(path: string): string {
  return path
    .replace(/^\\\\\?\\UNC\\/i, '\\\\')
    .replace(/^\\\\\?\\/i, '')
    .replaceAll('/', '\\')
    .replace(/\\+$/, '')
    .toLowerCase()
}

function posixLocalPathKey(path: string): string {
  const trimmed = path.replace(/\/+$/, '')

  return trimmed.length === 0 ? '/' : trimmed
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function formatUnknownError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? error.message
  }

  return String(error)
}

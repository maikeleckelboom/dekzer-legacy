import { expect, type Page } from '@playwright/test'

import type { RendererApi } from '../../../../src/shared/rendererApi'

export class LibraryRuntimeProbe {
  constructor(private readonly page: Page) {}

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

  async sourceIdForAdmittedPath(sourcePath: string): Promise<string | undefined> {
    const admittedPathKey = normalizeLocalSourcePath(sourcePath)
    const roots = await this.page.evaluate(async () => {
      const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
      const result = await api.library.roots.readLocalRoots()

      if (result.state !== 'read') {
        return []
      }

      return result.roots.map((root) => ({
        rootId: root.rootId,
        admittedRootPath: root.admittedRootPath
      }))
    })

    return roots.find((root) => normalizeLocalSourcePath(root.admittedRootPath) === admittedPathKey)
      ?.rootId
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

import type { Locator, Page } from '@playwright/test'

import type { RendererApi } from '../../../src/shared/rendererApi'
import { ContentsPanel } from './ContentsPanel'
import { LibraryTree } from './LibraryTree'
import { SourceStatusPanel } from './SourceStatusPanel'

export type LibraryBrowseProfileLabel = 'Audio' | 'Audio + Video' | 'All Files'

export class LibraryPanel {
  private readonly page: Page
  readonly root: Locator
  readonly title: Locator
  readonly addSourceButton: Locator
  readonly addMusicFolderButton: Locator
  readonly libraryBrowseButton: Locator
  readonly indexedContentsViewButton: Locator
  readonly tree: LibraryTree
  readonly contents: ContentsPanel
  readonly sourceStatus: SourceStatusPanel

  constructor(page: Page) {
    this.page = page
    this.root = page.getByRole('region', { name: 'Library panel' })
    this.title = this.root.getByRole('heading', { level: 2 })
    this.addSourceButton = this.root.getByRole('button', { name: 'Add Source' })
    this.addMusicFolderButton = this.root.getByRole('button', { name: 'Add music folder' }).first()
    this.libraryBrowseButton = this.root.getByRole('button', { name: 'Library Browse' })
    this.indexedContentsViewButton = this.root.getByRole('button', {
      name: 'Indexed contents view'
    })
    this.tree = new LibraryTree(this.root)
    this.contents = new ContentsPanel(this.root)
    this.sourceStatus = new SourceStatusPanel(this.root, page)
  }

  async openAddSource(): Promise<void> {
    await this.addSourceButton.click()
  }

  async openAddSourceIfNeeded(): Promise<void> {
    if (await this.addSourceButton.isVisible()) {
      await this.openAddSource()
    }
  }

  async addMusicFolder(): Promise<void> {
    await this.addMusicFolderButton.click()
  }

  async openLibraryBrowseIfNeeded(): Promise<void> {
    if (await this.libraryBrowseButton.isVisible()) {
      await this.libraryBrowseButton.click()
    }
  }

  async selectBrowseProfile(label: LibraryBrowseProfileLabel): Promise<void> {
    await this.indexedContentsViewButton.click()
    await this.root
      .getByRole('listbox', { name: 'Indexed contents view' })
      .getByRole('option', { name: label, exact: true })
      .click()
  }

  async sourceIdForAdmittedPath(sourcePath: string): Promise<string | undefined> {
    return this.page.evaluate(async (admittedRootPath) => {
      const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
      const localPathKey = (path: string): string =>
        path
          .replace(/^\\\\\?\\UNC\\/i, '\\\\')
          .replace(/^\\\\\?\\/i, '')
          .replaceAll('/', '\\')
          .replace(/\\+$/, '')
          .toLowerCase()
      const roots = await api.library.roots.readLocalRoots()

      if (roots.state !== 'read') {
        return undefined
      }

      const admittedPathKey = localPathKey(admittedRootPath)
      return roots.roots.find((root) => localPathKey(root.admittedRootPath) === admittedPathKey)
        ?.rootId
    }, sourcePath)
  }

  async waitForAdmittedPath(sourcePath: string): Promise<string> {
    await this.page.waitForFunction(
      async (admittedRootPath) => {
        const api = (window as unknown as { readonly dekzer: RendererApi }).dekzer
        const localPathKey = (path: string): string =>
          path
            .replace(/^\\\\\?\\UNC\\/i, '\\\\')
            .replace(/^\\\\\?\\/i, '')
            .replaceAll('/', '\\')
            .replace(/\\+$/, '')
            .toLowerCase()
        const roots = await api.library.roots.readLocalRoots()

        if (roots.state !== 'read') {
          return false
        }

        const admittedPathKey = localPathKey(admittedRootPath)
        return (
          roots.roots.find((root) => localPathKey(root.admittedRootPath) === admittedPathKey)
            ?.rootId ?? false
        )
      },
      sourcePath,
      { timeout: 30_000 }
    )

    const sourceId = await this.sourceIdForAdmittedPath(sourcePath)

    if (sourceId === undefined) {
      throw new Error(`Admitted source was not found for path: ${sourcePath}`)
    }

    return sourceId
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

import type { Locator, Page } from '@playwright/test'

import { ContentsPanel } from './ContentsPanel'
import { LibraryTree } from './LibraryTree'
import { SourceStatusPanel } from './SourceStatusPanel'

export type LibraryBrowseProfileLabel = 'Audio' | 'Audio + Video' | 'All Files'

export class LibraryPanel {
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
}

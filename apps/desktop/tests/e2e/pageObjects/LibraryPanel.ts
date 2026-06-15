import type { Locator, Page } from '@playwright/test'

import { ContentsPanel } from './ContentsPanel'
import { LibraryTree } from './LibraryTree'
import { SourceStatusPanel } from './SourceStatusPanel'

export class LibraryPanel {
  readonly root: Locator
  readonly title: Locator
  readonly addSourceButton: Locator
  readonly tree: LibraryTree
  readonly contents: ContentsPanel
  readonly sourceStatus: SourceStatusPanel

  constructor(page: Page) {
    this.root = page.getByRole('region', { name: 'Library panel' })
    this.title = this.root.getByRole('heading', { level: 2 })
    this.addSourceButton = this.root.getByRole('button', { name: 'Add Source' })
    this.tree = new LibraryTree(this.root)
    this.contents = new ContentsPanel(this.root)
    this.sourceStatus = new SourceStatusPanel(this.root)
  }

  async openAddSource(): Promise<void> {
    await this.addSourceButton.click()
  }

  async openAddSourceIfNeeded(): Promise<void> {
    if (await this.addSourceButton.isVisible()) {
      await this.openAddSource()
    }
  }
}

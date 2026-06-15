import { expect, type Locator, type Page } from '@playwright/test'

import type { LibraryBrowseProfileLabel } from '../domain/LibraryContracts'
import { button, region } from '../locators/locatorPolicy'
import { AddSourceSurface } from './AddSourceSurface'
import { ContentsSurface } from './ContentsSurface'
import { LibraryBrowseSurface } from './LibraryBrowseSurface'
import { ProfileMenu } from './ProfileMenu'
import { SearchSurface } from './SearchSurface'
import { SourceStatusSurface } from './SourceStatusSurface'

export class LibraryPanel {
  readonly root: Locator
  readonly title: Locator
  readonly addSourceButton: Locator
  readonly addMusicFolderButton: Locator
  readonly libraryBrowseButton: Locator
  readonly indexedContentsViewButton: Locator
  readonly addSource: AddSourceSurface
  readonly browse: LibraryBrowseSurface
  readonly tree: LibraryBrowseSurface
  readonly contents: ContentsSurface
  readonly sourceStatus: SourceStatusSurface
  readonly profileMenu: ProfileMenu
  readonly search: SearchSurface

  constructor(page: Page) {
    this.root = region(page, 'Library panel')
    this.title = this.root.getByRole('heading', { level: 2 })
    this.addSourceButton = button(this.root, 'Add Source')
    this.libraryBrowseButton = button(this.root, 'Library Browse')
    this.indexedContentsViewButton = button(this.root, 'Indexed contents view')
    this.addSource = new AddSourceSurface(this.root)
    this.addMusicFolderButton = this.addSource.addMusicFolderButton
    this.browse = new LibraryBrowseSurface(this.root)
    this.tree = this.browse
    this.contents = new ContentsSurface(this.root)
    this.sourceStatus = new SourceStatusSurface(this.root, page)
    this.profileMenu = new ProfileMenu(this.root)
    this.search = new SearchSurface(this.root)
  }

  async expectVisible(): Promise<void> {
    await expect(this.root).toBeVisible()
    await expect(this.title).toHaveText(/^(Library Browse|Add Source)$/)
  }

  async expectAddSourceEntryReachable(): Promise<void> {
    if (await this.addSourceButton.isVisible()) {
      await expect(this.addSourceButton).toBeEnabled()
      return
    }

    await this.expectAddSourceSurfaceVisible()
  }

  async expectAddSourceSurfaceVisible(): Promise<void> {
    await expect(this.title).toHaveText('Add Source')
    await this.addSource.expectVisible()
  }

  async expectLibraryBrowseSurfaceVisible(): Promise<void> {
    await expect(this.title).toHaveText('Library Browse')
    await this.browse.expectVisible()
  }

  async openAddSource(): Promise<void> {
    await this.addSourceButton.click()
  }

  async openAddSourceIfNeeded(): Promise<void> {
    if (await this.addSourceButton.isVisible()) {
      await this.openAddSource()
    }
  }

  async openLibraryBrowseIfNeeded(): Promise<void> {
    if (await this.libraryBrowseButton.isVisible()) {
      await this.libraryBrowseButton.click()
    }
  }

  async addMusicFolder(): Promise<void> {
    await this.addSource.addMusicFolder()
  }

  async selectBrowseProfile(label: LibraryBrowseProfileLabel): Promise<void> {
    await this.profileMenu.select(label)
  }
}

import { expect, type Locator } from '@playwright/test'

export class LibraryBrowseSurface {
  readonly root: Locator
  readonly emptyStatus: Locator

  constructor(libraryRoot: Locator) {
    this.root = libraryRoot.getByRole('tree')
    this.emptyStatus = libraryRoot.getByRole('status')
  }

  item(name: string | RegExp): Locator {
    return this.root.getByRole('treeitem', { name })
  }

  async expectVisible(): Promise<void> {
    await expect(this.root).toBeVisible()
  }

  async expectSourceVisible(sourceName: string | RegExp): Promise<void> {
    await this.expectVisible()
    await expect(this.item(sourceName)).toBeVisible()
  }

  async expectSourceHidden(sourceName: string | RegExp): Promise<void> {
    await expect(this.item(sourceName)).toHaveCount(0)
  }

  async expectNoRegisteredHierarchyPendingRows(): Promise<void> {
    await expect(
      this.root.getByRole('treeitem').filter({
        hasText:
          /Loading children|Loading literal hierarchy children|Contents pending|Folder child scopes pending|Probing folder child scopes/i
      })
    ).toHaveCount(0)
  }

  async select(name: string | RegExp): Promise<void> {
    await this.item(name).click()
  }

  async expand(name: string | RegExp): Promise<void> {
    const item = this.item(name)

    if ((await item.getAttribute('aria-expanded')) === 'true') {
      return
    }

    await item.getByRole('button', { name: 'Expand' }).click()
  }
}

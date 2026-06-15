import type { Locator } from '@playwright/test'

export class LibraryTree {
  readonly root: Locator
  readonly emptyStatus: Locator

  constructor(libraryRoot: Locator) {
    this.root = libraryRoot.getByRole('tree')
    this.emptyStatus = libraryRoot.getByRole('status')
  }

  item(name: string | RegExp): Locator {
    return this.root.getByRole('treeitem', { name })
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

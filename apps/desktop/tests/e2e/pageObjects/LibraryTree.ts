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
}

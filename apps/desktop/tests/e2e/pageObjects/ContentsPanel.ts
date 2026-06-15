import type { Locator } from '@playwright/test'

export class ContentsPanel {
  readonly title: Locator
  readonly table: Locator

  constructor(libraryRoot: Locator) {
    this.title = libraryRoot.getByRole('heading', { level: 3 })
    this.table = libraryRoot.getByRole('table')
  }

  row(name: string | RegExp): Locator {
    return this.table.getByRole('row', { name })
  }
}

import type { Locator } from '@playwright/test'

export class ContentsPanel {
  readonly root: Locator
  readonly title: Locator
  readonly table: Locator

  constructor(libraryRoot: Locator) {
    this.root = libraryRoot.getByRole('region', { name: /Library contents/i })
    this.title = this.root.getByRole('heading', { level: 3 })
    this.table = this.root.getByRole('table')
  }

  row(name: string | RegExp): Locator {
    return this.table.getByRole('row', { name })
  }

  rows(): Locator {
    return this.table.getByRole('row').filter({ hasNotText: 'Name' })
  }
}

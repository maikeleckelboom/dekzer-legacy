import { expect, type Locator } from '@playwright/test'

import { region } from '../locators/locatorPolicy'

export class ContentsSurface {
  readonly root: Locator
  readonly title: Locator
  readonly table: Locator

  constructor(libraryRoot: Locator) {
    this.root = region(libraryRoot, /Library contents/i)
    this.title = this.root.getByRole('heading', { level: 3 })
    this.table = this.root.getByRole('table')
  }

  row(name: string | RegExp): Locator {
    return this.table.getByRole('row', { name })
  }

  rows(): Locator {
    return this.table.getByRole('row').filter({ hasNotText: 'Name' })
  }

  async expectVisible(): Promise<void> {
    await expect(this.root).toBeVisible()
    await expect(this.title).toBeVisible()
    await expect(this.table).toBeVisible()
  }

  async expectTitle(title: string | RegExp): Promise<void> {
    await expect(this.title).toHaveText(title)
  }

  async expectRowsVisible(names: readonly (string | RegExp)[]): Promise<void> {
    for (const name of names) {
      await expect(this.row(name)).toBeVisible()
    }
  }

  async expectRowsHidden(names: readonly (string | RegExp)[]): Promise<void> {
    for (const name of names) {
      await expect(this.row(name)).toHaveCount(0)
    }
  }

  async expectAuthoritativeEmpty(): Promise<void> {
    await expect(
      this.row(
        /No (audio tracks in this view|playable media in this view|files in this source inventory view)\./
      )
    ).toBeVisible()
  }

  async expectSearchEmpty(scope: 'source' | 'folder'): Promise<void> {
    const label =
      scope === 'source'
        ? /No matching audio tracks in this source\./
        : /No matching audio tracks in this folder\./

    await this.expectTitle('Search results')
    await expect(this.row(label)).toBeVisible()
  }
}

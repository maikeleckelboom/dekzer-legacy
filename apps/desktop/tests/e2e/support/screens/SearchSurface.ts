import type { Locator } from '@playwright/test'

export class SearchSurface {
  readonly openButton: Locator
  readonly input: Locator
  readonly clearButton: Locator

  constructor(private readonly libraryRoot: Locator) {
    this.openButton = this.libraryRoot.getByRole('button', { name: 'Search indexed library' })
    this.input = this.libraryRoot.getByLabel('Search indexed library')
    this.clearButton = this.libraryRoot.getByRole('button', { name: 'Clear search' })
  }

  async searchFor(query: string): Promise<void> {
    if (await this.openButton.isVisible()) {
      await this.openButton.click()
    }

    await this.input.fill(query)
  }

  async search(query: string): Promise<void> {
    await this.searchFor(query)
  }

  async clear(): Promise<void> {
    if (await this.clearButton.isVisible()) {
      await this.clearButton.click()
      return
    }

    await this.input.fill('')
  }
}

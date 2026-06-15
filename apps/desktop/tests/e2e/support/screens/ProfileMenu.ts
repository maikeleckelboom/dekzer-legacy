import type { Locator } from '@playwright/test'

import type { LibraryBrowseProfileLabel } from '../domain/LibraryContracts'

export class ProfileMenu {
  readonly button: Locator

  constructor(private readonly libraryRoot: Locator) {
    this.button = this.libraryRoot.getByRole('button', { name: 'Indexed contents view' })
  }

  async select(label: LibraryBrowseProfileLabel): Promise<void> {
    await this.button.click()
    await this.libraryRoot
      .getByRole('listbox', { name: 'Indexed contents view' })
      .getByRole('option', { name: label, exact: true })
      .click()
  }
}

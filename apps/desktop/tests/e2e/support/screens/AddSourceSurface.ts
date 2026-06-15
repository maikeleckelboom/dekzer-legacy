import { expect, type Locator } from '@playwright/test'

export class AddSourceSurface {
  readonly addMusicFolderButton: Locator

  constructor(private readonly libraryRoot: Locator) {
    this.addMusicFolderButton = this.libraryRoot
      .getByRole('button', { name: 'Add music folder' })
      .first()
  }

  async expectVisible(): Promise<void> {
    await expect(this.addMusicFolderButton).toBeVisible()
  }

  async addMusicFolder(): Promise<void> {
    await this.addMusicFolderButton.click()
  }
}

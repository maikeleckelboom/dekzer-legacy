import type { Locator, Page } from '@playwright/test'
import { expect } from '@playwright/test'

export class SourceStatusPanel {
  private readonly page: Page
  readonly root: Locator

  constructor(libraryRoot: Locator, page: Page) {
    this.page = page
    this.root = libraryRoot.getByLabel('Source status')
  }

  action(name: string | RegExp): Locator {
    return this.root.getByRole('button', { name })
  }

  async runScanIfAvailable(): Promise<void> {
    const scan = this.action(/^(Scan source|Rescan source)$/)

    if (
      (await scan.count()) === 0 ||
      !(await scan.first().isVisible()) ||
      !(await scan.first().isEnabled())
    ) {
      return
    }

    await scan.first().click()
  }

  async removeSourceWithConfirmation(): Promise<void> {
    const confirmationHandled = new Promise<void>((resolve, reject) => {
      this.page.once('dialog', async (dialog) => {
        try {
          expect(dialog.type()).toBe('confirm')
          expect(dialog.message()).toContain('Remove this source from Dekzer?')
          await dialog.accept()
          resolve()
        } catch (error) {
          reject(error)
        }
      })
    })

    await this.action('Remove source').click()
    await confirmationHandled
  }
}

import { expect, type Locator, type Page } from '@playwright/test'

import { group } from '../locators/locatorPolicy'
import type { ContentsSurface } from './ContentsSurface'

export class SourceStatusSurface {
  readonly root: Locator

  constructor(
    libraryRoot: Locator,
    private readonly page: Page
  ) {
    this.root = group(libraryRoot, 'Source status')
  }

  action(name: string | RegExp): Locator {
    return this.root.getByRole('button', { name })
  }

  async expectVisible(): Promise<void> {
    await expect(this.root).toBeVisible()
  }

  async expectReadyHasVisibleEvidence(contents: ContentsSurface): Promise<void> {
    const readyVisible = await this.root.getByText('Ready', { exact: true }).isVisible()

    if (!readyVisible) {
      await expect(this.root).not.toContainText('Ready')
      return
    }

    await expect(contents.rows().first()).toBeVisible()
    await expect(contents.row(/Loading contents|Still indexing|Contents pending/)).toHaveCount(0)
  }

  async expectNoActiveScanCopy(): Promise<void> {
    await expect(this.root).not.toContainText(/Scanning source|Indexing|Still indexing|Needs scan/)
  }

  async expectBadge(name: string | RegExp): Promise<void> {
    await expect(this.root.getByText(name, { exact: typeof name === 'string' })).toBeVisible()
  }

  async expectNoReadyBadge(): Promise<void> {
    await expect(this.root.getByText('Ready', { exact: true })).toHaveCount(0)
  }

  async expectMissingOrUnavailable(): Promise<void> {
    await expect(this.root.getByText(/Missing|Offline\/unavailable|Blocked|Partial/)).toBeVisible()
    await this.expectNoReadyBadge()
  }

  async expectActionEnabled(name: string | RegExp): Promise<void> {
    await expect(this.action(name).first()).toBeVisible()
    await expect(this.action(name).first()).toBeEnabled()
  }

  async expectActionHiddenOrDisabled(name: string | RegExp): Promise<void> {
    const action = this.action(name)

    if ((await action.count()) === 0 || !(await action.first().isVisible())) {
      return
    }

    await expect(action.first()).toBeDisabled()
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

  async refreshStatus(): Promise<void> {
    await this.expectActionEnabled('Refresh status')
    await this.action('Refresh status').click()
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

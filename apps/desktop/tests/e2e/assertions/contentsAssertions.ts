import { expect } from '../fixtures/libraryFixture'
import type { ContentsPanel } from '../pageObjects/ContentsPanel'

export async function expectContentsPanelVisible(contentsPanel: ContentsPanel): Promise<void> {
  await expect(contentsPanel.title).toBeVisible()
  await expect(contentsPanel.table).toBeVisible()
}

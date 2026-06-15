import { expect } from '../fixtures/libraryFixture'
import type { SourceStatusPanel } from '../pageObjects/SourceStatusPanel'

export async function expectSourceStatusVisible(
  sourceStatusPanel: SourceStatusPanel
): Promise<void> {
  await expect(sourceStatusPanel.root).toBeVisible()
}

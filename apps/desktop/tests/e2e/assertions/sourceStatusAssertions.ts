import { expect } from '../fixtures/libraryFixture'
import type { ContentsPanel } from '../pageObjects/ContentsPanel'
import type { SourceStatusPanel } from '../pageObjects/SourceStatusPanel'

export async function expectSourceStatusVisible(
  sourceStatusPanel: SourceStatusPanel
): Promise<void> {
  await expect(sourceStatusPanel.root).toBeVisible()
}

export async function expectReadyStatusHasVisibleEvidence(
  sourceStatusPanel: SourceStatusPanel,
  contentsPanel: ContentsPanel
): Promise<void> {
  const readyVisible = await sourceStatusPanel.root.getByText('Ready', { exact: true }).isVisible()

  if (!readyVisible) {
    await expect(sourceStatusPanel.root).not.toContainText('Ready')
    return
  }

  await expect(contentsPanel.rows().first()).toBeVisible()
  await expect(contentsPanel.row(/Loading contents|Still indexing|Contents pending/)).toHaveCount(0)
}

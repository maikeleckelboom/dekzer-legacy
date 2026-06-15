import { expect } from '../fixtures/libraryFixture'
import type { ContentsPanel } from '../pageObjects/ContentsPanel'

export async function expectContentsPanelVisible(contentsPanel: ContentsPanel): Promise<void> {
  await expect(contentsPanel.title).toBeVisible()
  await expect(contentsPanel.table).toBeVisible()
}

export async function expectContentRowsVisible(
  contentsPanel: ContentsPanel,
  names: readonly (string | RegExp)[]
): Promise<void> {
  for (const name of names) {
    await expect(contentsPanel.row(name)).toBeVisible()
  }
}

export async function expectContentRowsHidden(
  contentsPanel: ContentsPanel,
  names: readonly (string | RegExp)[]
): Promise<void> {
  for (const name of names) {
    await expect(contentsPanel.row(name)).toHaveCount(0)
  }
}

export async function expectAuthoritativeEmptyContents(
  contentsPanel: ContentsPanel
): Promise<void> {
  await expect(
    contentsPanel.row(
      /No (audio tracks in this view|playable media in this view|files in this source inventory view)\./
    )
  ).toBeVisible()
}

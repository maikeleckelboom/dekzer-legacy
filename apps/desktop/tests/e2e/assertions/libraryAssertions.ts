import { expect } from '../fixtures/libraryFixture'
import type { LibraryPanel } from '../pageObjects/LibraryPanel'

export async function expectLibrarySurfaceVisible(libraryPanel: LibraryPanel): Promise<void> {
  await expect(libraryPanel.root).toBeVisible()
  await expect(libraryPanel.title).toHaveText(/^(Library Browse|Add Source)$/)
}

export async function expectAddSourceEntryReachable(libraryPanel: LibraryPanel): Promise<void> {
  if (await libraryPanel.addSourceButton.isVisible()) {
    await expect(libraryPanel.addSourceButton).toBeEnabled()
    return
  }

  await expectAddSourceSurfaceVisible(libraryPanel)
}

export async function expectAddSourceSurfaceVisible(libraryPanel: LibraryPanel): Promise<void> {
  await expect(libraryPanel.title).toHaveText('Add Source')
  await expect(
    libraryPanel.root.getByRole('button', { name: 'Add music folder' }).first()
  ).toBeVisible()
}

export async function expectLibraryBrowseSurfaceVisible(libraryPanel: LibraryPanel): Promise<void> {
  await expect(libraryPanel.title).toHaveText('Library Browse')
  await expect(libraryPanel.tree.root).toBeVisible()
}

export async function expectAdmittedSourceVisible(
  libraryPanel: LibraryPanel,
  sourceName: string | RegExp
): Promise<void> {
  await expectLibraryBrowseSurfaceVisible(libraryPanel)
  await expect(libraryPanel.tree.item(sourceName)).toBeVisible()
}

export async function expectAdmittedSourceHidden(
  libraryPanel: LibraryPanel,
  sourceName: string | RegExp
): Promise<void> {
  await expect(libraryPanel.tree.item(sourceName)).toHaveCount(0)
}

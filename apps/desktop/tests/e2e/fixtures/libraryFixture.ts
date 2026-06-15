import { test as base, expect } from './dialogs.fixture'
import { LibraryV0 } from '../support/domain/LibraryV0'
import { ContentsSurface } from '../support/screens/ContentsSurface'
import { LibraryBrowseSurface } from '../support/screens/LibraryBrowseSurface'
import { LibraryPanel } from '../support/screens/LibraryPanel'
import { SourceStatusSurface } from '../support/screens/SourceStatusSurface'

export type LibraryFixtures = {
  readonly libraryV0: LibraryV0
  readonly libraryPanel: LibraryPanel
  readonly libraryTree: LibraryBrowseSurface
  readonly contentsPanel: ContentsSurface
  readonly sourceStatusPanel: SourceStatusSurface
}

export const test = base.extend<LibraryFixtures>({
  libraryV0: async ({ dialogs, electronApp, mainWindow }, use) => {
    await use(new LibraryV0({ page: mainWindow, dialogs, electronApp }))
  },
  libraryPanel: async ({ mainWindow }, use) => {
    await use(new LibraryPanel(mainWindow))
  },
  libraryTree: async ({ libraryPanel }, use) => {
    await use(libraryPanel.tree)
  },
  contentsPanel: async ({ libraryPanel }, use) => {
    await use(libraryPanel.contents)
  },
  sourceStatusPanel: async ({ libraryPanel }, use) => {
    await use(libraryPanel.sourceStatus)
  }
})

export { expect }

import { test as base, expect } from './dialogs.fixture'
import { ContentsPanel } from '../pageObjects/ContentsPanel'
import { LibraryPanel } from '../pageObjects/LibraryPanel'
import { LibraryTree } from '../pageObjects/LibraryTree'
import { SourceStatusPanel } from '../pageObjects/SourceStatusPanel'

export type LibraryFixtures = {
  readonly libraryPanel: LibraryPanel
  readonly libraryTree: LibraryTree
  readonly contentsPanel: ContentsPanel
  readonly sourceStatusPanel: SourceStatusPanel
}

export const test = base.extend<LibraryFixtures>({
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

import type { ElectronApplication } from '@playwright/test'

import { test as base, expect, type ElectronAppHarness } from './electronApp.fixture'

export type DialogHelpers = {
  readonly selectDirectory: (directoryPath: string) => Promise<void>
  readonly cancelDirectorySelection: () => Promise<void>
  readonly restoreDirectorySelection: () => Promise<void>
}

export type DialogFixtures = {
  readonly dialogs: DialogHelpers
}

export const test = base.extend<DialogFixtures>({
  dialogs: async ({ electronApp }, use) => {
    const helpers: DialogHelpers = {
      selectDirectory: async (directoryPath) => {
        await electronApp.app().evaluate(
          ({ dialog }, args) => {
            const globalState = globalThis as typeof globalThis & {
              __dekzerE2EOriginalShowOpenDialog?: typeof dialog.showOpenDialog
            }

            globalState.__dekzerE2EOriginalShowOpenDialog ??= dialog.showOpenDialog.bind(dialog)
            dialog.showOpenDialog = (async () => ({
              canceled: false,
              filePaths: [args.directoryPath],
              bookmarks: []
            })) as typeof dialog.showOpenDialog
          },
          { directoryPath }
        )
      },
      cancelDirectorySelection: async () => {
        await electronApp.app().evaluate(({ dialog }) => {
          const globalState = globalThis as typeof globalThis & {
            __dekzerE2EOriginalShowOpenDialog?: typeof dialog.showOpenDialog
          }

          globalState.__dekzerE2EOriginalShowOpenDialog ??= dialog.showOpenDialog.bind(dialog)
          dialog.showOpenDialog = (async () => ({
            canceled: true,
            filePaths: [],
            bookmarks: []
          })) as typeof dialog.showOpenDialog
        })
      },
      restoreDirectorySelection: async () => {
        const app = safeRunningElectronApp(electronApp)

        if (app === undefined) {
          return
        }

        await app.evaluate(({ dialog }) => {
          const globalState = globalThis as typeof globalThis & {
            __dekzerE2EOriginalShowOpenDialog?: typeof dialog.showOpenDialog
          }
          const original = globalState.__dekzerE2EOriginalShowOpenDialog

          if (original !== undefined) {
            dialog.showOpenDialog = original
            delete globalState.__dekzerE2EOriginalShowOpenDialog
          }
        })
      }
    }

    await use(helpers)
    await helpers.restoreDirectorySelection()
  }
})

export { expect }

function safeRunningElectronApp(electronApp: ElectronAppHarness): ElectronApplication | undefined {
  try {
    return electronApp.app()
  } catch (error) {
    if (error instanceof Error && error.message === 'Electron app is not running.') {
      return undefined
    }

    throw error
  }
}

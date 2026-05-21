import type { BrowserWindow, OpenDialogOptions, OpenDialogReturnValue } from 'electron'

import type { LibraryBoundaryHost } from '../libraryBoundary/host'
import { rootChannels } from '../../shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../../shared/libraryRoots/chooseAndRegisterLocal'
import type {
  LocalRootRegistrationRequest,
  LocalRootRegistrationResult
} from '../../shared/libraryRoots/registerLocalRoot'
import { registerLocalRoot } from './registerLocalRoot'

export type LocalRootChoiceIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, ...args: readonly unknown[]) => Promise<LocalRootChoiceResult>
  ): void
}

export type LocalRootChoiceDialog = {
  showOpenDialog(options: OpenDialogOptions): Promise<OpenDialogReturnValue>
  showOpenDialog(
    browserWindow: BrowserWindow,
    options: OpenDialogOptions
  ): Promise<OpenDialogReturnValue>
}

export type LocalRootChoiceDependencies = {
  readonly dialog: LocalRootChoiceDialog
  readonly getParentWindow?: () => BrowserWindow | undefined
  readonly registerLocalRoot?: (
    host: LibraryBoundaryHost,
    request: LocalRootRegistrationRequest
  ) => Promise<LocalRootRegistrationResult>
}

const directoryPickerOptions = {
  title: 'Choose music folder',
  buttonLabel: 'Add Music Folder',
  properties: ['openDirectory']
} satisfies OpenDialogOptions

export function registerLocalRootChoiceIpc(
  ipcMain: LocalRootChoiceIpcMain,
  host: LibraryBoundaryHost,
  dependencies: LocalRootChoiceDependencies
): void {
  ipcMain.handle(rootChannels.chooseAndRegisterLocal, () =>
    chooseAndRegisterLocalRoot(host, dependencies)
  )
}

export async function chooseAndRegisterLocalRoot(
  host: LibraryBoundaryHost,
  dependencies: LocalRootChoiceDependencies
): Promise<LocalRootChoiceResult> {
  let pickerResult: OpenDialogReturnValue

  try {
    pickerResult = await showDirectoryPicker(dependencies)
  } catch {
    return createDialogFailedResult()
  }

  const selectedPath = selectedDirectoryPath(pickerResult)

  if (selectedPath === undefined) {
    return {
      state: 'canceled'
    }
  }

  try {
    return mapRegistrationResult(
      await (dependencies.registerLocalRoot ?? registerLocalRoot)(host, {
        absolutePath: selectedPath
      })
    )
  } catch {
    return {
      state: 'registrationFailed',
      error: {
        code: 'registrationFailed',
        message: 'Unable to register local library root.'
      }
    }
  }
}

async function showDirectoryPicker(
  dependencies: LocalRootChoiceDependencies
): Promise<OpenDialogReturnValue> {
  const parentWindow = dependencies.getParentWindow?.()

  if (parentWindow === undefined) {
    return dependencies.dialog.showOpenDialog(directoryPickerOptions)
  }

  return dependencies.dialog.showOpenDialog(parentWindow, directoryPickerOptions)
}

function selectedDirectoryPath(pickerResult: OpenDialogReturnValue): string | undefined {
  if (pickerResult.canceled) {
    return undefined
  }

  const selectedPath = pickerResult.filePaths[0]

  if (typeof selectedPath !== 'string' || selectedPath.trim().length === 0) {
    return undefined
  }

  return selectedPath
}

function mapRegistrationResult(result: LocalRootRegistrationResult): LocalRootChoiceResult {
  if (result.state === 'registered') {
    return {
      state: 'registered',
      root: result.root
    }
  }

  if (result.state === 'hostUnavailable') {
    return {
      state: 'hostUnavailable',
      error: result.error
    }
  }

  return {
    state: 'registrationFailed',
    error: result.error
  }
}

function createDialogFailedResult(): LocalRootChoiceResult {
  return {
    state: 'dialogFailed',
    error: {
      code: 'dialogFailed',
      message: 'Unable to open the music folder picker.'
    }
  }
}

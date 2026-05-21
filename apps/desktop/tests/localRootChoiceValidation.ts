import { strict as assert } from 'node:assert'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../src/main/libraryBoundary/host'
import {
  chooseAndRegisterLocalRoot,
  registerLocalRootChoiceIpc,
  type LocalRootChoiceDialog
} from '../src/main/libraryRoots/chooseAndRegisterLocal'
import { rootChannels } from '../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootRegistrationResult } from '../src/shared/libraryRoots/registerLocalRoot'
import { silentLogger, testApp } from './support/libraryBoundary'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-root-choice-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  const config = resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })

  await validatesCanceledPickerReturnsCanceled()
  await validatesNoSelectedPathReturnsCanceled()
  await validatesSelectedPathRegistersInsideMain()
  await validatesRegistrationFailureMapsSafely()
  await validatesHostUnavailableMapsThroughRegistrationBridge(config)
  await validatesDialogFailureMapsSafely()
  await validatesChoiceIpcRegistrationIgnoresRendererPath()
}

async function validatesCanceledPickerReturnsCanceled(): Promise<void> {
  const result = await chooseAndRegisterLocalRoot(unusedHost(), {
    dialog: fakeDialog({ canceled: true, filePaths: ['C:/Music'] }),
    registerLocalRoot: async () => assert.fail('registration should not run after cancellation')
  })

  assert.deepEqual(result, {
    state: 'canceled'
  } satisfies LocalRootChoiceResult)
}

async function validatesNoSelectedPathReturnsCanceled(): Promise<void> {
  const result = await chooseAndRegisterLocalRoot(unusedHost(), {
    dialog: fakeDialog({ canceled: false, filePaths: [] }),
    registerLocalRoot: async () => assert.fail('registration should not run without a path')
  })

  assert.deepEqual(result, {
    state: 'canceled'
  } satisfies LocalRootChoiceResult)
}

async function validatesSelectedPathRegistersInsideMain(): Promise<void> {
  let receivedAbsolutePath = ''
  let capturedPickerArgs: readonly unknown[] = []
  const result = await chooseAndRegisterLocalRoot(unusedHost(), {
    dialog: fakeDialog({ canceled: false, filePaths: ['C:/Music'] }, (args) => {
      capturedPickerArgs = args
    }),
    registerLocalRoot: async (_host, request) => {
      receivedAbsolutePath = request.absolutePath
      return registeredRoot()
    }
  })

  assert.equal(receivedAbsolutePath, 'C:/Music')
  assert.deepEqual(result, registeredChoice())
  assert.equal(capturedPickerArgs.length, 1)
  assert.deepEqual(readPickerOptions(capturedPickerArgs).properties, ['openDirectory'])
  assert.equal(readPickerOptions(capturedPickerArgs).title, 'Choose music folder')
  assert.equal(readPickerOptions(capturedPickerArgs).buttonLabel, 'Add Music Folder')
}

async function validatesRegistrationFailureMapsSafely(): Promise<void> {
  const result = await chooseAndRegisterLocalRoot(unusedHost(), {
    dialog: fakeDialog({ canceled: false, filePaths: ['C:/Missing'] }),
    registerLocalRoot: async () => ({
      state: 'registrationFailed',
      error: {
        code: 'registrationFailed',
        message: 'Unable to register local library root.'
      }
    })
  })

  assert.deepEqual(result, {
    state: 'registrationFailed',
    error: {
      code: 'registrationFailed',
      message: 'Unable to register local library root.'
    }
  } satisfies LocalRootChoiceResult)
}

async function validatesHostUnavailableMapsThroughRegistrationBridge(
  config: LibraryBoundaryHostConfig
): Promise<void> {
  const idleHost = new LibraryBoundaryHost(config, silentLogger())
  const result = await chooseAndRegisterLocalRoot(idleHost, {
    dialog: fakeDialog({ canceled: false, filePaths: ['C:/Music'] })
  })

  assert.equal(result.state, 'hostUnavailable')

  if (result.state !== 'hostUnavailable') {
    assert.fail('expected hostUnavailable local root choice result')
  }

  assert.equal(result.error.code, 'hostNotStarted')
  assert.equal(result.error.message, 'The library boundary host has not started yet.')
}

async function validatesDialogFailureMapsSafely(): Promise<void> {
  const result = await chooseAndRegisterLocalRoot(unusedHost(), {
    dialog: failingDialog(),
    registerLocalRoot: async () => assert.fail('registration should not run after dialog failure')
  })

  assert.deepEqual(result, {
    state: 'dialogFailed',
    error: {
      code: 'dialogFailed',
      message: 'Unable to open the music folder picker.'
    }
  } satisfies LocalRootChoiceResult)
}

async function validatesChoiceIpcRegistrationIgnoresRendererPath(): Promise<void> {
  let receivedAbsolutePath = ''
  const registration: {
    channel?: string
    handler?: (...args: readonly unknown[]) => Promise<LocalRootChoiceResult>
  } = {}

  registerLocalRootChoiceIpc(
    {
      handle(channel, listener): void {
        registration.channel = channel
        registration.handler = (...args) => listener({}, ...args)
      }
    },
    unusedHost(),
    {
      dialog: fakeDialog({ canceled: false, filePaths: ['C:/MainSelectedMusic'] }),
      registerLocalRoot: async (_host, request) => {
        receivedAbsolutePath = request.absolutePath
        return registeredRoot({
          rootId: 'main-selected-root',
          canonicalPath: request.absolutePath
        })
      }
    }
  )

  assert.equal(registration.channel, rootChannels.chooseAndRegisterLocal)

  const handler = registration.handler
  if (handler === undefined) {
    assert.fail('expected local root choice handler to be registered')
  }

  const result = await handler({ absolutePath: 'C:/RendererProvidedPath' })

  assert.equal(receivedAbsolutePath, 'C:/MainSelectedMusic')
  assert.deepEqual(result, {
    state: 'registered',
    root: {
      rootId: 'main-selected-root',
      canonicalPath: 'C:/MainSelectedMusic'
    }
  } satisfies LocalRootChoiceResult)
}

function fakeDialog(
  result: {
    readonly canceled: boolean
    readonly filePaths: readonly string[]
  },
  captureArgs: (args: readonly unknown[]) => void = () => undefined
): LocalRootChoiceDialog {
  return {
    async showOpenDialog(firstArgument: unknown, secondArgument?: unknown) {
      captureArgs(secondArgument === undefined ? [firstArgument] : [firstArgument, secondArgument])
      return {
        canceled: result.canceled,
        filePaths: [...result.filePaths]
      }
    }
  }
}

function failingDialog(): LocalRootChoiceDialog {
  return {
    async showOpenDialog() {
      throw new Error('fixture dialog failure')
    }
  }
}

function registeredRoot(
  root: {
    readonly rootId: string
    readonly canonicalPath: string
  } = {
    rootId: '7',
    canonicalPath: 'C:/Music'
  }
): LocalRootRegistrationResult {
  return {
    state: 'registered',
    root
  }
}

function registeredChoice(): LocalRootChoiceResult {
  return {
    state: 'registered',
    root: {
      rootId: '7',
      canonicalPath: 'C:/Music'
    }
  }
}

function readPickerOptions(args: readonly unknown[]): {
  readonly title?: string
  readonly buttonLabel?: string
  readonly properties?: readonly string[]
} {
  const options = args[args.length - 1]

  if (!isRecord(options)) {
    assert.fail('expected folder picker options')
  }

  return options as {
    readonly title?: string
    readonly buttonLabel?: string
    readonly properties?: readonly string[]
  }
}

function unusedHost(): LibraryBoundaryHost {
  return {} as LibraryBoundaryHost
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}

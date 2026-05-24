import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'vitest'

import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../../../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../../../src/main/libraryBoundary/host'
import {
  chooseAndRegisterLocalRoot,
  registerLocalRootChoiceIpc,
  type LocalRootChoiceDialog
} from '../../../src/main/libraryRoots/chooseAndRegisterLocal'
import { registerLocalRoot } from '../../../src/main/libraryRoots/registerLocalRoot'
import { rootChannels } from '../../../src/shared/libraryRoots/channels'
import type { LocalRootChoiceResult } from '../../../src/shared/libraryRoots/chooseAndRegisterLocal'
import type { LocalRootRegistrationResult } from '../../../src/shared/libraryRoots/registerLocalRoot'
import {
  createFakeClient,
  silentLogger,
  startedHostWithClient,
  testApp
} from '../../support/libraryBoundary'

const tempRoots: string[] = []

afterEach(() => {
  for (const tempRoot of tempRoots.splice(0)) {
    rmSync(tempRoot, { recursive: true, force: true })
  }
})

describe('local root registration boundaries', () => {
  it('registers the path selected by the main-process folder picker', async () => {
    let receivedAbsolutePath = ''
    let pickerArguments: readonly unknown[] = []

    const result = await chooseAndRegisterLocalRoot(unusedHost(), {
      dialog: fakeDialog({ canceled: false, filePaths: ['C:/Music'] }, (args) => {
        pickerArguments = args
      }),
      registerLocalRoot: async (_host, request) => {
        receivedAbsolutePath = request.absolutePath
        return registeredRoot()
      }
    })

    expect(receivedAbsolutePath).toBe('C:/Music')
    expect(result).toEqual(registeredChoice())
    expect(readPickerProperties(pickerArguments)).toContain('openDirectory')
  })

  it('maps picker cancellation, dialog failure, and registration failure safely', async () => {
    await expect(
      chooseAndRegisterLocalRoot(unusedHost(), {
        dialog: fakeDialog({ canceled: true, filePaths: ['C:/Music'] }),
        registerLocalRoot: async () => {
          throw new Error('registration should not run after cancellation')
        }
      })
    ).resolves.toEqual({ state: 'canceled' })

    await expect(
      chooseAndRegisterLocalRoot(unusedHost(), {
        dialog: failingDialog(),
        registerLocalRoot: async () => {
          throw new Error('registration should not run after dialog failure')
        }
      })
    ).resolves.toEqual({
      state: 'dialogFailed',
      error: {
        code: 'dialogFailed',
        message: 'Unable to open the music folder picker.'
      }
    } satisfies LocalRootChoiceResult)

    await expect(
      chooseAndRegisterLocalRoot(unusedHost(), {
        dialog: fakeDialog({ canceled: false, filePaths: ['C:/Missing'] }),
        registerLocalRoot: async () => ({
          state: 'registrationFailed',
          error: {
            code: 'registrationFailed',
            message: 'Unable to register local library root.'
          }
        })
      })
    ).resolves.toMatchObject({
      state: 'registrationFailed',
      error: { code: 'registrationFailed' }
    })
  })

  it('ignores renderer-provided paths at the choice IPC boundary', async () => {
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

    expect(registration.channel).toBe(rootChannels.chooseAndRegisterLocal)

    const result = await registration.handler?.({ absolutePath: 'C:/RendererProvidedPath' })

    expect(receivedAbsolutePath).toBe('C:/MainSelectedMusic')
    expect(result).toEqual({
      state: 'registered',
      root: {
        rootId: 'main-selected-root',
        canonicalPath: 'C:/MainSelectedMusic'
      }
    } satisfies LocalRootChoiceResult)
  })

  it('registers local roots through a started host and maps invalid or unavailable hosts', async () => {
    const config = hostConfig()
    const idleHost = new LibraryBoundaryHost(config, silentLogger())

    await expect(registerLocalRoot(idleHost, { absolutePath: 'C:/Music' })).resolves.toMatchObject({
      state: 'hostUnavailable',
      error: {
        code: 'hostNotStarted',
        message: 'The library boundary host has not started yet.'
      }
    })

    await expect(
      registerLocalRoot(await startedHostWithClient(config, createFakeClient()), {
        absolutePath: '   '
      })
    ).resolves.toMatchObject({
      state: 'invalidRequest',
      error: { code: 'invalidRequest' }
    })

    let receivedAbsolutePath = ''
    await expect(
      registerLocalRoot(
        await startedHostWithClient(
          config,
          createFakeClient({
            registerLocalRoot: async (request) => {
              receivedAbsolutePath = request.absolutePath
              return {
                rootId: '7',
                canonicalPath: 'C:/Music'
              }
            }
          })
        ),
        { absolutePath: 'C:/Music' }
      )
    ).resolves.toEqual(registeredRoot())
    expect(receivedAbsolutePath).toBe('C:/Music')

    await expect(
      registerLocalRoot(
        await startedHostWithClient(
          config,
          createFakeClient({
            registerLocalRoot: async () => {
              throw new Error('fixture registration failure')
            }
          })
        ),
        { absolutePath: 'C:/Missing' }
      )
    ).resolves.toMatchObject({
      state: 'registrationFailed',
      error: { code: 'registrationFailed' }
    })
  })
})

function hostConfig(): LibraryBoundaryHostConfig {
  const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-root-registration-'))
  tempRoots.push(tempRoot)
  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  return resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })
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

function readPickerProperties(args: readonly unknown[]): readonly string[] {
  const options = args[args.length - 1]

  if (!isRecord(options) || !Array.isArray(options.properties)) {
    throw new Error('Expected folder picker options.')
  }

  return options.properties.filter((property): property is string => typeof property === 'string')
}

function unusedHost(): LibraryBoundaryHost {
  return {} as LibraryBoundaryHost
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}

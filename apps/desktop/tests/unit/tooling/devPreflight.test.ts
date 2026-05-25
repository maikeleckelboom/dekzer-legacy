import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { isAbsolute } from 'node:path'

import {
  evaluateDevPreflight,
  formatAbortCommands,
  formatIncompatibleMessage,
  runDevPreflight,
  type DevPreflightDeps
} from '../../../tooling/dev.mjs'
import {
  parseDevArgs,
  type StorageSchemaState,
  type UserDataSource
} from '../../../tooling/storage.mjs'

describe('dev preflight evaluateDevPreflight', () => {
  it('starts Electron when schema is compatible', () => {
    const result = evaluateDevPreflight(
      'compatible',
      undefined,
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'developmentDefault',
      true
    )

    expect(result).toEqual({ kind: 'start' })
  })

  it('starts Electron when schema is missing', () => {
    const result = evaluateDevPreflight(
      'missing',
      undefined,
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'developmentDefault',
      true
    )

    expect(result).toEqual({
      kind: 'start',
      message: '[dev] development storage will be created on first startup'
    })
  })

  it('prompts on incompatible schema in TTY', () => {
    const result = evaluateDevPreflight(
      'incompatible',
      'table source_directories column count mismatch: canonical=17 live=16',
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'developmentDefault',
      true
    )

    expect(result.kind).toBe('promptForReset')
    if (result.kind !== 'promptForReset') return

    expect(result.lines).toContain('Development storage is incompatible with the current schema.')
    expect(result.lines).toContain(
      'Detail: table source_directories column count mismatch: canonical=17 live=16'
    )
    expect(result.lines).toContain('Storage root: /dev-user-data/default/development')
    expect(result.lines).toContain('Database: /dev-user-data/default/development/library.sqlite')
    expect(result.lines).toContain(
      'Reset development storage and start Dekzer? This deletes only the development storage root.'
    )
  })

  it('aborts on incompatible schema in non-TTY without prompting', () => {
    const result = evaluateDevPreflight(
      'incompatible',
      'schema does not match',
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'developmentDefault',
      false
    )

    expect(result.kind).toBe('abort')
    if (result.kind !== 'abort') return

    expect(result.lines).toContain('Development storage is incompatible with the current schema.')
    expect(result.lines).toContain('  pnpm --filter @dekzer/desktop run storage:doctor')
    expect(result.lines).toContain(
      '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete'
    )
    expect(result.lines).toContain('  pnpm --filter @dekzer/desktop run dev:fresh')
  })

  it('treats unreadable schema like incompatible', () => {
    const result = evaluateDevPreflight(
      'unreadable',
      'file is not a database',
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'developmentDefault',
      true
    )

    expect(result.kind).toBe('promptForReset')
  })

  it('includes env var note on storage root when user data source is environmentOverride', () => {
    const lines = formatIncompatibleMessage(
      '/custom/path/development',
      '/custom/path/development/library.sqlite',
      undefined,
      'environmentOverride'
    )

    expect(lines).toContain('Storage root: /custom/path/development')
    expect(lines).toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
    expect(lines).toContain('Database: /custom/path/development/library.sqlite')

    const storageRootIndex = lines.indexOf('Storage root: /custom/path/development')
    const envNoteIndex = lines.indexOf('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
    const databaseIndex = lines.indexOf('Database: /custom/path/development/library.sqlite')
    expect(storageRootIndex).toBeLessThan(envNoteIndex)
    expect(envNoteIndex).toBeLessThan(databaseIndex)
  })

  it('omits env var note when user data source is developmentDefault', () => {
    const lines = formatIncompatibleMessage(
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      undefined,
      'developmentDefault'
    )

    expect(lines).not.toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  })

  it('shows both storage root and database in incompatible message', () => {
    const lines = formatIncompatibleMessage(
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      'schema version mismatch',
      'developmentDefault'
    )

    expect(lines).toContain('Storage root: /dev-user-data/default/development')
    expect(lines).toContain('Database: /dev-user-data/default/development/library.sqlite')
  })

  it('shows both storage root and database in unreadable schema message', () => {
    const result = evaluateDevPreflight(
      'unreadable',
      'file is not a database',
      '/custom/path/development',
      '/custom/path/development/library.sqlite',
      'environmentOverride',
      true
    )

    expect(result.kind).toBe('promptForReset')
    if (result.kind !== 'promptForReset') return

    expect(result.lines).toContain('Storage root: /custom/path/development')
    expect(result.lines).toContain('Database: /custom/path/development/library.sqlite')
    expect(result.lines).toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  })
})

describe('dev preflight runDevPreflight', () => {
  function createDeps(overrides: Partial<DevPreflightDeps> = {}): DevPreflightDeps {
    return {
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'compatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      resetStorage: async () => 0,
      spawnDev: async () => 0,
      isInteractive: () => true,
      promptYesNo: async () => false,
      println: () => undefined,
      exit: () => {
        throw new ExitSignal(1)
      },
      ...overrides
    }
  }

  class ExitSignal {
    readonly code: number
    constructor(code: number) {
      this.code = code
    }
  }

  it('starts Electron when schema is compatible and propagates success', async () => {
    let devStarted = false
    const deps = createDeps({
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(devStarted).toBe(true)
  })

  it('exits when compatible storage starts Electron and Electron exits nonzero', async () => {
    let exitCode: number | undefined
    const deps = createDeps({
      spawnDev: async () => 1,
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(exitCode).toBe(1)
  })

  it('starts Electron when schema is missing', async () => {
    let devStarted = false
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'missing' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(devStarted).toBe(true)
  })

  it('exits when missing storage starts Electron and Electron exits nonzero', async () => {
    let exitCode: number | undefined
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'missing' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      spawnDev: async () => 1,
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(exitCode).toBe(1)
  })

  it('aborts when storage check fails', async () => {
    let devStarted = false
    const printed: string[] = []
    let exitCode: number | undefined
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checkFailed' as const,
        detail: 'storage status returned no JSON output'
      }),
      spawnDev: async () => {
        devStarted = true
        return 0
      },
      promptYesNo: async () => {
        throw new Error('prompt should not be called')
      },
      resetStorage: async () => {
        throw new Error('reset should not be called')
      },
      println: (message: string) => {
        printed.push(message)
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(devStarted).toBe(false)
    expect(exitCode).toBe(1)
    expect(printed).toContain(
      'Could not verify development storage compatibility. Dekzer was not started.'
    )
    expect(printed).toContain('Detail: storage status returned no JSON output')
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run storage:doctor')
    expect(printed).toContain(
      '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete'
    )
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run dev:fresh')
  })

  it('does not prompt or reset when storage check fails', async () => {
    let promptCalled = false
    let resetCalled = false
    const deps = createDeps({
      checkStorage: async () => ({ kind: 'checkFailed' as const, detail: 'cargo failed' }),
      promptYesNo: async () => {
        promptCalled = true
        return true
      },
      resetStorage: async () => {
        resetCalled = true
        return 0
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(promptCalled).toBe(false)
    expect(resetCalled).toBe(false)
  })

  it('resets and starts on incompatible schema with confirmation', async () => {
    let devStarted = false
    let resetCalled = false
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: {
          state: 'incompatible' as StorageSchemaState,
          detail: 'column count mismatch'
        },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      promptYesNo: async () => true,
      resetStorage: async () => {
        resetCalled = true
        return 0
      },
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(resetCalled).toBe(true)
    expect(devStarted).toBe(true)
  })

  it('exits when confirmed reset succeeds and Electron exits nonzero', async () => {
    let exitCode: number | undefined
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      promptYesNo: async () => true,
      resetStorage: async () => 0,
      spawnDev: async () => 1,
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(exitCode).toBe(1)
  })

  it('exits without reset when user declines on incompatible schema', async () => {
    let resetCalled = false
    let devStarted = false
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      promptYesNo: async () => false,
      resetStorage: async () => {
        resetCalled = true
        return 0
      },
      spawnDev: async () => {
        devStarted = true
        return 0
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(resetCalled).toBe(false)
    expect(devStarted).toBe(false)
    expect(exitCode).toBe(1)
  })

  it('does not prompt in non-TTY', async () => {
    let promptCalled = false
    let resetCalled = false
    let devStarted = false
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      isInteractive: () => false,
      promptYesNo: async () => {
        promptCalled = true
        return false
      },
      resetStorage: async () => {
        resetCalled = true
        return 0
      },
      spawnDev: async () => {
        devStarted = true
        return 0
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(promptCalled).toBe(false)
    expect(resetCalled).toBe(false)
    expect(devStarted).toBe(false)
    expect(exitCode).toBe(1)
  })

  it('prints clear commands in non-TTY abort', async () => {
    const printed: string[] = []
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      isInteractive: () => false,
      println: (message: string) => {
        printed.push(message)
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run storage:doctor')
    expect(printed).toContain(
      '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete'
    )
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run dev:fresh')
    expect(exitCode).toBe(1)
  })

  it('prevents Electron start when reset fails', async () => {
    let devStarted = false
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      promptYesNo: async () => true,
      resetStorage: async () => 1,
      spawnDev: async () => {
        devStarted = true
        return 0
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(devStarted).toBe(false)
    expect(exitCode).toBe(1)
  })
})

describe('formatAbortCommands', () => {
  it('includes all three recovery commands', () => {
    const commands = formatAbortCommands()
    expect(commands).toContain('  pnpm --filter @dekzer/desktop run storage:doctor')
    expect(commands).toContain(
      '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete'
    )
    expect(commands).toContain('  pnpm --filter @dekzer/desktop run dev:fresh')
  })
})

describe('parseDevArgs', () => {
  const envVar = 'DESKTOP_LIBRARY_USER_DATA_PATH'
  let originalEnv: string | undefined

  beforeEach(() => {
    originalEnv = process.env[envVar]
    delete process.env[envVar]
  })

  afterEach(() => {
    if (originalEnv === undefined) {
      delete process.env[envVar]
    } else {
      process.env[envVar] = originalEnv
    }
  })

  it('resolves default user data when no env and no arguments', () => {
    const result = parseDevArgs([])

    expect(isAbsolute(result.userDataPath)).toBe(true)
    expect(result.userDataSource).toBe('developmentDefault')
  })

  it('uses DESKTOP_LIBRARY_USER_DATA_PATH when set', () => {
    process.env[envVar] = '/custom/env/path'

    const result = parseDevArgs([])

    expect(result.userDataPath).toBe('/custom/env/path')
    expect(result.userDataSource).toBe('environmentOverride')
  })

  it('accepts --user-data with space-separated absolute path', () => {
    const result = parseDevArgs(['--user-data', '/custom/args/path'])

    expect(result.userDataPath).toBe('/custom/args/path')
    expect(result.userDataSource).toBe('argument')
  })

  it('accepts --user-data with equals-separated absolute path', () => {
    const result = parseDevArgs(['--user-data=/custom/args/path'])

    expect(result.userDataPath).toBe('/custom/args/path')
    expect(result.userDataSource).toBe('argument')
  })

  it('prefers --user-data argument over environment variable', () => {
    process.env[envVar] = '/custom/env/path'

    const result = parseDevArgs(['--user-data', '/custom/args/path'])

    expect(result.userDataPath).toBe('/custom/args/path')
    expect(result.userDataSource).toBe('argument')
  })

  it('rejects duplicate --user-data', () => {
    expect(() => parseDevArgs(['--user-data', '/a', '--user-data', '/b'])).toThrow(
      'duplicate --user-data'
    )
  })

  it('rejects duplicate --user-data with mixed forms', () => {
    expect(() => parseDevArgs(['--user-data=/a', '--user-data', '/b'])).toThrow(
      'duplicate --user-data'
    )
  })

  it('rejects relative --user-data path', () => {
    expect(() => parseDevArgs(['--user-data', 'relative/path'])).toThrow(
      '--user-data requires an absolute path value'
    )
  })

  it('rejects relative --user-data path in equals form', () => {
    expect(() => parseDevArgs(['--user-data=relative/path'])).toThrow(
      '--user-data requires an absolute path value'
    )
  })

  it('rejects missing --user-data value at end of argv', () => {
    expect(() => parseDevArgs(['--user-data'])).toThrow(
      '--user-data requires an absolute path value'
    )
  })

  it('rejects missing --user-data value before another flag', () => {
    expect(() => parseDevArgs(['--user-data', '--other'])).toThrow(
      '--user-data requires an absolute path value'
    )
  })

  it('rejects empty --user-data= value', () => {
    expect(() => parseDevArgs(['--user-data='])).toThrow(
      '--user-data requires an absolute path value'
    )
  })

  it('rejects unknown argument', () => {
    expect(() => parseDevArgs(['--unknown'])).toThrow('unknown argument "--unknown"')
  })

  it('rejects unexpected positional argument', () => {
    expect(() => parseDevArgs(['foobar'])).toThrow('unexpected argument "foobar"')
  })

  it('accepts -- separator before --user-data', () => {
    const result = parseDevArgs(['--', '--user-data', '/custom/args/path'])

    expect(result.userDataPath).toBe('/custom/args/path')
    expect(result.userDataSource).toBe('argument')
  })
})

describe('formatIncompatibleMessage argument source', () => {
  it('includes argument source note when user data source is argument', () => {
    const lines = formatIncompatibleMessage(
      '/custom/args/development',
      '/custom/args/development/library.sqlite',
      undefined,
      'argument'
    )

    expect(lines).toContain('Storage root: /custom/args/development')
    expect(lines).toContain('  (resolved via --user-data)')
    expect(lines).toContain('Database: /custom/args/development/library.sqlite')

    const rootIndex = lines.indexOf('Storage root: /custom/args/development')
    const noteIndex = lines.indexOf('  (resolved via --user-data)')
    const dbIndex = lines.indexOf('Database: /custom/args/development/library.sqlite')
    expect(rootIndex).toBeLessThan(noteIndex)
    expect(noteIndex).toBeLessThan(dbIndex)
  })

  it('does not show argument note for developmentDefault source', () => {
    const lines = formatIncompatibleMessage(
      '/dev-user-data/default/development',
      '/dev-user-data/default/development/library.sqlite',
      undefined,
      'developmentDefault'
    )

    expect(lines).not.toContain('  (resolved via --user-data)')
  })

  it('does not show argument note for environmentOverride source', () => {
    const lines = formatIncompatibleMessage(
      '/custom/path/development',
      '/custom/path/development/library.sqlite',
      undefined,
      'environmentOverride'
    )

    expect(lines).toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
    expect(lines).not.toContain('  (resolved via --user-data)')
  })
})

describe('dev preflight with argument userDataSource', () => {
  function createDeps(overrides: Partial<DevPreflightDeps> = {}): DevPreflightDeps {
    return {
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'compatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
        durableStorePath: '/dev-user-data/default/development/library.sqlite',
        userDataSource: 'developmentDefault' as const
      }),
      resetStorage: async () => 0,
      spawnDev: async () => 0,
      isInteractive: () => true,
      promptYesNo: async () => false,
      println: () => undefined,
      exit: () => {
        throw new ExitSignal(1)
      },
      ...overrides
    }
  }

  class ExitSignal {
    readonly code: number
    constructor(code: number) {
      this.code = code
    }
  }

  it('starts Electron when schema is compatible with argument source', async () => {
    let devStarted = false
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'compatible' as StorageSchemaState },
        storageRootPath: '/custom/args/development',
        durableStorePath: '/custom/args/development/library.sqlite',
        userDataSource: 'argument' as UserDataSource
      }),
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(devStarted).toBe(true)
  })

  it('resets and starts on incompatible schema with argument source', async () => {
    let devStarted = false
    let resetCalled = false
    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: {
          state: 'incompatible' as StorageSchemaState,
          detail: 'column mismatch'
        },
        storageRootPath: '/custom/args/development',
        durableStorePath: '/custom/args/development/library.sqlite',
        userDataSource: 'argument' as UserDataSource
      }),
      promptYesNo: async () => true,
      resetStorage: async () => {
        resetCalled = true
        return 0
      },
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(resetCalled).toBe(true)
    expect(devStarted).toBe(true)
  })

  it('aborts and prints argument source in non-TTY with incompatible schema', async () => {
    const printed: string[] = []
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/custom/args/development',
        durableStorePath: '/custom/args/development/library.sqlite',
        userDataSource: 'argument' as UserDataSource
      }),
      isInteractive: () => false,
      println: (message: string) => {
        printed.push(message)
      },
      exit: (code: number) => {
        exitCode = code
        throw new ExitSignal(code)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(exitCode).toBe(1)
    expect(printed).toContain('  (resolved via --user-data)')
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run storage:doctor')
    expect(printed).toContain(
      '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete'
    )
    expect(printed).toContain('  pnpm --filter @dekzer/desktop run dev:fresh')
  })

  it('prints argument source in prompt message when incompatible and interactive', async () => {
    const printed: string[] = []
    let promptCalled = false

    const deps = createDeps({
      checkStorage: async () => ({
        kind: 'checked' as const,
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/custom/args/development',
        durableStorePath: '/custom/args/development/library.sqlite',
        userDataSource: 'argument' as UserDataSource
      }),
      promptYesNo: async () => {
        promptCalled = true
        return false
      },
      println: (message: string) => {
        printed.push(message)
      }
    })

    await expect(runDevPreflight(deps)).rejects.toThrow(ExitSignal)
    expect(promptCalled).toBe(true)
    expect(printed).toContain('  (resolved via --user-data)')
  })
})

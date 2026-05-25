import { describe, expect, it } from 'vitest'

import {
  evaluateDevPreflight,
  formatAbortCommands,
  formatIncompatibleMessage,
  runDevPreflight,
  type DevPreflightDeps
} from '../../../tooling/dev.mjs'
import type { StorageSchemaState } from '../../../tooling/storage.mjs'

describe('dev preflight evaluateDevPreflight', () => {
  it('starts Electron when schema is compatible', () => {
    const result = evaluateDevPreflight(
      'compatible',
      undefined,
      '/dev-user-data/default/development',
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
      'developmentDefault',
      true
    )

    expect(result.kind).toBe('promptForReset')
    if (result.kind !== 'promptForReset') return

    expect(result.lines).toContain('Development storage is incompatible with the current schema.')
    expect(result.lines).toContain(
      'Detail: table source_directories column count mismatch: canonical=17 live=16'
    )
    expect(result.lines).toContain('Database: /dev-user-data/default/development')
    expect(result.lines).toContain(
      'Reset development storage and start Dekzer? This deletes only the development storage root.'
    )
  })

  it('aborts on incompatible schema in non-TTY without prompting', () => {
    const result = evaluateDevPreflight(
      'incompatible',
      'schema does not match',
      '/dev-user-data/default/development',
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
      'developmentDefault',
      true
    )

    expect(result.kind).toBe('promptForReset')
  })

  it('includes env var note when user data source is environmentOverride', () => {
    const lines = formatIncompatibleMessage(
      '/custom/path/development',
      undefined,
      'environmentOverride'
    )

    expect(lines).toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  })

  it('omits env var note when user data source is developmentDefault', () => {
    const lines = formatIncompatibleMessage(
      '/dev-user-data/default/development',
      undefined,
      'developmentDefault'
    )

    expect(lines).not.toContain('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  })
})

describe('dev preflight runDevPreflight', () => {
  function createDeps(overrides: Partial<DevPreflightDeps> = {}): DevPreflightDeps {
    return {
      checkStorage: async () => ({
        schema: { state: 'compatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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

  it('starts Electron when schema is compatible', async () => {
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

  it('starts Electron when schema is missing', async () => {
    let devStarted = false
    const deps = createDeps({
      checkStorage: async () => ({
        schema: { state: 'missing' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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

  it('starts Electron when storage check fails', async () => {
    let devStarted = false
    const deps = createDeps({
      checkStorage: async () => undefined,
      spawnDev: async () => {
        devStarted = true
        return 0
      }
    })

    await runDevPreflight(deps)
    expect(devStarted).toBe(true)
  })

  it('resets and starts on incompatible schema with confirmation', async () => {
    let devStarted = false
    let resetCalled = false
    const deps = createDeps({
      checkStorage: async () => ({
        schema: {
          state: 'incompatible' as StorageSchemaState,
          detail: 'column count mismatch'
        },
        storageRootPath: '/dev-user-data/default/development',
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

  it('exits without reset when user declines on incompatible schema', async () => {
    let resetCalled = false
    let devStarted = false
    let exitCode: number | undefined

    const deps = createDeps({
      checkStorage: async () => ({
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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
        schema: { state: 'incompatible' as StorageSchemaState },
        storageRootPath: '/dev-user-data/default/development',
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

import { spawn } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createInterface } from 'node:readline'

import {
  checkDevelopmentStorage,
  resolveDefaultUserDataPath,
  spawnCargoStorageCommand,
  type ParsedStorageArgs,
  type StorageSchemaState
} from './storage.mjs'

export type DevPreflightResult =
  | { kind: 'start'; message?: string }
  | { kind: 'promptForReset'; lines: string[] }
  | { kind: 'abort'; lines: string[] }

export function evaluateDevPreflight(
  schemaState: StorageSchemaState,
  schemaDetail: string | undefined,
  storageRootPath: string,
  userDataSource: 'environmentOverride' | 'developmentDefault',
  isInteractive: boolean
): DevPreflightResult {
  if (schemaState === 'missing') {
    return { kind: 'start', message: '[dev] development storage will be created on first startup' }
  }

  if (schemaState === 'compatible') {
    return { kind: 'start' }
  }

  const lines = formatIncompatibleMessage(storageRootPath, schemaDetail, userDataSource)

  if (!isInteractive) {
    lines.push('')
    lines.push('To fix this, run one of:')
    lines.push('  pnpm --filter @dekzer/desktop run storage:doctor')
    lines.push('  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete')
    lines.push('  pnpm --filter @dekzer/desktop run dev:fresh')
    return { kind: 'abort', lines }
  }

  lines.push(
    'Reset development storage and start Dekzer? This deletes only the development storage root.'
  )
  lines.push('')
  return { kind: 'promptForReset', lines }
}

export function formatIncompatibleMessage(
  storageRootPath: string,
  schemaDetail: string | undefined,
  userDataSource: 'environmentOverride' | 'developmentDefault'
): string[] {
  const lines: string[] = [
    '',
    'Development storage is incompatible with the current schema.',
    '',
    `Database: ${storageRootPath}`
  ]

  if (userDataSource === 'environmentOverride') {
    lines.push('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  }

  if (schemaDetail !== undefined) {
    lines.push(`Detail: ${schemaDetail}`)
  }

  lines.push('')
  lines.push('This usually happens after changing the canonical development schema.')
  return lines
}

export function formatAbortCommands(): string[] {
  return [
    '',
    'To fix this, run one of:',
    '  pnpm --filter @dekzer/desktop run storage:doctor',
    '  pnpm --filter @dekzer/desktop run storage:reset -- --confirm-delete',
    '  pnpm --filter @dekzer/desktop run dev:fresh'
  ]
}

export type DevPreflightDeps = {
  readonly checkStorage: () => Promise<
    | {
        readonly schema: { readonly state: StorageSchemaState; readonly detail?: string }
        readonly storageRootPath: string
        readonly userDataSource: 'environmentOverride' | 'developmentDefault'
      }
    | undefined
  >
  readonly resetStorage: () => Promise<number>
  readonly spawnDev: () => Promise<number>
  readonly isInteractive: () => boolean
  readonly promptYesNo: () => Promise<boolean>
  readonly println: (message: string) => void
  readonly exit: (code: number) => never
}

export async function runDevPreflight(deps: DevPreflightDeps): Promise<void> {
  const check = await deps.checkStorage()

  if (check === undefined) {
    deps.println('[dev] warning: could not check development storage status, starting anyway')
    await deps.spawnDev()
    return
  }

  const result = evaluateDevPreflight(
    check.schema.state,
    check.schema.detail,
    check.storageRootPath,
    check.userDataSource,
    deps.isInteractive()
  )

  if (result.kind === 'start') {
    if (result.message !== undefined) {
      deps.println(result.message)
    }
    await deps.spawnDev()
    return
  }

  for (const line of result.lines) {
    deps.println(line)
  }

  if (result.kind === 'abort') {
    deps.exit(1)
  }

  const confirmed = await deps.promptYesNo()
  if (!confirmed) {
    for (const line of formatAbortCommands()) {
      deps.println(line)
    }
    deps.exit(1)
  }

  const resetExitCode = await deps.resetStorage()
  if (resetExitCode !== 0) {
    deps.println('[dev] storage reset failed, not starting Electron')
    deps.exit(1)
  }

  await deps.spawnDev()
}

function resolveDesktopRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..')
}

async function promptYesNo(): Promise<boolean> {
  const rl = createInterface({ input: process.stdin, output: process.stdout })
  try {
    const answer = await new Promise<string>((resolveAnswer) => {
      rl.question('[y/N] ', resolveAnswer)
    })
    return answer.trim().toLowerCase() === 'y'
  } finally {
    rl.close()
  }
}

function spawnElectronViteDev(): Promise<number> {
  const command = process.platform === 'win32' ? 'electron-vite.cmd' : 'electron-vite'
  const desktopRoot = resolveDesktopRoot()

  return new Promise<number>((resolveExit) => {
    const child = spawn(command, ['dev', '--ignoreConfigWarning'], {
      cwd: desktopRoot,
      stdio: 'inherit'
    })

    child.on('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'ENOENT') {
        console.error(`[dev] error: ${command} not found`)
      } else {
        console.error(`[dev] error: failed to spawn ${command}: ${err.message}`)
      }
      resolveExit(1)
    })

    child.on('exit', (code) => {
      resolveExit(code ?? 1)
    })
  })
}

function createResetArgs(userDataPath: string, userDataSource: string): ParsedStorageArgs {
  return {
    subcommand: 'reset',
    userDataPath,
    userDataSource: userDataSource as 'environmentOverride' | 'developmentDefault',
    confirmDelete: true
  }
}

async function main(): Promise<void> {
  const { path: userDataPath, source: userDataSource } = resolveDefaultUserDataPath()

  await runDevPreflight({
    checkStorage: async () => {
      try {
        return await checkDevelopmentStorage(userDataPath, userDataSource)
      } catch {
        return undefined
      }
    },
    resetStorage: () => spawnCargoStorageCommand(createResetArgs(userDataPath, userDataSource)),
    spawnDev: spawnElectronViteDev,
    isInteractive: () => process.stdin.isTTY === true && process.stdout.isTTY === true,
    promptYesNo,
    println: (message: string) => console.log(message),
    exit: (code: number) => process.exit(code)
  })
}

const thisFile = resolve(fileURLToPath(import.meta.url))
const entryFile = process.argv[1] !== undefined ? resolve(process.argv[1]) : undefined

if (entryFile === thisFile) {
  main()
}

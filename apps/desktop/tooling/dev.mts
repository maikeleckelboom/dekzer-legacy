import { spawn } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createInterface } from 'node:readline'

import {
  checkDevelopmentStorage,
  desktopLibraryUserDataEnvironmentVariable,
  parseDevArgs,
  spawnCargoStorageCommand,
  type ParsedStorageArgs,
  type StorageSchemaState,
  type UserDataSource
} from './storage.mjs'

export type DevPreflightResult =
  | { kind: 'start'; message?: string }
  | { kind: 'promptForReset'; lines: string[] }
  | { kind: 'abort'; lines: string[] }

export type DevStorageCheckResult =
  | {
      readonly kind: 'checked'
      readonly schema: { readonly state: StorageSchemaState; readonly detail?: string }
      readonly storageRootPath: string
      readonly durableStorePath: string
      readonly userDataSource: UserDataSource
    }
  | {
      readonly kind: 'checkFailed'
      readonly detail?: string
    }

export function evaluateDevPreflight(
  schemaState: StorageSchemaState,
  schemaDetail: string | undefined,
  storageRootPath: string,
  durableStorePath: string,
  userDataSource: UserDataSource,
  isInteractive: boolean
): DevPreflightResult {
  if (schemaState === 'missing') {
    return { kind: 'start', message: '[dev] development storage will be created on first startup' }
  }

  if (schemaState === 'compatible') {
    return { kind: 'start' }
  }

  const lines = formatIncompatibleMessage(
    storageRootPath,
    durableStorePath,
    schemaDetail,
    userDataSource
  )

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
  durableStorePath: string,
  schemaDetail: string | undefined,
  userDataSource: UserDataSource
): string[] {
  const lines: string[] = [
    '',
    'Development storage is incompatible with the current schema.',
    '',
    `Storage root: ${storageRootPath}`
  ]

  if (userDataSource === 'environmentOverride') {
    lines.push('  (resolved via DESKTOP_LIBRARY_USER_DATA_PATH)')
  } else if (userDataSource === 'argument') {
    lines.push('  (resolved via --user-data)')
  }

  lines.push(`Database: ${durableStorePath}`)

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

export function formatCheckFailureMessage(detail: string | undefined): string[] {
  const lines = ['', 'Could not verify development storage compatibility. Dekzer was not started.']

  if (detail !== undefined) {
    lines.push(`Detail: ${detail}`)
  }

  lines.push(...formatAbortCommands())
  return lines
}

export type DevPreflightDeps = {
  readonly checkStorage: () => Promise<DevStorageCheckResult>
  readonly resetStorage: () => Promise<number>
  readonly spawnDev: () => Promise<number>
  readonly isInteractive: () => boolean
  readonly promptYesNo: () => Promise<boolean>
  readonly println: (message: string) => void
  readonly exit: (code: number) => never
}

export async function runDevPreflight(deps: DevPreflightDeps): Promise<void> {
  const check = await deps.checkStorage()

  if (check.kind === 'checkFailed') {
    for (const line of formatCheckFailureMessage(check.detail)) {
      deps.println(line)
    }
    deps.exit(1)
    return
  }

  const result = evaluateDevPreflight(
    check.schema.state,
    check.schema.detail,
    check.storageRootPath,
    check.durableStorePath,
    check.userDataSource,
    deps.isInteractive()
  )

  if (result.kind === 'start') {
    if (result.message !== undefined) {
      deps.println(result.message)
    }
    await startDevOrExit(deps)
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

  await startDevOrExit(deps)
}

async function startDevOrExit(deps: Pick<DevPreflightDeps, 'spawnDev' | 'exit'>): Promise<void> {
  const exitCode = await deps.spawnDev()
  if (exitCode !== 0) {
    deps.exit(exitCode)
  }
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

export function createElectronViteDevEnvironment(
  userDataPath: string,
  parentEnv: NodeJS.ProcessEnv = process.env
): NodeJS.ProcessEnv {
  return {
    ...parentEnv,
    [desktopLibraryUserDataEnvironmentVariable]: userDataPath
  }
}

export function spawnElectronViteDev(userDataPath: string): Promise<number> {
  const command = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
  const desktopRoot = resolveDesktopRoot()

  return new Promise<number>((resolveExit) => {
    const child = spawn(command, ['exec', 'electron-vite', 'dev', '--ignoreConfigWarning'], {
      cwd: desktopRoot,
      env: createElectronViteDevEnvironment(userDataPath),
      stdio: 'inherit'
    })

    child.on('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'ENOENT') {
        console.error(`[dev] error: ${command} not found while starting electron-vite`)
      } else {
        console.error(
          `[dev] error: failed to start electron-vite through ${command}: ${err.message}`
        )
      }
      resolveExit(1)
    })

    child.on('exit', (code) => {
      resolveExit(code ?? 1)
    })
  })
}

export function createResetArgs(
  userDataPath: string,
  userDataSource: UserDataSource
): ParsedStorageArgs {
  return {
    subcommand: 'reset',
    userDataPath,
    userDataSource,
    confirmDelete: true
  }
}

async function main(): Promise<void> {
  try {
    const { userDataPath, userDataSource } = parseDevArgs(process.argv.slice(2))

    console.log(`[dev] user data: ${userDataPath} (source: ${userDataSource})`)

    await runDevPreflight({
      checkStorage: async () => {
        try {
          const check = await checkDevelopmentStorage(userDataPath, userDataSource)
          return {
            kind: 'checked',
            schema: check.schema,
            storageRootPath: check.storageRootPath,
            durableStorePath: check.durableStorePath,
            userDataSource: check.userDataSource
          }
        } catch (error: unknown) {
          return checkFailed(errorDetail(error))
        }
      },
      resetStorage: () => spawnCargoStorageCommand(createResetArgs(userDataPath, userDataSource)),
      spawnDev: () => spawnElectronViteDev(userDataPath),
      isInteractive: () => process.stdin.isTTY === true && process.stdout.isTTY === true,
      promptYesNo,
      println: (message: string) => console.log(message),
      exit: (code: number) => process.exit(code)
    })
  } catch (error: unknown) {
    console.error(`[dev] ${error instanceof Error ? error.message : 'unknown error'}`)
    process.exit(1)
  }
}

const thisFile = resolve(fileURLToPath(import.meta.url))
const entryFile = process.argv[1] !== undefined ? resolve(process.argv[1]) : undefined

if (entryFile === thisFile) {
  main()
}

function errorDetail(error: unknown): string | undefined {
  if (error instanceof Error) {
    return error.message
  }

  if (typeof error === 'string') {
    return error
  }

  return undefined
}

function checkFailed(detail: string | undefined): DevStorageCheckResult {
  if (detail === undefined) {
    return { kind: 'checkFailed' }
  }

  return { kind: 'checkFailed', detail }
}

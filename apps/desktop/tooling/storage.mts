import { spawn } from 'node:child_process'
import { isAbsolute, resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

export type StorageSubcommand = 'status' | 'reset'

export type ParsedStorageArgs = {
  subcommand: StorageSubcommand
  userDataPath: string
  userDataSource: 'environmentOverride' | 'developmentDefault' | 'argument'
  confirmDelete: boolean
}

const VALID_SUBCOMMANDS: readonly string[] = ['status', 'reset']
const ENV_VAR = 'DESKTOP_LIBRARY_USER_DATA_PATH'

export function resolveWorkspaceRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
}

export function resolveDefaultUserDataPath(): {
  path: string
  source: 'environmentOverride' | 'developmentDefault'
} {
  const envPath = process.env[ENV_VAR]
  if (envPath !== undefined && envPath !== '') {
    return { path: envPath, source: 'environmentOverride' }
  }
  return {
    path: resolve(resolveWorkspaceRoot(), '.dev-user-data', 'default'),
    source: 'developmentDefault'
  }
}

export function parseArgs(argv: readonly string[]): ParsedStorageArgs {
  const subcommandRaw = argv[0]
  if (subcommandRaw === undefined || !VALID_SUBCOMMANDS.includes(subcommandRaw)) {
    throw new Error(`unknown subcommand "${subcommandRaw ?? ''}"; expected status or reset`)
  }
  const subcommand = subcommandRaw as StorageSubcommand

  let userDataPathFromArg: string | undefined
  let confirmDelete = false
  let seenUserData = false
  let seenConfirmDelete = false

  let i = 1
  while (i < argv.length) {
    const arg = argv[i]!

    if (arg === '--user-data') {
      if (seenUserData) {
        throw new Error('duplicate --user-data')
      }
      seenUserData = true

      i++
      const value = argv[i]

      if (value === undefined || value === '' || value.startsWith('--')) {
        throw new Error('--user-data requires an absolute path value')
      }
      if (!isAbsolute(value)) {
        throw new Error('--user-data requires an absolute path value')
      }

      userDataPathFromArg = value
      i++
    } else if (arg.startsWith('--user-data=')) {
      if (seenUserData) {
        throw new Error('duplicate --user-data')
      }
      seenUserData = true

      const value = arg.slice('--user-data='.length)

      if (value === '') {
        throw new Error('--user-data requires an absolute path value')
      }
      if (!isAbsolute(value)) {
        throw new Error('--user-data requires an absolute path value')
      }

      userDataPathFromArg = value
      i++
    } else if (arg === '--confirm-delete') {
      if (seenConfirmDelete) {
        throw new Error('duplicate --confirm-delete')
      }
      seenConfirmDelete = true
      confirmDelete = true
      i++
    } else if (arg === '--') {
      i++
    } else if (arg.startsWith('--')) {
      throw new Error(`unknown argument "${arg}"`)
    } else {
      throw new Error(`unexpected argument "${arg}"`)
    }
  }

  if (subcommand === 'reset' && !confirmDelete) {
    throw new Error('--confirm-delete is required for reset')
  }

  let userDataPath: string
  let userDataSource: 'argument' | 'environmentOverride' | 'developmentDefault'

  if (userDataPathFromArg !== undefined) {
    userDataPath = userDataPathFromArg
    userDataSource = 'argument'
  } else {
    const def = resolveDefaultUserDataPath()
    userDataPath = def.path
    userDataSource = def.source
  }

  return { subcommand, userDataPath, userDataSource, confirmDelete }
}

export async function spawnCargoStorageCommand(parsed: ParsedStorageArgs): Promise<number> {
  const cargoArgs = [
    'run',
    '-p',
    'library-boundary-stdio',
    '--',
    'storage',
    parsed.subcommand,
    '--user-data',
    parsed.userDataPath
  ]

  if (parsed.subcommand === 'reset' && parsed.confirmDelete) {
    cargoArgs.push('--confirm-delete')
  }

  const cargoCommand = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
  const workspaceRoot = resolveWorkspaceRoot()

  return new Promise<number>((resolveExitCode) => {
    const child = spawn(cargoCommand, cargoArgs, {
      cwd: workspaceRoot,
      stdio: 'inherit'
    })

    child.on('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'ENOENT') {
        console.error(`[storage] error: cargo not found at workspace root ${workspaceRoot}`)
      } else {
        console.error(`[storage] error: failed to spawn cargo: ${err.message}`)
      }
      resolveExitCode(1)
    })

    child.on('exit', (code) => {
      resolveExitCode(code ?? 1)
    })
  })
}

async function main(): Promise<void> {
  try {
    const parsed = parseArgs(process.argv.slice(2))

    console.log(`[storage] user data path: ${parsed.userDataPath}`)
    console.log(`[storage] source: ${parsed.userDataSource}`)

    const exitCode = await spawnCargoStorageCommand(parsed)
    process.exit(exitCode)
  } catch (e: unknown) {
    if (e instanceof Error) {
      console.error(`[storage] error: ${e.message}`)
    }
    process.exit(1)
  }
}

const thisFile = resolve(fileURLToPath(import.meta.url))
const entryFile = process.argv[1] !== undefined ? resolve(process.argv[1]) : undefined

if (entryFile === thisFile) {
  main()
}

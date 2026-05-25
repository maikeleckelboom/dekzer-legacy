import { spawn } from 'node:child_process'
import { isAbsolute, resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

export type StorageSubcommand = 'status' | 'reset' | 'doctor'
type CargoStorageSubcommand = Exclude<StorageSubcommand, 'doctor'>

export type UserDataSource = 'environmentOverride' | 'developmentDefault' | 'argument'

export type ParsedStorageArgs = {
  subcommand: StorageSubcommand
  userDataPath: string
  userDataSource: UserDataSource
  confirmDelete: boolean
}

type StorageEnvironmentStatus = {
  environment: string
  storageRootPath: string
  storageRootExists: boolean
  durableStorePath: string
  durableStoreExists: boolean
  artifactFileStorePath: string
  artifactFileStoreExists: boolean
  walPath: string
  walExists: boolean
  shmPath: string
  shmExists: boolean
  schema?: StorageSchemaStatus
}

export type StorageSchemaState = 'missing' | 'compatible' | 'incompatible' | 'unreadable'

export type StorageSchemaStatus = {
  state: StorageSchemaState
  detail?: string
}

type StorageStatusEnvelope = {
  type: 'storageStatus'
  userDataPath: string
  development: StorageEnvironmentStatus
  production: StorageEnvironmentStatus
}

export const libraryUserDataPathEnvVar = 'DESKTOP_LIBRARY_USER_DATA_PATH'

export function resolveWorkspaceRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..', '..', '..')
}

export function resolveDefaultUserDataPath(): {
  path: string
  source: 'environmentOverride' | 'developmentDefault'
} {
  const envPath = process.env[libraryUserDataPathEnvVar]
  if (envPath !== undefined && envPath !== '') {
    return { path: envPath, source: 'environmentOverride' }
  }
  return {
    path: resolve(resolveWorkspaceRoot(), '.dev-user-data', 'default'),
    source: 'developmentDefault'
  }
}

export function parseDevArgs(argv: readonly string[]): {
  userDataPath: string
  userDataSource: UserDataSource
} {
  let userDataPathFromArg: string | undefined
  let seenUserData = false

  let i = 0
  while (i < argv.length) {
    const arg = argv[i]!

    const userDataOption = parseUserDataOption(argv, i, seenUserData)
    if (userDataOption !== undefined) {
      seenUserData = true
      userDataPathFromArg = userDataOption.userDataPath
      i = userDataOption.nextIndex
    } else if (arg === '--') {
      i++
    } else if (arg.startsWith('--')) {
      throw new Error(`unknown argument "${arg}"`)
    } else {
      throw new Error(`unexpected argument "${arg}"`)
    }
  }

  if (userDataPathFromArg !== undefined) {
    return { userDataPath: userDataPathFromArg, userDataSource: 'argument' }
  }

  const def = resolveDefaultUserDataPath()
  return { userDataPath: def.path, userDataSource: def.source }
}

export function parseArgs(argv: readonly string[]): ParsedStorageArgs {
  const subcommandRaw = argv[0]
  if (!isStorageSubcommand(subcommandRaw)) {
    throw new Error(
      `unknown subcommand "${subcommandRaw ?? ''}"; expected status, reset, or doctor`
    )
  }
  const subcommand = subcommandRaw

  let userDataPathFromArg: string | undefined
  let confirmDelete = false
  let seenUserData = false
  let seenConfirmDelete = false

  let i = 1
  while (i < argv.length) {
    const arg = argv[i]!

    const userDataOption = parseUserDataOption(argv, i, seenUserData)
    if (userDataOption !== undefined) {
      seenUserData = true
      userDataPathFromArg = userDataOption.userDataPath
      i = userDataOption.nextIndex
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
  if (subcommand !== 'reset' && confirmDelete) {
    throw new Error('--confirm-delete is only valid for reset')
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

function isStorageSubcommand(value: string | undefined): value is StorageSubcommand {
  return value === 'status' || value === 'reset' || value === 'doctor'
}

function parseUserDataOption(
  argv: readonly string[],
  index: number,
  seenUserData: boolean
): { readonly userDataPath: string; readonly nextIndex: number } | undefined {
  const arg = argv[index]
  if (arg === undefined) {
    return undefined
  }

  if (arg === '--user-data') {
    if (seenUserData) {
      throw new Error('duplicate --user-data')
    }

    const value = argv[index + 1]
    validateUserDataPathArgument(value)

    return { userDataPath: value, nextIndex: index + 2 }
  }

  if (arg.startsWith('--user-data=')) {
    if (seenUserData) {
      throw new Error('duplicate --user-data')
    }

    const value = arg.slice('--user-data='.length)
    validateUserDataPathArgument(value)

    return { userDataPath: value, nextIndex: index + 1 }
  }

  return undefined
}

function validateUserDataPathArgument(value: string | undefined): asserts value is string {
  if (value === undefined || value === '' || value.startsWith('--') || !isAbsolute(value)) {
    throw new Error('--user-data requires an absolute path value')
  }
}

function cargoStorageArgs(parsed: ParsedStorageArgs, subcommand: CargoStorageSubcommand): string[] {
  const cargoArgs = [
    'run',
    '--quiet',
    '-p',
    'library-boundary-stdio',
    '--',
    'storage',
    subcommand,
    '--user-data',
    parsed.userDataPath
  ]

  if (subcommand === 'reset' && parsed.confirmDelete) {
    cargoArgs.push('--confirm-delete')
  }

  return cargoArgs
}

export async function spawnCargoStorageCommand(parsed: ParsedStorageArgs): Promise<number> {
  if (parsed.subcommand === 'doctor') {
    await runStorageDoctor(parsed)
    return 0
  }

  const cargoArgs = cargoStorageArgs(parsed, parsed.subcommand)
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

async function readCargoStorageStatus(parsed: ParsedStorageArgs): Promise<StorageStatusEnvelope> {
  const cargoArgs = cargoStorageArgs(parsed, 'status')
  const cargoCommand = process.platform === 'win32' ? 'cargo.exe' : 'cargo'
  const workspaceRoot = resolveWorkspaceRoot()

  return new Promise<StorageStatusEnvelope>((resolveStatus, rejectStatus) => {
    const child = spawn(cargoCommand, cargoArgs, {
      cwd: workspaceRoot,
      stdio: ['ignore', 'pipe', 'inherit']
    })

    let stdout = ''

    child.stdout.setEncoding('utf8')
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk
    })

    child.on('error', (err: NodeJS.ErrnoException) => {
      if (err.code === 'ENOENT') {
        rejectStatus(new Error(`cargo not found at workspace root ${workspaceRoot}`))
      } else {
        rejectStatus(new Error(`failed to spawn cargo: ${err.message}`))
      }
    })

    child.on('exit', (code) => {
      if (code !== 0) {
        rejectStatus(new Error(`storage status exited with code ${code ?? 1}`))
        return
      }

      const output = stdout.trim()
      try {
        resolveStatus(parseStorageStatusEnvelope(output))
      } catch (e: unknown) {
        if (e instanceof Error) {
          rejectStatus(e)
          return
        }
        rejectStatus(new Error('failed to parse storage status output'))
      }
    })
  })
}

function parseStorageStatusEnvelope(output: string): StorageStatusEnvelope {
  if (output.length === 0) {
    throw new Error('storage status returned no JSON output')
  }

  const value: unknown = JSON.parse(output)
  if (!isStorageStatusEnvelope(value)) {
    throw new Error('storage status returned an unexpected JSON shape')
  }

  return value
}

function isStorageStatusEnvelope(value: unknown): value is StorageStatusEnvelope {
  if (typeof value !== 'object' || value === null) {
    return false
  }

  const candidate = value as Partial<StorageStatusEnvelope>
  return (
    candidate.type === 'storageStatus' &&
    typeof candidate.userDataPath === 'string' &&
    isStorageEnvironmentStatus(candidate.development, true) &&
    isStorageEnvironmentStatus(candidate.production, false)
  )
}

function isStorageEnvironmentStatus(
  value: unknown,
  schemaRequired: boolean
): value is StorageEnvironmentStatus {
  if (typeof value !== 'object' || value === null) {
    return false
  }

  const candidate = value as Partial<StorageEnvironmentStatus>
  const hasExpectedStorageShape =
    typeof candidate.environment === 'string' &&
    typeof candidate.storageRootPath === 'string' &&
    typeof candidate.storageRootExists === 'boolean' &&
    typeof candidate.durableStorePath === 'string' &&
    typeof candidate.durableStoreExists === 'boolean' &&
    typeof candidate.artifactFileStorePath === 'string' &&
    typeof candidate.artifactFileStoreExists === 'boolean' &&
    typeof candidate.walPath === 'string' &&
    typeof candidate.walExists === 'boolean' &&
    typeof candidate.shmPath === 'string' &&
    typeof candidate.shmExists === 'boolean'

  if (!hasExpectedStorageShape) {
    return false
  }

  if (schemaRequired) {
    return isStorageSchemaStatus(candidate.schema)
  }

  return candidate.schema === undefined || isStorageSchemaStatus(candidate.schema)
}

function isStorageSchemaStatus(value: unknown): value is StorageSchemaStatus {
  if (typeof value !== 'object' || value === null) {
    return false
  }

  const candidate = value as Partial<StorageSchemaStatus>
  return (
    (candidate.state === 'missing' ||
      candidate.state === 'compatible' ||
      candidate.state === 'incompatible' ||
      candidate.state === 'unreadable') &&
    (candidate.detail === undefined || typeof candidate.detail === 'string')
  )
}

async function runStorageDoctor(parsed: ParsedStorageArgs): Promise<void> {
  const status = await readCargoStorageStatus(parsed)
  const development = status.development
  const schema = development.schema
  if (schema === undefined) {
    throw new Error('storage status did not include development schema compatibility')
  }

  console.log(`[storage:doctor] development storage root: ${development.storageRootPath}`)
  console.log(
    `[storage:doctor] development storage root exists: ${yesNo(development.storageRootExists)}`
  )
  console.log(`[storage:doctor] development database: ${development.durableStorePath}`)
  console.log(
    `[storage:doctor] development database exists: ${yesNo(development.durableStoreExists)}`
  )
  console.log(`[storage:doctor] artifact file store: ${development.artifactFileStorePath}`)
  console.log(
    `[storage:doctor] artifact file store exists: ${yesNo(development.artifactFileStoreExists)}`
  )
  console.log(`[storage:doctor] schema compatibility: ${schema.state}`)
  if (schema.detail !== undefined) {
    console.log(`[storage:doctor] schema detail: ${schema.detail}`)
  }

  if (schema.state === 'missing') {
    console.log(
      '[storage:doctor] no development database exists; normal dev startup will create one'
    )
  } else if (schema.state === 'compatible') {
    console.log(
      '[storage:doctor] development database is compatible with the current storage schema'
    )
  } else {
    console.log(`[storage:doctor] reset development storage with: ${resetCommand(parsed)}`)
  }
}

function yesNo(value: boolean): 'yes' | 'no' {
  return value ? 'yes' : 'no'
}

function resetCommand(parsed: ParsedStorageArgs): string {
  const base = 'pnpm --filter @dekzer/desktop run storage:reset --'
  if (parsed.userDataSource === 'argument') {
    return `${base} --user-data "${parsed.userDataPath}" --confirm-delete`
  }

  return `${base} --confirm-delete`
}

export type DevelopmentStorageCheck = {
  readonly schema: StorageSchemaStatus
  readonly storageRootPath: string
  readonly durableStorePath: string
  readonly userDataPath: string
  readonly userDataSource: UserDataSource
}

export async function checkDevelopmentStorage(
  userDataPath: string,
  userDataSource: UserDataSource
): Promise<DevelopmentStorageCheck> {
  const parsed: ParsedStorageArgs = {
    subcommand: 'status',
    userDataPath,
    userDataSource,
    confirmDelete: false
  }
  const envelope = await readCargoStorageStatus(parsed)
  const schema = envelope.development.schema
  if (schema === undefined) {
    throw new Error('storage status did not include development schema compatibility')
  }
  return {
    schema,
    storageRootPath: envelope.development.storageRootPath,
    durableStorePath: envelope.development.durableStorePath,
    userDataPath,
    userDataSource
  }
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

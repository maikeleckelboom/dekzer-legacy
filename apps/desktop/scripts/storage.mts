import { spawn } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const scriptDir = dirname(fileURLToPath(import.meta.url))
const workspaceRoot = resolve(scriptDir, '..', '..', '..')
const defaultDevUserDataPath = resolve(workspaceRoot, '.dev-user-data', 'default')

const subcommand = process.argv[2]
const extraArgs = process.argv.slice(3)

let userDataPath = defaultDevUserDataPath
let confirmDelete = false
const passthroughArgs: string[] = []

let i = 0
while (i < extraArgs.length) {
  const arg = extraArgs[i]
  if (arg === '--user-data' && i + 1 < extraArgs.length) {
    const value = extraArgs[i + 1]
    if (value !== undefined) {
      userDataPath = value
    }
    i += 2
  } else if (arg === '--confirm-delete') {
    confirmDelete = true
    i += 1
  } else if (arg === '--') {
    i += 1
  } else if (arg !== undefined) {
    passthroughArgs.push(arg)
    i += 1
  } else {
    i += 1
  }
}

if (subcommand !== 'status' && subcommand !== 'reset') {
  console.error(`[storage] error: unknown subcommand "${subcommand ?? ''}"; expected status or reset`)
  process.exit(1)
}

if (subcommand === 'reset' && !confirmDelete) {
  console.error('[storage] error: --confirm-delete is required for reset')
  process.exit(1)
}

console.log(`[storage] user data path: ${userDataPath}`)

const cargoArgs = [
  'run',
  '-p',
  'library-boundary-stdio',
  '--',
  'storage',
  subcommand,
  '--user-data',
  userDataPath
]

if (subcommand === 'reset' && confirmDelete) {
  cargoArgs.push('--confirm-delete')
}

cargoArgs.push(...passthroughArgs)

const cargoCommand = process.platform === 'win32' ? 'cargo.exe' : 'cargo'

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
  process.exit(1)
})

child.on('exit', (code) => {
  process.exit(code ?? 1)
})

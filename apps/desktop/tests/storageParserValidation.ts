import { strict as assert } from 'node:assert'
import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const thisDir = dirname(fileURLToPath(import.meta.url))
const storageScript = resolve(thisDir, '..', 'tooling', 'storage.mts')
const desktopRoot = resolve(thisDir, '..')

function runScript(
  args: readonly string[],
  env?: Record<string, string | undefined>
): { stdout: string; stderr: string; status: number | null } {
  const tsxArgs = args.map((a) => (a.includes(' ') ? `"${a}"` : a)).join(' ')
  const result = spawnSync(`pnpm exec tsx ${storageScript} ${tsxArgs}`, {
    cwd: desktopRoot,
    encoding: 'utf8',
    env: { ...process.env, ...env },
    shell: true
  })
  return { stdout: result.stdout ?? '', stderr: result.stderr ?? '', status: result.status }
}

main()

function main(): void {
  validatesDefaultPath()
  validatesEnvOverride()
  validatesExplicitUserData()
  validatesExplicitUserDataEqualsForm()
  validatesMissingUserDataValue()
  validatesUserDataValueStartingWithDash()
  validatesDuplicateUserData()
  validatesDuplicateConfirmDelete()
  validatesResetWithoutConfirm()
  validatesUnknownArgument()
  validatesUnknownSubcommand()
  validatesRelativePathRejected()
}

function validatesDefaultPath(): void {
  const { stdout, stderr, status } = runScript(['status'], { DESKTOP_LIBRARY_USER_DATA_PATH: '' })
  assert.equal(status, 0, `expected status 0, got ${status}: stderr: ${stderr}`)
  assert.ok(stdout.includes('[storage] user data path:'), stdout)
  assert.ok(stdout.includes('[storage] source: developmentDefault'), stdout)
}

function validatesEnvOverride(): void {
  const overridePath = resolve(desktopRoot, 'custom-user-data')
  const { stdout, stderr, status } = runScript(['status'], {
    DESKTOP_LIBRARY_USER_DATA_PATH: overridePath
  })
  assert.equal(status, 0, `expected status 0, got ${status}: stderr: ${stderr}`)
  assert.ok(stdout.includes(overridePath), stdout)
  assert.ok(stdout.includes('[storage] source: environmentOverride'), stdout)
}

function validatesExplicitUserData(): void {
  const customPath = resolve(desktopRoot, 'explicit-path')
  const { stdout, stderr, status } = runScript(
    ['status', '--user-data', customPath],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.equal(status, 0, `expected status 0, got ${status}: stderr: ${stderr}`)
  assert.ok(stdout.includes(customPath), stdout)
  assert.ok(stdout.includes('[storage] source: argument'), stdout)
}

function validatesExplicitUserDataEqualsForm(): void {
  const customPath = resolve(desktopRoot, 'explicit-path-eq')
  const { stdout, stderr, status } = runScript(
    ['reset', `--user-data=${customPath}`, '--confirm-delete'],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.equal(status, 0, `expected status 0, got ${status}: stderr: ${stderr}`)
  assert.ok(stdout.includes(customPath), stdout)
  assert.ok(stdout.includes('[storage] source: argument'), stdout)
}

function validatesMissingUserDataValue(): void {
  const { stderr, status } = runScript(['status', '--user-data'], {
    DESKTOP_LIBRARY_USER_DATA_PATH: ''
  })
  assert.notEqual(status, 0)
  assert.ok(
    stderr.includes('--user-data requires an absolute path value'),
    `stderr: ${stderr}`
  )
}

function validatesUserDataValueStartingWithDash(): void {
  const { stderr, status } = runScript(
    ['status', '--user-data', '--confirm-delete'],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.notEqual(status, 0)
  assert.ok(
    stderr.includes('--user-data requires an absolute path value'),
    `stderr: ${stderr}`
  )
}

function validatesDuplicateUserData(): void {
  const { stderr, status } = runScript(
    ['status', '--user-data', resolve(desktopRoot, 'dup-a'), '--user-data', resolve(desktopRoot, 'dup-b')],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.notEqual(status, 0)
  assert.ok(stderr.includes('duplicate --user-data'), `stderr: ${stderr}`)
}

function validatesDuplicateConfirmDelete(): void {
  const { stderr, status } = runScript(
    ['reset', '--confirm-delete', '--confirm-delete'],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.notEqual(status, 0)
  assert.ok(stderr.includes('duplicate --confirm-delete'), `stderr: ${stderr}`)
}

function validatesResetWithoutConfirm(): void {
  const { stderr, status } = runScript(['reset'], {
    DESKTOP_LIBRARY_USER_DATA_PATH: ''
  })
  assert.notEqual(status, 0)
  assert.ok(
    stderr.includes('--confirm-delete is required for reset'),
    `stderr: ${stderr}`
  )
}

function validatesUnknownArgument(): void {
  const { stderr, status } = runScript(['status', '--foo'], {
    DESKTOP_LIBRARY_USER_DATA_PATH: ''
  })
  assert.notEqual(status, 0)
  assert.ok(stderr.includes('unknown argument'), `stderr: ${stderr}`)
}

function validatesUnknownSubcommand(): void {
  const { stderr, status } = runScript(['unknown'], {
    DESKTOP_LIBRARY_USER_DATA_PATH: ''
  })
  assert.notEqual(status, 0)
  assert.ok(stderr.includes('unknown subcommand'), `stderr: ${stderr}`)
}

function validatesRelativePathRejected(): void {
  const { stderr, status } = runScript(
    ['status', '--user-data', 'relative/path'],
    { DESKTOP_LIBRARY_USER_DATA_PATH: '' }
  )
  assert.notEqual(status, 0)
  assert.ok(
    stderr.includes('--user-data requires an absolute path value'),
    `stderr: ${stderr}`
  )
}

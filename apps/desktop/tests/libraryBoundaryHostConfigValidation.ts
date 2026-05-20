import { strict as assert } from 'node:assert'
import {
  existsSync,
  mkdtempSync,
  rmSync,
  writeFileSync
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  resolveLibraryBoundaryStdioBinaryPath,
  selectLibraryBoundaryHostEnvironment
} from '../src/main/libraryBoundaryHostConfig'
import { LibraryBoundaryHostError } from '../src/main/libraryBoundaryHostErrors'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-host-'))

try {
  assert.equal(selectLibraryBoundaryHostEnvironment(true), 'development')
  assert.equal(selectLibraryBoundaryHostEnvironment(false), 'production')

  const fakeBinaryPath = join(tempRoot, 'library-boundary-stdio')
  writeFileSync(fakeBinaryPath, '')

  const devConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux'
  })

  assert.equal(devConfig.environment, 'development')
  assert.equal(devConfig.userDataPath, join(tempRoot, 'user-data'))
  assert.deepEqual(devConfig.binaryPolicy, {
    kind: 'developmentBinary',
    binaryPath: resolve(fakeBinaryPath),
    source: 'environmentOverride'
  })
  assert.equal(
    resolveLibraryBoundaryStdioBinaryPath(devConfig.binaryPolicy),
    resolve(fakeBinaryPath)
  )

  const defaultDevConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: true,
    env: {},
    platform: 'win32'
  })

  assert.deepEqual(defaultDevConfig.binaryPolicy, {
    kind: 'developmentBinary',
    binaryPath: join(
      tempRoot,
      'target',
      'debug',
      'library-boundary-stdio.exe'
    ),
    source: 'repoDebugTarget'
  })

  assertHostError(
    () =>
      resolveLibraryBoundaryStdioBinaryPath(
        defaultDevConfig.binaryPolicy,
        () => false
      ),
    'missingDevelopmentBinary'
  )

  const prodConfig = resolveLibraryBoundaryHostConfig({
    app: testApp({ appPath: join(tempRoot, 'apps', 'desktop') }),
    isDev: false,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: fakeBinaryPath
    },
    platform: 'linux',
    resourcesPath: join(tempRoot, 'resources')
  })

  assert.equal(prodConfig.environment, 'production')
  assert.deepEqual(prodConfig.binaryPolicy, {
    kind: 'packagedBinaryUnavailable',
    executableName: 'library-boundary-stdio',
    resourceRoot: join(tempRoot, 'resources')
  })
  assertHostError(
    () => resolveLibraryBoundaryStdioBinaryPath(prodConfig.binaryPolicy),
    'packagedBinaryUnavailable'
  )

  assert.equal(existsSync(fakeBinaryPath), true)
} finally {
  rmSync(tempRoot, { recursive: true, force: true })
}

function testApp(options: { readonly appPath: string }): {
  getPath(name: 'userData'): string
  getAppPath(): string
} {
  return {
    getPath(name: 'userData'): string {
      assert.equal(name, 'userData')
      return join(tempRoot, 'user-data')
    },
    getAppPath(): string {
      return options.appPath
    }
  }
}

function assertHostError(
  action: () => void,
  code: LibraryBoundaryHostError['code']
): void {
  assert.throws(action, (error: unknown) => {
    assert.equal(error instanceof LibraryBoundaryHostError, true)
    assert.equal((error as LibraryBoundaryHostError).code, code)
    return true
  })
}

import { strict as assert } from 'node:assert'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, relative } from 'node:path'

import {
  libraryBoundaryUserDataEnvironmentVariable,
  resolveLibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import { assertHostError, testApp } from './support/libraryBoundary'
import {
  desktopRoot,
  listSourceFiles,
  normalizePath,
  rendererSourceRoot,
  sourceRoot
} from './support/files'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-storage-environment-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  validatesProductionAndDevelopmentStorageRootsAreExplicit()
  validatesUserDataEnvironmentOverride()
  validatesInvalidUserDataRootsAreRejected()
  validatesUserDataOverrideIsResolvedAtOneMainBoundary()
  validatesRendererHasNoStoragePathAuthority()
}

function validatesProductionAndDevelopmentStorageRootsAreExplicit(): void {
  const appPath = join(tempRoot, 'apps', 'desktop')
  const app = testApp(tempRoot, { appPath })
  const developmentConfig = resolveLibraryBoundaryHostConfig({
    app,
    isDev: true,
    env: {},
    platform: 'linux'
  })
  const productionConfig = resolveLibraryBoundaryHostConfig({
    app,
    isDev: false,
    env: {},
    platform: 'linux'
  })

  assert.equal(developmentConfig.environment, 'development')
  assert.deepEqual(developmentConfig.storageEnvironment, {
    kind: 'userDataRoot',
    userDataPath: join(tempRoot, 'user-data'),
    source: 'electronUserData'
  })
  assert.equal(productionConfig.environment, 'production')
  assert.deepEqual(productionConfig.storageEnvironment, developmentConfig.storageEnvironment)
}

function validatesUserDataEnvironmentOverride(): void {
  const appPath = join(tempRoot, 'apps', 'desktop')
  const overridePath = join(tempRoot, 'diagnostic-user-data')
  const config = resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath }),
    isDev: true,
    env: {
      [libraryBoundaryUserDataEnvironmentVariable]: overridePath
    },
    platform: 'linux'
  })

  assert.deepEqual(config.storageEnvironment, {
    kind: 'userDataRoot',
    userDataPath: overridePath,
    source: 'environmentOverride'
  })
}

function validatesInvalidUserDataRootsAreRejected(): void {
  const appPath = join(tempRoot, 'apps', 'desktop')

  assertHostError(
    () =>
      resolveLibraryBoundaryHostConfig({
        app: testApp(tempRoot, { appPath }),
        isDev: true,
        env: {
          [libraryBoundaryUserDataEnvironmentVariable]: 'relative-user-data'
        },
        platform: 'linux'
      }),
    'invalidUserDataPath'
  )

  assertHostError(
    () =>
      resolveLibraryBoundaryHostConfig({
        app: {
          getPath: () => '',
          getAppPath: () => appPath
        },
        isDev: true,
        env: {},
        platform: 'linux'
      }),
    'invalidUserDataPath'
  )
}

function validatesUserDataOverrideIsResolvedAtOneMainBoundary(): void {
  const violations = sourceFilesContaining(
    sourceRoot,
    new RegExp(libraryBoundaryUserDataEnvironmentVariable)
  ).filter((filePath) => filePath !== 'src/main/libraryBoundary/config.ts')

  assert.deepEqual(violations, [])
}

function validatesRendererHasNoStoragePathAuthority(): void {
  const violations = sourceFilesContaining(
    rendererSourceRoot,
    /\b(DEKZER_LIBRARY_USER_DATA_PATH|userDataPath|library\.sqlite3|sqlite)\b/i
  )

  assert.deepEqual(violations, [])
}

function sourceFilesContaining(root: string, pattern: RegExp): readonly string[] {
  const violations: string[] = []

  for (const filePath of listSourceFiles(root)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    if (pattern.test(contents)) {
      violations.push(relativePath)
    }
  }

  return violations
}

import { strict as assert } from 'node:assert'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { createLibraryBoundaryClient } from '../../../packages/library-boundary-client/src/index.js'
import { createLibraryBoundaryStdioTransport } from '../../../packages/library-boundary-stdio-transport/src/index.js'
import {
  libraryBoundaryStdioBinaryEnvironmentVariable,
  resolveLibraryBoundaryHostConfig,
  type LibraryBoundaryHostConfig
} from '../src/main/libraryBoundary/config'
import { LibraryBoundaryHost } from '../src/main/libraryBoundary/host'
import { readThroughHost } from '../src/main/libraryHierarchy/readChildren'
import { registerLocalRoot } from '../src/main/libraryRoots/registerLocalRoot'
import { runLocalRootScanThroughHost } from '../src/main/libraryRoots/runScan'
import type {
  LibraryHierarchyReadChildrenNode,
  LibraryHierarchyReadChildrenRoot
} from '../src/shared/libraryHierarchy/readChildren'
import { desktopRoot } from './support/files'
import { silentLogger, testApp } from './support/libraryBoundary'
import { firstAvailableSourceReadRequest } from './support/libraryHierarchy'

const tempRoot = mkdtempSync(join(tmpdir(), 'dekzer-desktop-local-library-restart-'))

void main()
  .catch((error: unknown) => {
    console.error(error)
    process.exitCode = 1
  })
  .finally(() => {
    rmSync(tempRoot, { recursive: true, force: true })
  })

async function main(): Promise<void> {
  const musicRoot = join(tempRoot, 'music-root')
  const crateDirectory = join(musicRoot, 'Crate')
  mkdirSync(crateDirectory, { recursive: true })
  writeFileSync(join(crateDirectory, 'amen.wav'), Buffer.from('not-real-audio'))
  const boundaryBinaryPath = resolveBoundaryBinaryPath()
  const expectedUserDataPath = join(tempRoot, 'user-data')

  const config = resolveLibraryBoundaryHostConfig({
    app: testApp(tempRoot, { appPath: desktopRoot }),
    isDev: true,
    env: {
      [libraryBoundaryStdioBinaryEnvironmentVariable]: boundaryBinaryPath
    },
    platform: process.platform
  })
  const userDataPath = config.storageEnvironment.userDataPath
  const developmentDatabasePath = join(userDataPath, 'development', 'library.sqlite3')

  assert.deepEqual(config.storageEnvironment, {
    kind: 'userDataRoot',
    userDataPath: expectedUserDataPath,
    source: 'electronUserData'
  })
  assert.equal(existsSync(userDataPath), false)
  assertStorageStatus(readStorageStatus(boundaryBinaryPath, userDataPath), {
    userDataPath,
    developmentDatabasePath,
    developmentDatabaseExists: false,
    developmentStorageRootExists: false
  })

  const firstHost = await startRealBoundaryHost(config)
  let firstRead: Awaited<ReturnType<typeof readPersistedCrateWindow>>
  try {
    const registered = await registerLocalRoot(firstHost, {
      absolutePath: musicRoot
    })

    assert.equal(registered.state, 'registered')
    if (registered.state !== 'registered') {
      assert.fail('expected local root registration to succeed')
    }

    const scan = await runLocalRootScanThroughHost(firstHost, {
      rootId: registered.root.rootId
    })
    assert.equal(scan.state, 'scanned')
    if (scan.state !== 'scanned') {
      assert.fail('expected local root scan to succeed')
    }
    assert.equal(scan.rootId, registered.root.rootId)
    assert.equal(scan.discoveredFileCount, 1)

    firstRead = await readPersistedCrateWindow(firstHost)
    assert.equal(firstRead.crateFile.label, 'amen.wav')
    assertStorageStatus(readStorageStatus(boundaryBinaryPath, userDataPath), {
      userDataPath,
      developmentDatabasePath,
      developmentDatabaseExists: true,
      developmentStorageRootExists: true
    })
  } finally {
    await firstHost.stop()
  }

  const restartedHost = await startRealBoundaryHost(config)
  try {
    const restartedRead = await readPersistedCrateWindow(restartedHost)
    assert.deepEqual(restartedRead.crateRoot, firstRead.crateRoot)
    assert.deepEqual(restartedRead.crateDirectory, firstRead.crateDirectory)
    assert.deepEqual(restartedRead.crateFile, firstRead.crateFile)
    assertStorageStatus(readStorageStatus(boundaryBinaryPath, userDataPath), {
      userDataPath,
      developmentDatabasePath,
      developmentDatabaseExists: true,
      developmentStorageRootExists: true
    })
  } finally {
    await restartedHost.stop()
  }
}

async function startRealBoundaryHost(
  config: LibraryBoundaryHostConfig
): Promise<LibraryBoundaryHost> {
  const host = new LibraryBoundaryHost(config, silentLogger(), {
    createClient: createLibraryBoundaryClient,
    createTransport: createLibraryBoundaryStdioTransport
  })

  await host.start()
  return host
}

async function readPersistedCrateWindow(host: LibraryBoundaryHost): Promise<{
  readonly crateRoot: LibraryHierarchyReadChildrenRoot
  readonly crateDirectory: LibraryHierarchyReadChildrenNode
  readonly crateFile: LibraryHierarchyReadChildrenNode
}> {
  const rootRead = await readThroughHost(host, firstAvailableSourceReadRequest())
  assert.equal(rootRead.state, 'ready')
  if (rootRead.state !== 'ready') {
    assert.fail('expected first available source hierarchy read to be ready')
  }

  const crateDirectory = rootRead.window.nodes.find((node) => node.label === 'Crate')
  assert.equal(crateDirectory?.kind, 'directory')
  if (crateDirectory?.kind !== 'directory') {
    assert.fail('expected scanned Crate directory to be available')
  }

  const crateRead = await readThroughHost(host, {
    target: {
      kind: 'entryPoint',
      entryPoint: rootRead.window.root.entryPoint,
      ...(rootRead.window.root.label === undefined ? {} : { label: rootRead.window.root.label })
    },
    parentSourceDirectoryId: crateDirectory.sourceDirectoryId,
    offset: 0,
    limit: 50
  })
  assert.equal(crateRead.state, 'ready')
  if (crateRead.state !== 'ready') {
    assert.fail('expected Crate directory hierarchy read to be ready')
  }

  const crateFile = crateRead.window.nodes.find((node) => node.label === 'amen.wav')
  assert.equal(crateFile?.kind, 'file')
  if (crateFile?.kind !== 'file') {
    assert.fail('expected scanned amen.wav file to be available')
  }

  return {
    crateRoot: rootRead.window.root,
    crateDirectory,
    crateFile
  }
}

function resolveBoundaryBinaryPath(): string {
  const configuredPath = process.env[libraryBoundaryStdioBinaryEnvironmentVariable]?.trim()
  const binaryPath =
    configuredPath !== undefined && configuredPath.length > 0
      ? resolve(configuredPath)
      : resolve(
          desktopRoot,
          '..',
          '..',
          'target',
          'debug',
          process.platform === 'win32' ? 'library-boundary-stdio.exe' : 'library-boundary-stdio'
        )

  assert.equal(existsSync(binaryPath), true, `expected Rust stdio binary to exist at ${binaryPath}`)
  return binaryPath
}

type StorageStatusEnvelope = {
  readonly type: 'storageStatus'
  readonly userDataPath: string
  readonly development: StorageEnvironmentStatus
  readonly production: StorageEnvironmentStatus
}

type StorageEnvironmentStatus = {
  readonly environment: 'development' | 'production'
  readonly storageRootPath: string
  readonly storageRootExists: boolean
  readonly durableStorePath: string
  readonly durableStoreExists: boolean
  readonly artifactFileStorePath: string
  readonly artifactFileStoreExists: boolean
  readonly walPath: string
  readonly walExists: boolean
  readonly shmPath: string
  readonly shmExists: boolean
}

function readStorageStatus(binaryPath: string, userDataPath: string): StorageStatusEnvelope {
  const result = spawnSync(binaryPath, ['storage', 'status', '--user-data', userDataPath], {
    encoding: 'utf8'
  })

  if (result.error !== undefined) {
    throw result.error
  }

  assert.equal(result.status, 0, result.stderr)
  const parsed = JSON.parse(result.stdout.trim()) as StorageStatusEnvelope
  assert.equal(parsed.type, 'storageStatus')
  return parsed
}

function assertStorageStatus(
  status: StorageStatusEnvelope,
  expected: {
    readonly userDataPath: string
    readonly developmentDatabasePath: string
    readonly developmentDatabaseExists: boolean
    readonly developmentStorageRootExists: boolean
  }
): void {
  assert.equal(status.userDataPath, expected.userDataPath)
  assert.equal(status.development.environment, 'development')
  assert.equal(status.development.storageRootPath, join(expected.userDataPath, 'development'))
  assert.equal(status.development.storageRootExists, expected.developmentStorageRootExists)
  assert.equal(status.development.durableStorePath, expected.developmentDatabasePath)
  assert.equal(status.development.durableStoreExists, expected.developmentDatabaseExists)
  assert.equal(
    status.development.artifactFileStorePath,
    join(expected.userDataPath, 'development', 'library.sqlite3.artifact-file-store')
  )
  assert.equal(
    status.development.walPath,
    join(expected.userDataPath, 'development', 'library.sqlite3-wal')
  )
  assert.equal(
    status.development.shmPath,
    join(expected.userDataPath, 'development', 'library.sqlite3-shm')
  )
  assert.equal(status.production.environment, 'production')
  assert.equal(status.production.storageRootPath, expected.userDataPath)
  assert.equal(status.production.durableStorePath, join(expected.userDataPath, 'library.sqlite3'))
  assert.equal(status.production.durableStoreExists, false)
}

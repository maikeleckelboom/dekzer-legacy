import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import process from 'node:process'

import { LibraryBoundaryHostError } from './libraryBoundaryHostErrors'

export const libraryBoundaryStdioBinaryEnvironmentVariable = 'DEKZER_LIBRARY_BOUNDARY_STDIO_BINARY'

export type LibraryBoundaryHostEnvironment = 'development' | 'production'

export type LibraryBoundaryHostApp = {
  getPath(name: 'userData'): string
  getAppPath(): string
}

export type LibraryBoundaryHostDevelopmentBinarySource = 'environmentOverride' | 'repoDebugTarget'

export type LibraryBoundaryHostDevelopmentBinaryPolicy = {
  readonly kind: 'developmentBinary'
  readonly binaryPath: string
  readonly source: LibraryBoundaryHostDevelopmentBinarySource
}

export type LibraryBoundaryHostPackagedBinaryPolicy = {
  readonly kind: 'packagedBinaryUnavailable'
  readonly executableName: string
  readonly resourceRoot: string | null
}

export type LibraryBoundaryHostBinaryPolicy =
  | LibraryBoundaryHostDevelopmentBinaryPolicy
  | LibraryBoundaryHostPackagedBinaryPolicy

export type LibraryBoundaryHostConfig = {
  readonly userDataPath: string
  readonly environment: LibraryBoundaryHostEnvironment
  readonly binaryPolicy: LibraryBoundaryHostBinaryPolicy
}

export type ResolveLibraryBoundaryHostConfigOptions = {
  readonly app: LibraryBoundaryHostApp
  readonly isDev: boolean
  readonly env?: NodeJS.ProcessEnv
  readonly platform?: NodeJS.Platform
  readonly resourcesPath?: string | null
}

export type ResolveLibraryBoundaryStdioBinaryPolicyOptions = {
  readonly isDev: boolean
  readonly desktopAppPath: string
  readonly env?: NodeJS.ProcessEnv
  readonly platform?: NodeJS.Platform
  readonly resourcesPath?: string | null
}

type BinaryExists = (binaryPath: string) => boolean

export function resolveLibraryBoundaryHostConfig(
  options: ResolveLibraryBoundaryHostConfigOptions
): LibraryBoundaryHostConfig {
  return {
    userDataPath: options.app.getPath('userData'),
    environment: selectLibraryBoundaryHostEnvironment(options.isDev),
    binaryPolicy: resolveLibraryBoundaryStdioBinaryPolicy({
      isDev: options.isDev,
      desktopAppPath: options.app.getAppPath(),
      ...(options.env === undefined ? {} : { env: options.env }),
      ...(options.platform === undefined ? {} : { platform: options.platform }),
      ...(options.resourcesPath === undefined ? {} : { resourcesPath: options.resourcesPath })
    })
  }
}

export function selectLibraryBoundaryHostEnvironment(
  isDev: boolean
): LibraryBoundaryHostEnvironment {
  return isDev ? 'development' : 'production'
}

export function resolveLibraryBoundaryStdioBinaryPolicy(
  options: ResolveLibraryBoundaryStdioBinaryPolicyOptions
): LibraryBoundaryHostBinaryPolicy {
  const platform = options.platform ?? process.platform
  const executableName = libraryBoundaryStdioExecutableName(platform)

  if (!options.isDev) {
    return {
      kind: 'packagedBinaryUnavailable',
      executableName,
      resourceRoot: options.resourcesPath ?? defaultElectronResourcesPath()
    }
  }

  const env = options.env ?? process.env
  const overridePath = env[libraryBoundaryStdioBinaryEnvironmentVariable]?.trim()
  if (overridePath !== undefined && overridePath.length > 0) {
    return {
      kind: 'developmentBinary',
      binaryPath: resolve(overridePath),
      source: 'environmentOverride'
    }
  }

  const workspaceRoot = resolve(options.desktopAppPath, '..', '..')
  return {
    kind: 'developmentBinary',
    binaryPath: join(workspaceRoot, 'target', 'debug', executableName),
    source: 'repoDebugTarget'
  }
}

export function resolveLibraryBoundaryStdioBinaryPath(
  policy: LibraryBoundaryHostBinaryPolicy,
  binaryExists: BinaryExists = existsSync
): string {
  if (policy.kind === 'packagedBinaryUnavailable') {
    throw new LibraryBoundaryHostError(
      'packagedBinaryUnavailable',
      'The library boundary stdio binary is not packaged with the desktop app yet.',
      {
        details: {
          executableName: policy.executableName,
          resourceRoot: policy.resourceRoot
        }
      }
    )
  }

  if (!binaryExists(policy.binaryPath)) {
    throw new LibraryBoundaryHostError(
      'missingDevelopmentBinary',
      `Missing development library boundary stdio binary at ${policy.binaryPath}.`,
      {
        details: {
          binaryPath: policy.binaryPath,
          binarySource: policy.source
        }
      }
    )
  }

  return policy.binaryPath
}

function libraryBoundaryStdioExecutableName(platform: NodeJS.Platform): string {
  return platform === 'win32' ? 'library-boundary-stdio.exe' : 'library-boundary-stdio'
}

function defaultElectronResourcesPath(): string | null {
  const resourcesPath = (
    process as NodeJS.Process & {
      readonly resourcesPath?: string
    }
  ).resourcesPath

  return typeof resourcesPath === 'string' && resourcesPath.length > 0 ? resourcesPath : null
}

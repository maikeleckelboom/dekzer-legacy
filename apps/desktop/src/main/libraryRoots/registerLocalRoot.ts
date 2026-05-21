import type { RegisterLocalRootRequest } from '@dekzer/library-boundary-contract'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  libraryRootsIpcChannels,
  type LocalRootRegistrationErrorCode,
  type LocalRootRegistrationErrorState,
  type LocalRootRegistrationRequest,
  type LocalRootRegistrationResult
} from '../../shared/libraryRoots/registerLocalRoot'

export type LocalRootRegistrationIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LocalRootRegistrationResult>
  ): void
}

export function registerLocalRootRegistrationIpc(
  ipcMain: LocalRootRegistrationIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(libraryRootsIpcChannels.registerLocal, (_event, request) =>
    registerLocalRootThroughHost(host, request)
  )
}

export async function registerLocalRootThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<LocalRootRegistrationResult> {
  const normalizedRequest = normalizeLocalRootRegistrationRequest(request)

  if (isLocalRootRegistrationResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isLocalRootRegistrationResult(client)) {
    return client
  }

  try {
    const reply = await client.registerLocalRoot({
      absolutePath: normalizedRequest.absolutePath
    } satisfies RegisterLocalRootRequest)

    return {
      state: 'registered',
      root: {
        rootId: reply.rootId,
        canonicalPath: reply.canonicalPath
      }
    }
  } catch {
    return createLocalRootRegistrationErrorResult(
      'registrationFailed',
      'registrationFailed',
      'Unable to register local library root.'
    )
  }
}

function normalizeLocalRootRegistrationRequest(
  request: unknown
): LocalRootRegistrationRequest | LocalRootRegistrationResult {
  if (!isRecord(request)) {
    return createLocalRootRegistrationErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Local root registration requires a request object.'
    )
  }

  if (typeof request.absolutePath !== 'string' || request.absolutePath.trim().length === 0) {
    return createLocalRootRegistrationErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Local root registration absolutePath is invalid.'
    )
  }

  return {
    absolutePath: request.absolutePath
  }
}

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | LocalRootRegistrationResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createLocalRootRegistrationErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LocalRootRegistrationResult {
  return createLocalRootRegistrationErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LocalRootRegistrationErrorCode {
  if (host.state === 'failed') {
    return 'hostFailed'
  }

  switch (error.code) {
    case 'notStarted':
    case 'alreadyStarted':
    case 'missingDevelopmentBinary':
    case 'packagedBinaryUnavailable':
    case 'stdioTransportStartupFailure':
      return 'hostNotStarted'
    case 'stopping':
      return 'hostStopping'
    case 'stopped':
      return 'hostStopped'
  }
}

function hostErrorMessage(host: LibraryBoundaryHost, error: LibraryBoundaryHostError): string {
  if (host.state === 'failed') {
    return 'The library boundary host is unavailable after startup failure.'
  }

  switch (error.code) {
    case 'stopping':
      return 'The library boundary host is stopping.'
    case 'stopped':
      return 'The library boundary host is stopped.'
    default:
      return 'The library boundary host has not started yet.'
  }
}

function createLocalRootRegistrationErrorResult(
  state: LocalRootRegistrationErrorState,
  code: LocalRootRegistrationErrorCode,
  message: string
): LocalRootRegistrationResult {
  return {
    state,
    error: {
      code,
      message
    }
  }
}

function isLocalRootRegistrationResult(value: unknown): value is LocalRootRegistrationResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && Boolean(value) && !Array.isArray(value)
}

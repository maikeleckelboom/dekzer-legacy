import type {
  ReadSourceMaintenanceRequest as ContractReadSourceMaintenanceRequest,
  RunSourceMaintenanceRequest as ContractRunSourceMaintenanceRequest
} from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import {
  sourceMaintenanceChannels,
  type ReadSourceMaintenanceRequest,
  type ReadSourceMaintenanceResult,
  type RunSourceMaintenanceRequest,
  type RunSourceMaintenanceResult,
  type SourceMaintenanceErrorCode
} from '../../shared/librarySourceMaintenance/sourceMaintenance'

export type SourceMaintenanceIpcMain = {
  handle(
    channel: string,
    listener: (
      event: unknown,
      request: unknown
    ) => Promise<RunSourceMaintenanceResult | ReadSourceMaintenanceResult>
  ): void
}

const positiveOpaqueIdPattern = /^[1-9]\d*$/

export function registerSourceMaintenanceIpc(
  ipcMain: SourceMaintenanceIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(sourceMaintenanceChannels.runSourceMaintenance, (_event, request) =>
    runSourceMaintenanceThroughHost(host, request)
  )
  ipcMain.handle(sourceMaintenanceChannels.readSourceMaintenance, (_event, request) =>
    readSourceMaintenanceThroughHost(host, request)
  )
}

export async function runSourceMaintenanceThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<RunSourceMaintenanceResult> {
  const normalizedRequest = normalizeRunRequest(request)

  if (isRunSourceMaintenanceResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedRunClient(host)

  if (isRunSourceMaintenanceResult(client)) {
    return client
  }

  try {
    const reply = await client.runSourceMaintenance({
      sourceId: normalizedRequest.sourceId,
      ...(normalizedRequest.hashLimit === undefined
        ? {}
        : { hashLimit: normalizedRequest.hashLimit }),
      ...(normalizedRequest.attachmentLimit === undefined
        ? {}
        : { attachmentLimit: normalizedRequest.attachmentLimit }),
      ...(normalizedRequest.probeLimit === undefined
        ? {}
        : { probeLimit: normalizedRequest.probeLimit })
    } satisfies ContractRunSourceMaintenanceRequest)

    return {
      state: 'completed',
      result: reply
    }
  } catch (error: unknown) {
    const { code, message, detail } = classifyBoundaryError(error, 'maintenance')
    return createRunErrorResult('maintenanceFailed', code, message, detail)
  }
}

export async function readSourceMaintenanceThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadSourceMaintenanceResult> {
  const normalizedRequest = normalizeReadRequest(request)

  if (isReadSourceMaintenanceResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedReadClient(host)

  if (isReadSourceMaintenanceResult(client)) {
    return client
  }

  try {
    const reply = await client.readSourceMaintenance({
      sourceId: normalizedRequest.sourceId
    } satisfies ContractReadSourceMaintenanceRequest)

    return {
      state: 'ready',
      snapshot: reply
    }
  } catch (error: unknown) {
    const { code, message, detail } = classifyBoundaryError(error, 'read')
    return createReadErrorResult('readFailed', code, message, detail)
  }
}

function normalizeRunRequest(
  request: unknown
): RunSourceMaintenanceRequest | RunSourceMaintenanceResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceId)) {
    return createRunErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source maintenance requires a positive sourceId.'
    )
  }

  const hashLimit = normalizeOptionalLimit(request.hashLimit, 'hashLimit')
  if (isRunSourceMaintenanceResult(hashLimit)) {
    return hashLimit
  }

  const attachmentLimit = normalizeOptionalLimit(request.attachmentLimit, 'attachmentLimit')
  if (isRunSourceMaintenanceResult(attachmentLimit)) {
    return attachmentLimit
  }

  const probeLimit = normalizeOptionalLimit(request.probeLimit, 'probeLimit')
  if (isRunSourceMaintenanceResult(probeLimit)) {
    return probeLimit
  }

  return {
    sourceId: request.sourceId,
    ...(hashLimit === undefined ? {} : { hashLimit }),
    ...(attachmentLimit === undefined ? {} : { attachmentLimit }),
    ...(probeLimit === undefined ? {} : { probeLimit })
  }
}

function normalizeReadRequest(
  request: unknown
): ReadSourceMaintenanceRequest | ReadSourceMaintenanceResult {
  if (!isRecord(request) || !isPositiveOpaqueId(request.sourceId)) {
    return createReadErrorResult(
      'invalidRequest',
      'invalidRequest',
      'Source maintenance reads require a positive sourceId.'
    )
  }

  return {
    sourceId: request.sourceId
  }
}

function normalizeOptionalLimit(
  value: unknown,
  fieldName: string
): number | undefined | RunSourceMaintenanceResult {
  if (value === undefined || value === null) {
    return undefined
  }

  if (typeof value === 'number' && Number.isInteger(value) && value > 0) {
    return value
  }

  return createRunErrorResult(
    'invalidRequest',
    'invalidRequest',
    `Source maintenance ${fieldName} is invalid.`
  )
}

function getStartedRunClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | RunSourceMaintenanceResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      const code = hostErrorCode(host, error)
      const message = hostErrorMessage(host, error)
      return createRunErrorResult('hostUnavailable', code, message)
    }

    return createRunErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function getStartedReadClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | ReadSourceMaintenanceResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      const code = hostErrorCode(host, error)
      const message = hostErrorMessage(host, error)
      return createReadErrorResult('hostUnavailable', code, message)
    }

    return createReadErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): SourceMaintenanceErrorCode {
  if (host.state === 'failed') {
    return 'hostFailed'
  }

  switch (error.code) {
    case 'invalidUserDataPath':
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

function classifyBoundaryError(
  error: unknown,
  operation: 'maintenance' | 'read'
): {
  readonly code: SourceMaintenanceErrorCode
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: operation === 'maintenance' ? 'maintenanceFailed' : 'readFailed',
      message:
        operation === 'maintenance'
          ? 'Source maintenance failed due to a protocol error.'
          : 'Source maintenance read failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: operation === 'maintenance' ? 'maintenanceFailed' : 'readFailed',
      message: 'Source maintenance received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: operation === 'maintenance' ? 'maintenanceFailed' : 'readFailed',
      message: 'Source maintenance failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: operation === 'maintenance' ? 'maintenanceFailed' : 'readFailed',
      message:
        operation === 'maintenance'
          ? 'Unable to run source maintenance.'
          : 'Unable to read source maintenance.',
      detail: error.message
    }
  }

  return {
    code: operation === 'maintenance' ? 'maintenanceFailed' : 'readFailed',
    message:
      operation === 'maintenance'
        ? 'Unable to run source maintenance.'
        : 'Unable to read source maintenance.'
  }
}

function createRunErrorResult(
  state: Exclude<RunSourceMaintenanceResult['state'], 'completed'>,
  code: SourceMaintenanceErrorCode,
  message: string,
  detail?: string
): RunSourceMaintenanceResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function createReadErrorResult(
  state: Exclude<ReadSourceMaintenanceResult['state'], 'ready'>,
  code: SourceMaintenanceErrorCode,
  message: string,
  detail?: string
): ReadSourceMaintenanceResult {
  return {
    state,
    error: {
      code,
      message,
      ...(detail === undefined ? {} : { detail })
    }
  }
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRunSourceMaintenanceResult(value: unknown): value is RunSourceMaintenanceResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isReadSourceMaintenanceResult(value: unknown): value is ReadSourceMaintenanceResult {
  return isRecord(value) && typeof value.state === 'string'
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

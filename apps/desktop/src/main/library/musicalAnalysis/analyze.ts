import type {
  AnalyzePlayableMediaRequest as ContractAnalyzePlayableMediaRequest,
  AnalyzePlayableMediaReply,
  TrackMusicalAnalysisBasis,
  TrackMusicalAnalysisResult,
  TrackMusicalAnalysisTarget
} from '@dekzer/library-boundary-contract'

import {
  LibraryBoundaryProtocolError,
  LibraryBoundaryReplyMismatchError,
  LibraryBoundaryTransportError
} from '@dekzer/library-boundary-client'

import { LibraryBoundaryHostError } from '../boundary/errors'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import type { MusicalAnalysisRequest } from '../../../shared/library/musicalAnalysis/analyze'

const positiveOpaqueIdPattern = /^[1-9]\d*$/

const fallbackBasis: TrackMusicalAnalysisBasis = {
  adapterKey: 'stratum_dsp_v1',
  adapterVersion: 'dekzer_analyzer_musical_stratum_v1',
  upstreamCrateName: 'stratum-dsp',
  upstreamCrateVersion: '1.0.0',
  upstreamFeatureFlags: [],
  decoderPolicy: 'blocked_before_rust_service_v1',
  inputPolicy: 'blocked_before_adapter_invocation_v1',
  channelMixdownPolicy: 'not_applied',
  normalizationPolicy: 'not_applied',
  upstreamAnalysisConfigPolicy: 'not_applied',
  mlEnabled: false,
  persistenceAuthorized: false,
  authority: 'non_authoritative_advisory_v1'
}

export async function analyzePlayableMediaThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<AnalyzePlayableMediaReply> {
  const normalizedRequest = normalizeRequest(request)

  if (isAnalyzePlayableMediaReply(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host, normalizedRequest)

  if (isAnalyzePlayableMediaReply(client)) {
    return client
  }

  try {
    return await client.analyzePlayableMedia(
      normalizedRequest satisfies ContractAnalyzePlayableMediaRequest
    )
  } catch (error: unknown) {
    const { code, message, detail } = classifyBoundaryError(error)
    return blockedReply(normalizedRequest, code, message, detail)
  }
}

function normalizeRequest(request: unknown): MusicalAnalysisRequest | AnalyzePlayableMediaReply {
  if (!isRecord(request)) {
    return blockedReply(
      unresolvedTarget(),
      'invalid_request',
      'Musical analysis requires a selected playable media row.'
    )
  }

  if (
    !isPositiveOpaqueId(request.playableMediaId) ||
    !isPositiveOpaqueId(request.sourceId) ||
    !isPositiveOpaqueId(request.sourceFileId) ||
    !isPositiveOpaqueId(request.attachmentId)
  ) {
    return blockedReply(
      targetFromUnknownRequest(request),
      'invalid_request',
      'Musical analysis requires positive playable media, source, source file, and attachment IDs.'
    )
  }

  return {
    playableMediaId: request.playableMediaId,
    sourceId: request.sourceId,
    sourceFileId: request.sourceFileId,
    attachmentId: request.attachmentId
  }
}

function getStartedClient(
  host: LibraryBoundaryHost,
  target: TrackMusicalAnalysisTarget
): LibraryBoundaryHostClient | AnalyzePlayableMediaReply {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return blockedReply(target, hostErrorCode(host, error), hostErrorMessage(host, error))
    }

    return blockedReply(target, 'host_failed', 'The library boundary host is unavailable.')
  }
}

function classifyBoundaryError(error: unknown): {
  readonly code: string
  readonly message: string
  readonly detail?: string
} {
  if (error instanceof LibraryBoundaryProtocolError) {
    const payload = error.protocolError.payload
    return {
      code: 'protocol_error',
      message: 'Musical analysis failed due to a protocol error.',
      ...(payload?.detail === undefined
        ? { detail: error.protocolError.type }
        : { detail: payload.detail })
    }
  }

  if (error instanceof LibraryBoundaryReplyMismatchError) {
    return {
      code: 'reply_mismatch',
      message: 'Musical analysis received an unexpected response.',
      detail: `Reply mismatch: expected ${error.expectedFamily}/${error.expectedVariant}, received ${error.actualFamily}/${error.actualVariant}`
    }
  }

  if (error instanceof LibraryBoundaryTransportError) {
    return {
      code: 'transport_error',
      message: 'Musical analysis failed due to a transport error.',
      ...(error.cause instanceof Error ? { detail: error.cause.message } : {})
    }
  }

  if (error instanceof Error) {
    return {
      code: 'analysis_failed',
      message: 'Unable to run musical analysis.',
      detail: error.message
    }
  }

  return {
    code: 'analysis_failed',
    message: 'Unable to run musical analysis.'
  }
}

function blockedReply(
  target: TrackMusicalAnalysisTarget,
  code: string,
  message: string,
  detail?: string
): AnalyzePlayableMediaReply {
  const statusDetail =
    detail === undefined
      ? `Analysis blocked before Rust service execution: ${message}`
      : `Analysis blocked before Rust service execution: ${message} ${detail}`

  return {
    result: {
      target,
      status: 'blocked',
      statusDetail,
      warnings: [
        {
          severity: 'error',
          code,
          message: detail === undefined ? message : `${message} ${detail}`
        }
      ],
      basis: fallbackBasis
    } satisfies TrackMusicalAnalysisResult
  }
}

function hostErrorCode(host: LibraryBoundaryHost, error: LibraryBoundaryHostError): string {
  if (host.state === 'failed') {
    return 'host_failed'
  }

  switch (error.code) {
    case 'invalidUserDataPath':
    case 'notStarted':
    case 'alreadyStarted':
    case 'missingDevelopmentBinary':
    case 'packagedBinaryUnavailable':
    case 'stdioTransportStartupFailure':
      return 'host_not_started'
    case 'stopping':
      return 'host_stopping'
    case 'stopped':
      return 'host_stopped'
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

function targetFromUnknownRequest(request: Record<string, unknown>): TrackMusicalAnalysisTarget {
  return {
    playableMediaId: stringOrUnresolved(request.playableMediaId),
    sourceId: stringOrUnresolved(request.sourceId),
    sourceFileId: stringOrUnresolved(request.sourceFileId),
    attachmentId: stringOrUnresolved(request.attachmentId)
  }
}

function unresolvedTarget(): TrackMusicalAnalysisTarget {
  return {
    playableMediaId: 'unresolved',
    sourceId: 'unresolved',
    sourceFileId: 'unresolved',
    attachmentId: 'unresolved'
  }
}

function stringOrUnresolved(value: unknown): string {
  return typeof value === 'string' && value.trim().length > 0 ? value : 'unresolved'
}

function isAnalyzePlayableMediaReply(value: unknown): value is AnalyzePlayableMediaReply {
  return isRecord(value) && isRecord(value.result)
}

function isPositiveOpaqueId(value: unknown): value is string {
  return typeof value === 'string' && positiveOpaqueIdPattern.test(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

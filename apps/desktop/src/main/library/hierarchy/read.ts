import type { ReadLibraryTreeChildrenRequest } from '@dekzer/library-boundary-contract'

import {
  type ReadErrorCode,
  type ChildRow,
  type HierarchyCoverage,
  type ReadResult
} from '../../../shared/library/hierarchy/read'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../boundary/host'
import { LibraryBoundaryHostError } from '../boundary/errors'
import { mapLibraryTreeNode } from './mapping'
import { createHierarchyReadErrorResult, isReadResult, normalizeRequest } from './request'
import { resolveTarget } from './target'

export async function readThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<ReadResult> {
  const normalizedRequest = normalizeRequest(request)

  if (isReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isReadResult(client)) {
    return client
  }

  try {
    const resolvedTarget = await resolveTarget(client, normalizedRequest.target)

    if (isReadResult(resolvedTarget)) {
      return resolvedTarget
    }

    const reply = await client.readLibraryTreeChildren({
      entryPoint: resolvedTarget.entryPoint,
      parentSourceDirectoryId: normalizedRequest.parentDirectoryId ?? null,
      offset: normalizedRequest.offset,
      limit: normalizedRequest.limit
    } satisfies ReadLibraryTreeChildrenRequest)

    if (reply.window === null) {
      return createHierarchyReadErrorResult(
        'notFound',
        'notFound',
        'The requested library hierarchy target is not available.'
      )
    }

    const nodes = mapLibraryTreeNodes(reply.window.rows)

    if (nodes === undefined) {
      return createHierarchyReadErrorResult(
        'readFailed',
        'readFailed',
        'Unable to read library hierarchy children.'
      )
    }

    return {
      state: 'ready',
      window: {
        root: resolvedTarget.root,
        ...(reply.window.parentSourceDirectoryId === null
          ? {}
          : { parentDirectoryId: reply.window.parentSourceDirectoryId }),
        offset: reply.window.offset,
        limit: reply.window.limit,
        totalRows: reply.window.totalRows,
        coverage: mapCoverage(reply.window.coverage),
        nodes
      }
    }
  } catch {
    return createHierarchyReadErrorResult(
      'readFailed',
      'readFailed',
      'Unable to read library hierarchy children.'
    )
  }
}

function mapCoverage(coverage: {
  readonly state: HierarchyCoverage['state']
  readonly subtreeCoverageComplete: boolean
  readonly emptyResultAuthoritative: boolean
  readonly detail?: string | null
}): HierarchyCoverage {
  return {
    state: coverage.state,
    subtreeCoverageComplete: coverage.subtreeCoverageComplete,
    emptyResultAuthoritative: coverage.emptyResultAuthoritative,
    ...(coverage.detail == null ? {} : { detail: coverage.detail })
  }
}

function getStartedClient(host: LibraryBoundaryHost): LibraryBoundaryHostClient | ReadResult {
  try {
    return host.client
  } catch (error: unknown) {
    if (error instanceof LibraryBoundaryHostError) {
      return hostUnavailableResult(host, error)
    }

    return createHierarchyReadErrorResult(
      'hostUnavailable',
      'hostFailed',
      'The library boundary host is unavailable.'
    )
  }
}

function mapLibraryTreeNodes(
  rows: Parameters<typeof mapLibraryTreeNode>[0][]
): readonly ChildRow[] | undefined {
  const nodes: ChildRow[] = []

  for (const row of rows) {
    const node = mapLibraryTreeNode(row)

    if (node === undefined) {
      return undefined
    }

    nodes.push(node)
  }

  return nodes
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): ReadResult {
  return createHierarchyReadErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(host: LibraryBoundaryHost, error: LibraryBoundaryHostError): ReadErrorCode {
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

import type { ReadLiteralHierarchyChildrenRequest } from '@dekzer/library-boundary-contract'

import {
  hierarchyReadChannels,
  type LibraryHierarchyReadChildrenErrorCode,
  type LibraryHierarchyReadChildrenNode,
  type LibraryHierarchyReadChildrenResult
} from '../../shared/libraryHierarchy/readChildren'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from '../libraryBoundary/host'
import { LibraryBoundaryHostError } from '../libraryBoundary/errors'
import { mapLiteralHierarchyNode } from './mapping'
import {
  createHierarchyReadErrorResult,
  isLibraryHierarchyReadChildrenResult,
  normalizeLibraryHierarchyReadChildrenRequest
} from './request'
import { resolveLibraryHierarchyReadChildrenTarget } from './target'

export type LibraryHierarchyReadChildrenIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LibraryHierarchyReadChildrenResult>
  ): void
}

export function registerLibraryHierarchyReadChildrenIpc(
  ipcMain: LibraryHierarchyReadChildrenIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(hierarchyReadChannels.readChildren, (_event, request) =>
    readLibraryHierarchyChildrenThroughHost(host, request)
  )
}

export async function readLibraryHierarchyChildrenThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<LibraryHierarchyReadChildrenResult> {
  const normalizedRequest = normalizeLibraryHierarchyReadChildrenRequest(request)

  if (isLibraryHierarchyReadChildrenResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isLibraryHierarchyReadChildrenResult(client)) {
    return client
  }

  try {
    const resolvedTarget = await resolveLibraryHierarchyReadChildrenTarget(
      client,
      normalizedRequest.target
    )

    if (isLibraryHierarchyReadChildrenResult(resolvedTarget)) {
      return resolvedTarget
    }

    const reply = await client.readLiteralHierarchyChildren({
      entryPoint: resolvedTarget.entryPoint,
      parentSourceDirectoryId: normalizedRequest.parentSourceDirectoryId ?? null,
      offset: normalizedRequest.offset,
      limit: normalizedRequest.limit
    } satisfies ReadLiteralHierarchyChildrenRequest)

    if (reply.window === null) {
      return createHierarchyReadErrorResult(
        'notFound',
        'notFound',
        'The requested library hierarchy target is not available.'
      )
    }

    const nodes = mapLiteralHierarchyNodes(reply.window.rows)

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
          : { parentSourceDirectoryId: reply.window.parentSourceDirectoryId }),
        offset: reply.window.offset,
        limit: reply.window.limit,
        totalRows: reply.window.totalRows,
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

function getStartedClient(
  host: LibraryBoundaryHost
): LibraryBoundaryHostClient | LibraryHierarchyReadChildrenResult {
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

function mapLiteralHierarchyNodes(
  rows: Parameters<typeof mapLiteralHierarchyNode>[0][]
): readonly LibraryHierarchyReadChildrenNode[] | undefined {
  const nodes: LibraryHierarchyReadChildrenNode[] = []

  for (const row of rows) {
    const node = mapLiteralHierarchyNode(row)

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
): LibraryHierarchyReadChildrenResult {
  return createHierarchyReadErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryHierarchyReadChildrenErrorCode {
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

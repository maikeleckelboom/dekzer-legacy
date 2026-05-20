import type { ReadLiteralHierarchyChildrenRequest } from '@dekzer/library-boundary-contract'

import {
  libraryHierarchyReadIpcChannels,
  type LibraryHierarchyReadErrorCode,
  type LibraryHierarchyReadNode,
  type LibraryHierarchyReadResult
} from '../shared/libraryHierarchyRead'
import type { LibraryBoundaryHost, LibraryBoundaryHostClient } from './libraryBoundaryHost'
import { LibraryBoundaryHostError } from './libraryBoundaryHostErrors'
import { mapLiteralHierarchyNode } from './libraryHierarchyReadMapping'
import {
  createHierarchyReadErrorResult,
  isLibraryHierarchyReadResult,
  normalizeLibraryHierarchyReadRequest
} from './libraryHierarchyReadRequest'
import { resolveLibraryHierarchyReadTarget } from './libraryHierarchyReadTarget'

export type LibraryHierarchyReadIpcMain = {
  handle(
    channel: string,
    listener: (event: unknown, request: unknown) => Promise<LibraryHierarchyReadResult>
  ): void
}

export function registerLibraryHierarchyReadIpc(
  ipcMain: LibraryHierarchyReadIpcMain,
  host: LibraryBoundaryHost
): void {
  ipcMain.handle(libraryHierarchyReadIpcChannels.readLiteralHierarchyChildren, (_event, request) =>
    readLiteralHierarchyChildrenThroughHost(host, request)
  )
}

export async function readLiteralHierarchyChildrenThroughHost(
  host: LibraryBoundaryHost,
  request: unknown
): Promise<LibraryHierarchyReadResult> {
  const normalizedRequest = normalizeLibraryHierarchyReadRequest(request)

  if (isLibraryHierarchyReadResult(normalizedRequest)) {
    return normalizedRequest
  }

  const client = getStartedClient(host)

  if (isLibraryHierarchyReadResult(client)) {
    return client
  }

  try {
    const resolvedTarget = await resolveLibraryHierarchyReadTarget(client, normalizedRequest.target)

    if (isLibraryHierarchyReadResult(resolvedTarget)) {
      return resolvedTarget
    }

    const reply = await client.readLiteralHierarchyChildren({
      entryPoint: resolvedTarget.entryPoint,
      parentSourceDirectoryId: normalizedRequest.parentSourceDirectoryId,
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

    if (nodes === null) {
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
        parentSourceDirectoryId: reply.window.parentSourceDirectoryId,
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
): LibraryBoundaryHostClient | LibraryHierarchyReadResult {
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
): readonly LibraryHierarchyReadNode[] | null {
  const nodes: LibraryHierarchyReadNode[] = []

  for (const row of rows) {
    const node = mapLiteralHierarchyNode(row)

    if (node === null) {
      return null
    }

    nodes.push(node)
  }

  return nodes
}

function hostUnavailableResult(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryHierarchyReadResult {
  return createHierarchyReadErrorResult(
    'hostUnavailable',
    hostErrorCode(host, error),
    hostErrorMessage(host, error)
  )
}

function hostErrorCode(
  host: LibraryBoundaryHost,
  error: LibraryBoundaryHostError
): LibraryHierarchyReadErrorCode {
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

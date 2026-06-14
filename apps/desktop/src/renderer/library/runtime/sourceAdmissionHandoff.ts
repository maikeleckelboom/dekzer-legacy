import type { LocalRootRegistrationRoot } from '../../../shared/library/roots/register'
import type { SourceReadiness } from './sourceReadiness'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import type { RowBinding } from '../state'

export type SourceAdmissionHandoffState = {
  readonly sourceId: string
  readonly sourcePath?: string
}

export type SourceAdmissionHandoffAction =
  | {
      readonly kind: 'viewSource'
      readonly label: 'View source'
      readonly sourceId: string
      readonly enabled: true
    }
  | {
      readonly kind: 'addAnotherSource'
      readonly label: 'Add another source'
      readonly enabled: true
    }

export type SourceAdmissionHandoffProjection = {
  readonly title: 'Source added'
  readonly sourceId: string
  readonly sourceName?: string
  readonly sourcePath?: string
  readonly detail: string
  readonly readinessDetail: string
  readonly actions: readonly SourceAdmissionHandoffAction[]
}

export function sourceAdmissionHandoffFromRoot(
  root: LocalRootRegistrationRoot
): SourceAdmissionHandoffState {
  return {
    sourceId: root.rootId,
    sourcePath: root.admittedRootPath
  }
}

export function projectSourceAdmissionHandoff(input: {
  readonly handoff?: SourceAdmissionHandoffState
  readonly projection?: BrowserProjection
  readonly sourceReadinessByNodeId?: ReadonlyMap<BrowserTreeNodeId, SourceReadiness>
}): SourceAdmissionHandoffProjection | undefined {
  const handoff = input.handoff

  if (handoff === undefined) {
    return undefined
  }

  const visibleSource = visibleSourceForId(input.projection, handoff.sourceId)
  const readiness =
    visibleSource === undefined
      ? undefined
      : input.sourceReadinessByNodeId?.get(visibleSource.nodeId)
  const sourceName = visibleSource?.label

  return {
    title: 'Source added',
    sourceId: handoff.sourceId,
    ...(sourceName === undefined ? {} : { sourceName }),
    ...(handoff.sourcePath === undefined ? {} : { sourcePath: handoff.sourcePath }),
    detail: sourceAddedDetail(sourceName, handoff.sourcePath),
    readinessDetail: readinessDetail(readiness),
    actions: [
      {
        kind: 'viewSource',
        label: 'View source',
        sourceId: handoff.sourceId,
        enabled: true
      },
      {
        kind: 'addAnotherSource',
        label: 'Add another source',
        enabled: true
      }
    ]
  }
}

function visibleSourceForId(
  projection: BrowserProjection | undefined,
  sourceId: string
): { readonly nodeId: BrowserTreeNodeId; readonly label: string } | undefined {
  if (projection?.kind !== 'tree') {
    return undefined
  }

  for (const [nodeId, binding] of projection.bindingsById) {
    if (isSourceBindingForId(binding, sourceId)) {
      return {
        nodeId,
        label: binding.target.label
      }
    }
  }

  return undefined
}

function isSourceBindingForId(
  binding: RowBinding,
  sourceId: string
): binding is Extract<RowBinding, { readonly kind: 'source' }> {
  return (
    binding.kind === 'source' &&
    binding.target.entryPoint.kind === 'source' &&
    binding.target.entryPoint.sourceId === sourceId
  )
}

function sourceAddedDetail(sourceName: string | undefined, sourcePath: string | undefined): string {
  if (sourceName !== undefined && sourcePath !== undefined) {
    return `${sourceName} was added from ${sourcePath}.`
  }

  if (sourceName !== undefined) {
    return `${sourceName} was added.`
  }

  if (sourcePath !== undefined) {
    return `Source added from ${sourcePath}.`
  }

  return 'Source added.'
}

function readinessDetail(readiness: SourceReadiness | undefined): string {
  if (readiness === undefined) {
    return 'Checking source readiness.'
  }

  switch (readiness.kind) {
    case 'ready':
    case 'empty':
    case 'registered':
    case 'scanning':
    case 'rescanRunning':
    case 'missing':
    case 'unavailable':
    case 'blocked':
    case 'failed':
      return readiness.detail
  }
}

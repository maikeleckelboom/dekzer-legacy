import type { LocalRoot } from '../../../shared/libraryRoots/readLocalRoots'
import type { SourceLifecycleRecord } from '../../../shared/librarySourceLifecycle/readSourceLifecycle'
import type { ScanProgressState } from '../boundary/boundaryEvents'
import type { LocalRootScanStatus, LocalRootsReadState } from '../boundary/localRootActions'
import type { RowBinding, SourceState } from '../state'
import type { BrowserProjection } from '../tree/projection'

export type SourceReadinessKind =
  | 'registered'
  | 'scanning'
  | 'rescanRunning'
  | 'ready'
  | 'empty'
  | 'unavailable'
  | 'blocked'
  | 'failed'

export type SourceReadiness = {
  readonly kind: SourceReadinessKind
  readonly sourceNodeId: string
  readonly detail: string
  readonly rootId?: string
}

export type SourceReadinessInput = {
  readonly sourceNodeId: string
  readonly sourceLabel: string
  readonly rootId?: string
  readonly localRoot?: LocalRoot
  readonly sourceLifecycle?: SourceLifecycleRecord
  readonly sourceReadState?: SourceState
  readonly scanProgress?: ScanProgressState
  readonly currentScanStatus?: LocalRootScanStatus
}

export type ProjectSourceReadinessByNodeIdInput = {
  readonly projection: BrowserProjection | undefined
  readonly localRootsReadState: LocalRootsReadState
  readonly sourceReadStates: ReadonlyMap<string, SourceState>
  readonly sourceLifecycleBySourceId?: ReadonlyMap<string, SourceLifecycleRecord>
  readonly scanProgressByRootId: ReadonlyMap<string, ScanProgressState>
  readonly currentScanRootId?: string
  readonly currentScanStatus?: LocalRootScanStatus
}

const sourceUnavailableErrorCodes = new Set([
  'notFound',
  'hostUnavailable',
  'hostStopped',
  'hostStopping',
  'hostFailed',
  'hostNotStarted'
])

export function projectSourceReadinessByNodeId(
  input: ProjectSourceReadinessByNodeIdInput
): ReadonlyMap<string, SourceReadiness> {
  const readinessByNodeId = new Map<string, SourceReadiness>()
  const projection = input.projection

  if (projection === undefined) {
    return readinessByNodeId
  }

  const localRootsById = localRootsByIdFromReadState(input.localRootsReadState)

  for (const [nodeId, binding] of projection.bindingsById) {
    if (binding.kind !== 'source') {
      continue
    }

    const rootId = rootIdForSourceBinding(binding)
    const currentScanStatus =
      rootId !== undefined && rootId === input.currentScanRootId
        ? input.currentScanStatus
        : undefined
    const localRoot = rootId === undefined ? undefined : localRootsById.get(rootId)
    const sourceLifecycle =
      rootId === undefined ? undefined : input.sourceLifecycleBySourceId?.get(rootId)
    const sourceReadState = input.sourceReadStates.get(nodeId)
    const scanProgress = rootId === undefined ? undefined : input.scanProgressByRootId.get(rootId)

    readinessByNodeId.set(
      nodeId,
      deriveSourceReadiness({
        sourceNodeId: nodeId,
        sourceLabel: binding.target.label,
        ...(rootId === undefined ? {} : { rootId }),
        ...(localRoot === undefined ? {} : { localRoot }),
        ...(sourceLifecycle === undefined ? {} : { sourceLifecycle }),
        ...(sourceReadState === undefined ? {} : { sourceReadState }),
        ...(scanProgress === undefined ? {} : { scanProgress }),
        ...(currentScanStatus === undefined ? {} : { currentScanStatus })
      })
    )
  }

  return readinessByNodeId
}

export function deriveSourceReadiness(input: SourceReadinessInput): SourceReadiness {
  const runningScan = runningScanProgress(input.scanProgress, input.currentScanStatus)

  if (runningScan !== undefined) {
    return readiness(
      input,
      hasPriorAuthoritativeRead(input.sourceReadState) ? 'rescanRunning' : 'scanning',
      runningScan
    )
  }

  const lifecycleReadiness = sourceLifecycleReadiness(input)
  if (lifecycleReadiness !== undefined) {
    return lifecycleReadiness
  }

  const terminalProgress = terminalScanProgressReadiness(input)
  if (terminalProgress !== undefined) {
    return terminalProgress
  }

  const stateReadiness = sourceReadStateReadiness(input)
  if (
    input.localRoot?.availability === 'unavailable' &&
    stateReadiness?.kind !== 'blocked' &&
    stateReadiness?.kind !== 'failed'
  ) {
    return readiness(input, 'unavailable', 'The registered source is currently unavailable.')
  }

  if (stateReadiness !== undefined) {
    return stateReadiness
  }

  return readiness(
    input,
    'registered',
    'The source is registered. Scan or expand it to read library rows.'
  )
}

function sourceLifecycleReadiness(input: SourceReadinessInput): SourceReadiness | undefined {
  const lifecycle = input.sourceLifecycle

  if (lifecycle === undefined) {
    return undefined
  }

  if (lifecycle.sourceClass !== 'internal' && lifecycle.mountStatus !== 'mounted') {
    return readiness(input, 'unavailable', 'The registered source is currently unavailable.')
  }

  switch (lifecycle.accessState) {
    case 'missing':
      return readiness(input, 'unavailable', 'The registered source root is missing.')
    case 'blocked':
      return readiness(
        input,
        lifecycle.accessIssueKind === 'unavailableMount' ? 'unavailable' : 'blocked',
        'The registered source root is blocked.'
      )
    case 'accessible':
    case 'unknown':
      break
  }

  switch (lifecycle.scanPhase) {
    case 'scanning':
      return readiness(input, 'scanning', 'The source is still being indexed.')
    case 'blocked':
      return readiness(
        input,
        lifecycle.scanIssueKind === 'unavailableMount' ? 'unavailable' : 'blocked',
        'The source scan is blocked.'
      )
    case 'failed':
      return readiness(input, 'failed', 'The source scan failed.')
    case 'idle':
    case 'complete':
    case 'partial':
      return undefined
  }
}

function runningScanProgress(
  progress: ScanProgressState | undefined,
  currentScanStatus: LocalRootScanStatus | undefined
): string | undefined {
  if (progress?.kind === 'scanning') {
    return scanRunningDetail(progress)
  }

  if (currentScanStatus === 'scanning') {
    return 'Scan is starting for this source.'
  }

  return undefined
}

function terminalScanProgressReadiness(input: SourceReadinessInput): SourceReadiness | undefined {
  const progress = input.scanProgress

  if (progress === undefined) {
    return undefined
  }

  switch (progress.kind) {
    case 'blocked':
      return readiness(input, 'blocked', progress.detail ?? 'The source scan is blocked.')
    case 'failed':
      return readiness(input, 'failed', progress.detail ?? 'The source scan failed.')
    case 'cancelled':
      return readiness(input, 'registered', progress.detail ?? 'The source scan was canceled.')
    case 'completed':
    case 'idle':
    case 'scanning':
      return undefined
  }
}

function sourceReadStateReadiness(input: SourceReadinessInput): SourceReadiness | undefined {
  const state = input.sourceReadState

  if (state === undefined || state.kind === 'unloaded') {
    return undefined
  }

  if (state.kind === 'loading') {
    return readiness(input, 'registered', state.detail ?? 'Reading source hierarchy rows.')
  }

  if (state.kind === 'failed') {
    return readiness(
      input,
      sourceUnavailableErrorCodes.has(state.errorCode) ? 'unavailable' : 'failed',
      state.detail
    )
  }

  const coverage = state.children.coverage

  switch (coverage.state) {
    case 'sourceUnavailable':
    case 'locationMissing':
      return readiness(input, 'unavailable', coverage.detail ?? 'The source is unavailable.')
    case 'blocked':
      return readiness(input, 'blocked', coverage.detail ?? 'The source is blocked.')
    case 'failed':
      return readiness(input, 'failed', coverage.detail ?? 'The source scan failed.')
    case 'scanning':
      return readiness(input, 'scanning', coverage.detail ?? 'The source is still being indexed.')
    case 'pending':
      return readiness(
        input,
        'registered',
        coverage.detail ?? 'The source is registered and awaiting indexed rows.'
      )
    case 'complete':
      return readiness(
        input,
        state.children.rows.length === 0 && coverage.emptyResultAuthoritative ? 'empty' : 'ready',
        state.children.rows.length === 0 && coverage.emptyResultAuthoritative
          ? (coverage.detail ?? 'The source is ready, but no visible rows were found.')
          : 'The source hierarchy is ready.'
      )
  }
}

function hasPriorAuthoritativeRead(state: SourceState | undefined): boolean {
  return state?.kind === 'loaded' || state?.kind === 'refreshing'
}

function readiness(
  input: SourceReadinessInput,
  kind: SourceReadinessKind,
  detail: string
): SourceReadiness {
  return {
    kind,
    sourceNodeId: input.sourceNodeId,
    detail,
    ...(input.rootId === undefined ? {} : { rootId: input.rootId })
  }
}

function scanRunningDetail(
  progress: Extract<ScanProgressState, { readonly kind: 'scanning' }>
): string {
  const parts: string[] = []

  if (progress.filesDiscovered > 0) {
    parts.push(
      `${progress.filesDiscovered} ${progress.filesDiscovered === 1 ? 'file' : 'files'} discovered`
    )
  }

  if (progress.directoriesVisited > 0) {
    parts.push(
      `${progress.directoriesVisited} ${progress.directoriesVisited === 1 ? 'folder' : 'folders'} visited`
    )
  }

  return parts.length === 0
    ? 'Scan is running for this source.'
    : `Scan is running: ${parts.join(', ')}.`
}

function localRootsByIdFromReadState(
  localRootsReadState: LocalRootsReadState
): ReadonlyMap<string, LocalRoot> {
  if (localRootsReadState.kind !== 'ready') {
    return new Map()
  }

  return new Map(localRootsReadState.roots.map((root) => [root.rootId, root]))
}

function rootIdForSourceBinding(
  binding: Extract<RowBinding, { readonly kind: 'source' }>
): string | undefined {
  return binding.target.entryPoint.kind === 'source'
    ? binding.target.entryPoint.sourceId
    : undefined
}

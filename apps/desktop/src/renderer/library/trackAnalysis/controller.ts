import { computed, ref } from 'vue'
import type { ComputedRef, Ref } from 'vue'
import type {
  TrackBeatgridEvidence,
  TrackMusicalAnalysisResult,
  TrackMusicalAnalysisWarning
} from '@dekzer/library-boundary-contract'

import type { RendererApi } from '../../../shared/rendererApi'
import type { PrimarySelection } from '../selection/model'

export type TrackAnalysisStatus = 'idle' | 'running' | 'ready' | 'failed'

export type TrackAnalysisAction = {
  readonly kind: 'analyze'
  readonly label: string
  readonly enabled: boolean
  readonly reason?: string
}

export type TrackAnalysisFact = {
  readonly label: string
  readonly value: string
  readonly detail?: string
}

export type TrackAnalysisView = {
  readonly visible: boolean
  readonly status: TrackAnalysisStatus
  readonly statusLabel: string
  readonly detail: string
  readonly action: TrackAnalysisAction
  readonly facts: readonly TrackAnalysisFact[]
  readonly warningSummary?: string
  readonly warnings: readonly TrackMusicalAnalysisWarning[]
  readonly basisLine?: string
  readonly result?: TrackMusicalAnalysisResult
}

export type TrackAnalysisController = {
  readonly view: ComputedRef<TrackAnalysisView>
  readonly analyzeSelected: () => Promise<boolean>
}

type TrackAnalysisApi = RendererApi['library']['musicalAnalysis']

type TrackAnalysisSelectionTarget = {
  readonly key: string
  readonly request: {
    readonly playableMediaId: string
    readonly sourceId: string
    readonly sourceFileId: string
    readonly attachmentId: string
  }
}

type TrackAnalysisRun =
  | {
      readonly status: 'idle'
    }
  | {
      readonly status: 'running'
      readonly key: string
      readonly sequence: number
    }
  | {
      readonly status: 'ready'
      readonly key: string
      readonly result: TrackMusicalAnalysisResult
    }
  | {
      readonly status: 'failed'
      readonly key: string
      readonly detail: string
    }

const safeAnalysisFailure = 'Unable to request track analysis.'

export function useTrackAnalysis(
  analysisApi: TrackAnalysisApi = getRendererApi().library.musicalAnalysis,
  options: { readonly selection: Ref<PrimarySelection> }
): TrackAnalysisController {
  return createTrackAnalysisController(analysisApi, options)
}

export function createTrackAnalysisController(
  analysisApi: TrackAnalysisApi,
  options: { readonly selection: Ref<PrimarySelection> }
): TrackAnalysisController {
  const run = ref<TrackAnalysisRun>({ status: 'idle' })
  let sequence = 0

  const selectedTarget = computed(() => analysisTargetForSelection(options.selection.value))
  const view = computed<TrackAnalysisView>(() => {
    const target = selectedTarget.value

    if (target === undefined) {
      return hiddenView()
    }

    const currentRun = run.value

    if (currentRun.status === 'running' && currentRun.key === target.key) {
      return {
        visible: true,
        status: 'running',
        statusLabel: 'Analyzing',
        detail: 'Running a one-shot local analysis attempt for this selected playable media row.',
        action: analyzeAction(false, 'Analysis is already running.'),
        facts: [],
        warningSummary: 'No result has returned yet.',
        warnings: []
      }
    }

    if (currentRun.status === 'ready' && currentRun.key === target.key) {
      return viewFromResult(currentRun.result)
    }

    if (currentRun.status === 'failed' && currentRun.key === target.key) {
      return {
        visible: true,
        status: 'failed',
        statusLabel: 'Request failed',
        detail: currentRun.detail,
        action: analyzeAction(true),
        facts: [],
        warningSummary: currentRun.detail,
        warnings: []
      }
    }

    return {
      visible: true,
      status: 'idle',
      statusLabel: 'Not analyzed',
      detail: 'No one-shot musical analysis result is loaded for this selection.',
      action: analyzeAction(true),
      facts: [],
      warningSummary: 'Results are advisory and are not saved to the library.',
      warnings: []
    }
  })

  async function analyzeSelected(): Promise<boolean> {
    const target = selectedTarget.value

    if (target === undefined) {
      return false
    }

    const currentRun = run.value

    if (currentRun.status === 'running' && currentRun.key === target.key) {
      return false
    }

    const requestSequence = ++sequence
    run.value = {
      status: 'running',
      key: target.key,
      sequence: requestSequence
    }

    try {
      const reply = await analysisApi.analyzePlayableMedia(target.request)

      if (!isCurrentRequest(target.key, requestSequence)) {
        return false
      }

      run.value = {
        status: 'ready',
        key: target.key,
        result: reply.result
      }
      return true
    } catch {
      if (!isCurrentRequest(target.key, requestSequence)) {
        return false
      }

      run.value = {
        status: 'failed',
        key: target.key,
        detail: safeAnalysisFailure
      }
      return true
    }
  }

  function isCurrentRequest(key: string, requestSequence: number): boolean {
    const currentRun = run.value
    const target = selectedTarget.value

    return (
      currentRun.status === 'running' &&
      currentRun.key === key &&
      currentRun.sequence === requestSequence &&
      target?.key === key
    )
  }

  return {
    view,
    analyzeSelected
  }
}

function hiddenView(): TrackAnalysisView {
  return {
    visible: false,
    status: 'idle',
    statusLabel: 'Unavailable',
    detail: 'Select playable media to run musical analysis.',
    action: analyzeAction(false, 'Select playable media to run musical analysis.'),
    facts: [],
    warnings: []
  }
}

function viewFromResult(result: TrackMusicalAnalysisResult): TrackAnalysisView {
  return {
    visible: true,
    status: 'ready',
    statusLabel: statusLabel(result.status),
    detail: result.statusDetail,
    action: analyzeAction(true),
    facts: factsFromResult(result),
    warningSummary: warningSummary(result),
    warnings: result.warnings,
    basisLine: basisLine(result),
    result
  }
}

function factsFromResult(result: TrackMusicalAnalysisResult): readonly TrackAnalysisFact[] {
  const facts: TrackAnalysisFact[] = []

  if (result.bpm !== undefined) {
    const detail = confidenceDetail(result.bpm.confidence)
    facts.push({
      label: 'BPM',
      value: formatDecimal(result.bpm.bpm, 2),
      ...(detail === undefined ? {} : { detail })
    })
  }

  if (result.key !== undefined) {
    const detail = joinedDetail([
      modeLabel(result.key.mode),
      confidenceDetail(result.key.confidence)
    ])
    facts.push({
      label: 'Key',
      value: result.key.notation,
      ...(detail === undefined ? {} : { detail })
    })
  }

  if (result.beatgrid !== undefined) {
    facts.push({
      label: 'Beatgrid',
      value: beatgridValue(result.beatgrid),
      detail: beatgridDetail(result.beatgrid)
    })
  }

  return facts
}

function warningSummary(result: TrackMusicalAnalysisResult): string {
  const warnings = result.warnings

  if (warnings.length === 0) {
    return 'No adapter warnings returned; this result is still advisory.'
  }

  const errorCount = warnings.filter((warning) => warning.severity === 'error').length
  const warningCount = warnings.filter((warning) => warning.severity === 'warning').length

  if (errorCount > 0) {
    return `${errorCount} error warning${errorCount === 1 ? '' : 's'} returned.`
  }

  if (warningCount > 0) {
    return `${warningCount} warning${warningCount === 1 ? '' : 's'} returned.`
  }

  return `${warnings.length} advisory note${warnings.length === 1 ? '' : 's'} returned.`
}

function basisLine(result: TrackMusicalAnalysisResult): string {
  const basis = result.basis
  return `${basis.adapterKey} ${basis.adapterVersion}; ${basis.decoderPolicy}; ${basis.inputPolicy}; ${basis.authority}`
}

function beatgridValue(beatgrid: TrackBeatgridEvidence): string {
  if (beatgrid.beatCount === 1) {
    return '1 beat'
  }

  return `${beatgrid.beatCount} beats`
}

function beatgridDetail(beatgrid: TrackBeatgridEvidence): string {
  const preview = beatgrid.previewSeconds.slice(0, 4).map((seconds) => `${formatDecimal(seconds, 2)}s`)
  const previewText = preview.length === 0 ? 'No beat preview returned' : `Preview ${preview.join(', ')}`
  const stability =
    beatgrid.gridStability === undefined
      ? undefined
      : `stability ${formatPercent(beatgrid.gridStability)}`

  return joinedDetail([previewText, stability]) ?? previewText
}

function joinedDetail(values: readonly (string | undefined)[]): string | undefined {
  const joined = values.filter((value): value is string => value !== undefined).join(' - ')
  return joined.length === 0 ? undefined : joined
}

function statusLabel(status: TrackMusicalAnalysisResult['status']): string {
  switch (status) {
    case 'advisory':
      return 'Advisory'
    case 'inconclusive':
      return 'Inconclusive'
    case 'blocked':
      return 'Blocked'
    case 'unsupported':
      return 'Unsupported'
  }
}

function modeLabel(mode: 'major' | 'minor'): string {
  return mode === 'major' ? 'major' : 'minor'
}

function confidenceDetail(confidence: number | undefined): string | undefined {
  return confidence === undefined ? undefined : `confidence ${formatPercent(confidence)}`
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`
}

function formatDecimal(value: number, fractionDigits: number): string {
  return value.toFixed(fractionDigits).replace(/\.?0+$/, '')
}

function analyzeAction(enabled: boolean, reason?: string): TrackAnalysisAction {
  return {
    kind: 'analyze',
    label: 'Analyze',
    enabled,
    ...(reason === undefined ? {} : { reason })
  }
}

function analysisTargetForSelection(
  selection: PrimarySelection
): TrackAnalysisSelectionTarget | undefined {
  if (selection.kind !== 'row' || selection.subject.kind !== 'playableMedia') {
    return undefined
  }

  const subject = selection.subject
  const request = {
    playableMediaId: subject.playableMediaId,
    sourceId: subject.sourceId,
    sourceFileId: subject.sourceFileId,
    attachmentId: subject.attachmentId
  }

  return {
    key: [
      request.playableMediaId,
      request.sourceId,
      request.sourceFileId,
      request.attachmentId
    ].join(':'),
    request
  }
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

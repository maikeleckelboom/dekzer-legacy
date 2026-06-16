import type { ContentProjection, ContentRow } from '../contents/projection'
import type {
  SourceAdmissionHandoffAction,
  SourceAdmissionHandoffProjection
} from '../runtime/sourceAdmissionHandoff'
import type { PrimarySelection } from '../selection/model'
import type { StatusAction, StatusView } from '../sourceStatus/projection'
import type { TrackAnalysisAction, TrackAnalysisView } from '../trackAnalysis/controller'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNode, BrowserTreeNodeId } from '../tree/types'
import type { ViewMode } from '../viewMode/model'

export type WorkstationContextLabel = 'TRACK' | 'SOURCE' | 'SLOT' | 'WORKSPACE' | 'NO SELECTION'

export type WorkstationTreeProps = {
  readonly nodes: readonly BrowserTreeNode[]
  readonly selectedNodeId?: BrowserTreeNodeId
  readonly revealRequest?: {
    readonly nodeId: BrowserTreeNodeId
    readonly sequence: number
  }
  readonly expandedNodeIds: ReadonlySet<BrowserTreeNodeId>
  readonly labelledBy: string
  readonly emptyLabel?: string
}

export type WorkstationBrowseEvents = {
  select: [nodeId: BrowserTreeNodeId]
  toggle: [nodeId: BrowserTreeNodeId]
  activateAction: [nodeId: BrowserTreeNodeId]
  prepare: [nodeId: BrowserTreeNodeId]
  cancelPrepare: [nodeId: BrowserTreeNodeId]
}

export type WorkstationContentsProps = {
  readonly view: ViewMode
  readonly projection: ContentProjection
  readonly columnProjection?: BrowserProjection | undefined
  readonly selectedNodeId?: BrowserTreeNodeId | undefined
  readonly statusView?: StatusView | undefined
  readonly sourceAdmissionHandoff: SourceAdmissionHandoffProjection | undefined
  readonly selectedRowId?: string | undefined
  readonly selectNode: (nodeId: BrowserTreeNodeId) => void
  readonly selectRow: (row: ContentRow) => void
  readonly activateRowAction: (row: ContentRow) => void
  readonly activateStatusAction?: (action: StatusAction) => void
  readonly activateSourceAdmissionHandoffAction?: (action: SourceAdmissionHandoffAction) => void
}

export type WorkstationInspectorProps = {
  readonly selection: PrimarySelection
  readonly status?: StatusView | undefined
  readonly sourceAdmissionHandoff?: SourceAdmissionHandoffProjection | undefined
  readonly trackAnalysis?: TrackAnalysisView | undefined
  readonly activateStatusAction?: (action: StatusAction) => void
  readonly activateSourceAdmissionHandoffAction?: (action: SourceAdmissionHandoffAction) => void
  readonly activateTrackAnalysisAction?: (action: TrackAnalysisAction) => void
}

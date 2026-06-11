import type {
  SearchFilterReadReply,
  SearchFilterReadRequest,
  SearchFilterResult,
  SearchFilterResultRow
} from '@dekzer/library-boundary-contract'

export type {
  SearchFilterAuthorityLayer,
  SearchFilterEvidenceCoverageState,
  SearchFilterIndexState,
  SearchFilterMatchReason,
  SearchFilterQueryIdentity,
  SearchFilterReadReply,
  SearchFilterReadRequest,
  SearchFilterRecursion,
  SearchFilterResult,
  SearchFilterResultKind,
  SearchFilterResultRow,
  SearchFilterScope,
  SearchFilterSet,
  SearchFilterSort,
  SearchFilterState
} from '@dekzer/library-boundary-contract'

export type SearchFilterReadState = 'ready' | 'hostUnavailable' | 'invalidRequest' | 'readFailed'

export type SearchFilterReadErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'invalidRequest'
  | 'readFailed'

export type SearchFilterReadError = {
  readonly code: SearchFilterReadErrorCode
  readonly message: string
  readonly detail?: string
}

export type SearchFilterReadErrorState = Exclude<SearchFilterReadState, 'ready'>

export type SearchFilterReadResult =
  | {
      readonly state: 'ready'
      readonly reply: SearchFilterReadReply
    }
  | {
      readonly state: SearchFilterReadErrorState
      readonly error: SearchFilterReadError
    }

export type SearchFilterProtocolRequest = SearchFilterReadRequest
export type SearchFilterProtocolResult = SearchFilterResult
export type SearchFilterProtocolRow = SearchFilterResultRow

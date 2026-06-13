import type { ContentsFileKind } from '../contents/read'
import type { LocalBrowserEntryPointKind, LocalBrowserEntryPointPlatform } from './entryPoints'

export type LocalBrowserChildrenReadStatus =
  | 'complete'
  | 'partialFailure'
  | 'failed'
  | 'unsupportedPlatform'
  | 'missing'
  | 'permissionBlocked'
  | 'unavailable'

export type LocalBrowserCandidateRowKind =
  | 'directoryCandidate'
  | 'mediaFileCandidate'
  | 'unsupportedFileCandidate'
  | 'rejectedRootCandidate'
  | 'inaccessibleCandidate'
  | 'unknownCandidate'

export type LocalBrowserCandidateStatus =
  | 'available'
  | 'unavailable'
  | 'permissionBlocked'
  | 'missing'
  | 'unsupportedPlatform'
  | 'duplicateOfAdmittedSource'
  | 'rejected'
  | 'unknown'

export type LocalBrowserCandidateMediaRelevance =
  | 'mediaRelevant'
  | 'companionMetadata'
  | 'unsupported'
  | 'unknown'

export type LocalBrowserCandidateAdmissionHint =
  | 'canRequestAdmission'
  | 'requiresConfirmation'
  | 'chooseParentDirectory'
  | 'notDirectlyAdmissible'
  | 'duplicateOfAdmittedSource'
  | 'unavailable'
  | 'unsupportedPlatform'
  | 'rejected'

export type LocalBrowserChildFailureCode =
  | 'unsupportedPlatform'
  | 'rootPathUnavailable'
  | 'parentPathUnavailable'
  | 'parentMissing'
  | 'parentNotDirectory'
  | 'parentOutsideRoot'
  | 'permissionDenied'
  | 'metadataUnavailable'
  | 'enumerationUnavailable'
  | 'reparsePointSkipped'
  | 'rejectedRoot'
  | 'unknownFileType'

export type ReadLocalBrowserChildrenRequest = {
  readonly entryPointKind: LocalBrowserEntryPointKind
  readonly rootCanonicalPath: string
  readonly parentCanonicalPath: string
  readonly offset: number
  readonly limit: number
}

export type LocalBrowserChildWindowIdentity = {
  readonly entryPointKind: LocalBrowserEntryPointKind
  readonly rootCanonicalPath: string
  readonly parentCanonicalPath: string
}

export type LocalBrowserCandidateIdentity = {
  readonly entryPointKind: LocalBrowserEntryPointKind
  readonly rootCanonicalPath: string
  readonly candidateCanonicalPath: string
}

export type LocalBrowserCandidateAffordances = {
  readonly canBrowse: boolean
  readonly canRequestAdmission: boolean
  readonly canChooseDescendant: boolean
  readonly canRequestParentAdmission: boolean
  readonly requiresConfirmation: boolean
}

export type LocalBrowserChildFailure = {
  readonly code: LocalBrowserChildFailureCode
  readonly detail: string
}

export type LocalBrowserChildRow = {
  readonly identity: LocalBrowserCandidateIdentity
  readonly rowKind: LocalBrowserCandidateRowKind
  readonly displayName: string
  readonly status: LocalBrowserCandidateStatus
  readonly platform: LocalBrowserEntryPointPlatform
  readonly fileKind: ContentsFileKind | null
  readonly mediaRelevance: LocalBrowserCandidateMediaRelevance | null
  readonly admissionHint: LocalBrowserCandidateAdmissionHint
  readonly affordances: LocalBrowserCandidateAffordances
  readonly failure: LocalBrowserChildFailure | null
}

export type ReadLocalBrowserChildrenResult = {
  readonly state: 'read'
  readonly status: LocalBrowserChildrenReadStatus
  readonly windowIdentity: LocalBrowserChildWindowIdentity
  readonly offset: number
  readonly limit: number
  readonly totalRows: number
  readonly rows: readonly LocalBrowserChildRow[]
  readonly failure: LocalBrowserChildFailure | null
}

export type ReadLocalBrowserChildrenErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadLocalBrowserChildrenErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadLocalBrowserChildrenError = {
  readonly code: ReadLocalBrowserChildrenErrorCode
  readonly message: string
}

export type ReadLocalBrowserChildrenOutcome =
  | ReadLocalBrowserChildrenResult
  | {
      readonly state: ReadLocalBrowserChildrenErrorState
      readonly error: ReadLocalBrowserChildrenError
    }

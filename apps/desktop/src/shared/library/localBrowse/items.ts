import type { ContentsFileKind } from '../contents/read'
import type {
  LocalBrowseEntryPointKind,
  LocalBrowseOperation,
  LocalBrowsePlatform
} from './entryPoints'

export type LocalBrowseItemsReadStatus =
  | 'complete'
  | 'partialFailure'
  | 'failed'
  | 'unsupportedPlatform'
  | 'missing'
  | 'permissionBlocked'
  | 'unavailable'

export type LocalBrowseItemKind =
  | 'directory'
  | 'mediaFile'
  | 'unsupportedFile'
  | 'rejectedRoot'
  | 'inaccessible'
  | 'unknown'

export type LocalBrowseItemStatus =
  | 'available'
  | 'unavailable'
  | 'permissionBlocked'
  | 'missing'
  | 'unsupportedPlatform'
  | 'duplicateOfAdmittedSource'
  | 'rejected'
  | 'unknown'

export type LocalBrowseItemMediaRelevance =
  | 'mediaRelevant'
  | 'companionMetadata'
  | 'unsupported'
  | 'unknown'

export type LocalBrowseProfile = 'audioBrowse' | 'mediaBrowse' | 'allFiles'

export type LocalBrowseItemFailureCode =
  | 'unsupportedPlatform'
  | 'rootIdentityMismatch'
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

export type ReadLocalBrowseItemsRequest = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly resolvedRootPath: string
  readonly resolvedParentPath: string
  readonly profile: LocalBrowseProfile
  readonly offset: number
  readonly limit: number
}

export type LocalBrowseWindowIdentity = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly resolvedRootPath: string
  readonly resolvedParentPath: string
}

export type LocalBrowseItemIdentity = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly resolvedRootPath: string
  readonly resolvedItemPath: string
}

export type LocalBrowseItemFailure = {
  readonly code: LocalBrowseItemFailureCode
  readonly detail: string
}

export type LocalBrowseItem = {
  readonly identity: LocalBrowseItemIdentity
  readonly itemKind: LocalBrowseItemKind
  readonly displayName: string
  readonly status: LocalBrowseItemStatus
  readonly platform: LocalBrowsePlatform
  readonly fileKind: ContentsFileKind | null
  readonly mediaRelevance: LocalBrowseItemMediaRelevance | null
  readonly availableOperations: readonly LocalBrowseOperation[]
  readonly failure: LocalBrowseItemFailure | null
}

export type ReadLocalBrowseItemsResult = {
  readonly state: 'read'
  readonly status: LocalBrowseItemsReadStatus
  readonly windowIdentity: LocalBrowseWindowIdentity
  readonly offset: number
  readonly limit: number
  readonly totalItems: number
  readonly items: readonly LocalBrowseItem[]
  readonly failure: LocalBrowseItemFailure | null
}

export type ReadLocalBrowseItemsErrorState =
  | 'hostUnavailable'
  | 'hostFailed'
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'readFailed'

export type ReadLocalBrowseItemsErrorCode =
  | 'hostNotStarted'
  | 'hostStopping'
  | 'hostStopped'
  | 'hostFailed'
  | 'readFailed'

export type ReadLocalBrowseItemsError = {
  readonly code: ReadLocalBrowseItemsErrorCode
  readonly message: string
}

export type ReadLocalBrowseItemsOutcome =
  | ReadLocalBrowseItemsResult
  | {
      readonly state: ReadLocalBrowseItemsErrorState
      readonly error: ReadLocalBrowseItemsError
    }

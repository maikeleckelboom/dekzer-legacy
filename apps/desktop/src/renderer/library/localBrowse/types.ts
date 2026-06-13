import type {
  LocalBrowseEntryPoint,
  LocalBrowseEntryPointKind,
  ReadLocalBrowseEntryPointsResult
} from '../../../shared/library/localBrowse/entryPoints'
import type {
  LocalBrowseItem,
  LocalBrowseWindowIdentity,
  ReadLocalBrowseItemsResult
} from '../../../shared/library/localBrowse/items'

export type LocalBrowseEntryPointsState =
  | {
      readonly kind: 'unread'
    }
  | {
      readonly kind: 'loading'
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'ready'
      readonly result: ReadLocalBrowseEntryPointsResult
      readonly refreshError?: string
    }
  | {
      readonly kind: 'refreshing'
      readonly result: ReadLocalBrowseEntryPointsResult
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
    }

export type LocalBrowseEntryPointTarget = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly rootCanonicalPath: string
  readonly label: string
}

export type LocalBrowseDirectoryTarget = {
  readonly entryPointKind: LocalBrowseEntryPointKind
  readonly rootCanonicalPath: string
  readonly parentCanonicalPath: string
  readonly label: string
}

export type LocalBrowseMoreTarget = LocalBrowseDirectoryTarget & {
  readonly ownerNodeId: string
  readonly offset: number
  readonly limit: number
}

export type LocalBrowseMoreState =
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
    }

export type LoadedLocalBrowseItems = {
  readonly identity: LocalBrowseWindowIdentity
  readonly label: string
  readonly items: readonly LocalBrowseItem[]
  readonly totalItems: number
  readonly status: ReadLocalBrowseItemsResult['status']
  readonly failure: ReadLocalBrowseItemsResult['failure']
  readonly nextOffset?: number
  readonly limit: number
  readonly more?: LocalBrowseMoreState
}

export type LocalBrowseItemState =
  | {
      readonly kind: 'loading'
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'loaded'
      readonly window: LoadedLocalBrowseItems
    }
  | {
      readonly kind: 'refreshing'
      readonly window: LoadedLocalBrowseItems
      readonly requestKey: string
      readonly sequence: number
      readonly detail?: string
    }
  | {
      readonly kind: 'failed'
      readonly detail: string
      readonly errorCode: string
    }

export function targetForEntryPoint(
  entry: LocalBrowseEntryPoint
): LocalBrowseEntryPointTarget | undefined {
  const canonicalPath = entry.identity.canonicalPath

  if (canonicalPath === null || canonicalPath.trim().length === 0) {
    return undefined
  }

  return {
    entryPointKind: entry.identity.entryPointKind,
    rootCanonicalPath: canonicalPath,
    label: entry.displayName
  }
}

export function localBrowseRootTarget(
  target: LocalBrowseEntryPointTarget
): LocalBrowseDirectoryTarget {
  return {
    entryPointKind: target.entryPointKind,
    rootCanonicalPath: target.rootCanonicalPath,
    parentCanonicalPath: target.rootCanonicalPath,
    label: target.label
  }
}

export function localBrowseWindowKey(target: LocalBrowseDirectoryTarget): string {
  return [
    target.entryPointKind,
    encodeURIComponent(target.rootCanonicalPath),
    encodeURIComponent(target.parentCanonicalPath)
  ].join(':')
}

export function localBrowseWindowKeyFromIdentity(identity: LocalBrowseWindowIdentity): string {
  return localBrowseWindowKey({
    entryPointKind: identity.entryPointKind,
    rootCanonicalPath: identity.rootCanonicalPath,
    parentCanonicalPath: identity.parentCanonicalPath,
    label: ''
  })
}

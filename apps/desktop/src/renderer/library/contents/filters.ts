import type { ContentsReadPolicy, ContentsScopeDepth } from '../../../shared/library/contents/read'

export type BuiltInContentsFilterId =
  | 'audio'
  | 'video'
  | 'media'
  | 'companionFiles'
  | 'allSourceFiles'

export type BuiltInContentsFilter = {
  readonly id: BuiltInContentsFilterId
  readonly label: string
  readonly policy: ContentsReadPolicy
  readonly scopeDepth: ContentsScopeDepth
}

export const defaultContentsFilterId: BuiltInContentsFilterId = 'audio'

const builtInContentsFilters = {
  audio: {
    id: 'audio',
    label: 'Audio',
    policy: { kind: 'audioBrowse' },
    scopeDepth: 'recursive'
  },
  video: {
    id: 'video',
    label: 'Video',
    policy: { kind: 'primaryMedia', mediaKinds: ['video'] },
    scopeDepth: 'recursive'
  },
  media: {
    id: 'media',
    label: 'Media',
    policy: { kind: 'playableMediaBrowse' },
    scopeDepth: 'recursive'
  },
  companionFiles: {
    id: 'companionFiles',
    label: 'Companion Files',
    policy: { kind: 'sourceFileInventory', fileClasses: ['unsupported'] },
    scopeDepth: 'recursive'
  },
  allSourceFiles: {
    id: 'allSourceFiles',
    label: 'All Files',
    policy: {
      kind: 'sourceFileInventory',
      fileClasses: ['audio', 'video', 'image', 'unsupported']
    },
    scopeDepth: 'recursive'
  }
} as const satisfies Record<BuiltInContentsFilterId, BuiltInContentsFilter>

export function builtInContentsFilter(filterId: BuiltInContentsFilterId): BuiltInContentsFilter {
  return builtInContentsFilters[filterId]
}

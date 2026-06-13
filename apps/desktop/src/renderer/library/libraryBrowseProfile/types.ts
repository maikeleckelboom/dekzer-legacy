import type { ContentsReadPolicy } from '../../../shared/library/contents/read'

export type LibraryBrowseProfile = 'audio' | 'playable' | 'allFiles'

export type LibraryBrowseProfileOption = {
  readonly key: LibraryBrowseProfile
  readonly label: string
}

export const defaultLibraryBrowseProfile: LibraryBrowseProfile = 'audio'

export const libraryBrowseProfileOptions: readonly LibraryBrowseProfileOption[] = [
  { key: 'audio', label: 'Audio' },
  { key: 'playable', label: 'Audio + Video' },
  { key: 'allFiles', label: 'All Files' }
]

export function isLibraryBrowseProfile(value: unknown): value is LibraryBrowseProfile {
  return value === 'audio' || value === 'playable' || value === 'allFiles'
}

export function libraryBrowseProfileLabel(profile: LibraryBrowseProfile): string {
  return libraryBrowseProfileOptions.find((option) => option.key === profile)?.label ?? 'Audio'
}

export function mapLibraryBrowseProfileToContentsPolicy(
  profile: LibraryBrowseProfile
): ContentsReadPolicy {
  switch (profile) {
    case 'audio':
      return { kind: 'audioBrowse' }
    case 'playable':
      return { kind: 'playableMediaBrowse' }
    case 'allFiles':
      return {
        kind: 'sourceFileInventory',
        fileClasses: ['audio', 'video', 'image', 'unsupported']
      }
  }
}

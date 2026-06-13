import type { ContentsReadPolicy } from '../../../shared/library/contents/read'

export type ProfileKey = 'audio' | 'playable' | 'allFiles'

export type ProfileOption = {
  readonly key: ProfileKey
  readonly label: string
}

export const defaultProfile: ProfileKey = 'audio'

export const profileOptions: readonly ProfileOption[] = [
  { key: 'audio', label: 'Audio' },
  { key: 'playable', label: 'Audio + Video' },
  { key: 'allFiles', label: 'All Files' }
]

export function isProfileKey(value: unknown): value is ProfileKey {
  return value === 'audio' || value === 'playable' || value === 'allFiles'
}

export function profileLabel(profile: ProfileKey): string {
  return profileOptions.find((option) => option.key === profile)?.label ?? 'Audio'
}

export function mapProfileToContentsPolicy(profile: ProfileKey): ContentsReadPolicy {
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

export function emptyStateLabel(profile: ProfileKey): string {
  switch (profile) {
    case 'audio':
      return 'No audio tracks'
    case 'playable':
      return 'No playable media'
    case 'allFiles':
      return 'No files'
  }
}

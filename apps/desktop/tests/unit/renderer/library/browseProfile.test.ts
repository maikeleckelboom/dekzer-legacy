import { describe, expect, it } from 'vitest'

import { createProfileController } from '../../../../src/renderer/library/browseProfile/controller'
import {
  emptyStateLabel,
  mapProfileToContentsPolicy,
  mapProfileToLocalBrowseProfile,
  profileLabel,
  profileOptions
} from '../../../../src/renderer/library/browseProfile/types'

describe('browse profile', () => {
  it('defaults to audio and switches to playable and allFiles', () => {
    const controller = createProfileController()

    expect(controller.profile.value).toBe('audio')
    expect(controller.setProfile('playable')).toBe(true)
    expect(controller.profile.value).toBe('playable')
    expect(controller.setProfile('allFiles')).toBe(true)
    expect(controller.profile.value).toBe('allFiles')
    expect(controller.setProfile('allFiles')).toBe(false)
  })

  it('exposes compact user-facing labels', () => {
    expect(profileOptions.map((option) => option.label)).toEqual([
      'Audio',
      'Audio + Video',
      'All Files'
    ])
    expect(profileLabel('audio')).toBe('Audio')
    expect(profileLabel('playable')).toBe('Audio + Video')
    expect(profileLabel('allFiles')).toBe('All Files')
  })

  it('maps local browse and registered contents policies consistently', () => {
    expect(mapProfileToLocalBrowseProfile('audio')).toBe('audio')
    expect(mapProfileToLocalBrowseProfile('playable')).toBe('playable')
    expect(mapProfileToLocalBrowseProfile('allFiles')).toBe('allFiles')

    expect(mapProfileToContentsPolicy('audio')).toEqual({ kind: 'audioBrowse' })
    expect(mapProfileToContentsPolicy('playable')).toEqual({ kind: 'playableMediaBrowse' })
    expect(mapProfileToContentsPolicy('allFiles')).toEqual({
      kind: 'sourceFileInventory',
      fileClasses: ['audio', 'video', 'image', 'unsupported']
    })
  })

  it('maps empty-state copy by profile', () => {
    expect(emptyStateLabel('audio')).toBe('No audio tracks')
    expect(emptyStateLabel('playable')).toBe('No playable media')
    expect(emptyStateLabel('allFiles')).toBe('No files')
  })
})

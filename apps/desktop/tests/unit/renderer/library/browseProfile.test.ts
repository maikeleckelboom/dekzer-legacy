import { describe, expect, it } from 'vitest'

import { createProfileController } from '../../../../src/renderer/library/browseProfile/controller'
import {
  emptyStateLabel,
  mapProfileToContentsPolicy,
  profileLabel,
  profileOptions
} from '../../../../src/renderer/library/browseProfile/types'
import {
  createLocalPreviewModeController,
  defaultLocalPreviewMode,
  localPreviewModeLabel,
  localPreviewModeOptions,
  localPreviewSurfaceLabel,
  mapLocalPreviewModeToLocalBrowseProfile
} from '../../../../src/renderer/library/localBrowse/previewMode'

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

  it('maps registered contents policies by library browse profile', () => {
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

describe('local preview mode', () => {
  it('defaults to music evidence and keeps advanced inventory explicit', () => {
    const controller = createLocalPreviewModeController()

    expect(defaultLocalPreviewMode).toBe('musicEvidence')
    expect(controller.mode.value).toBe('musicEvidence')
    expect(controller.setMode('advancedInventory')).toBe(true)
    expect(controller.mode.value).toBe('advancedInventory')
    expect(controller.setMode('advancedInventory')).toBe(false)
  })

  it('uses Add Source language and maps to local browse profile only at the boundary', () => {
    expect(localPreviewModeOptions.map((option) => option.label)).toEqual([
      'Music Evidence',
      'Advanced Inventory'
    ])
    expect(localPreviewModeLabel('musicEvidence')).toBe('Music Evidence')
    expect(localPreviewModeLabel('advancedInventory')).toBe('Advanced Inventory')
    expect(localPreviewSurfaceLabel('musicEvidence')).toBe('Source Preview')
    expect(localPreviewSurfaceLabel('advancedInventory')).toBe('Source Inventory')
    expect(mapLocalPreviewModeToLocalBrowseProfile('musicEvidence')).toBe('audio')
    expect(mapLocalPreviewModeToLocalBrowseProfile('advancedInventory')).toBe('allFiles')
  })
})

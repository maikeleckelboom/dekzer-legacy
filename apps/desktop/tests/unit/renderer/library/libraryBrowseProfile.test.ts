import { describe, expect, it } from 'vitest'

import { createLibraryBrowseProfileController } from '../../../../src/renderer/library/libraryBrowseProfile/controller'
import {
  libraryBrowseEmptyStateLabel,
  mapLibraryBrowseProfileToContentsPolicy,
  libraryBrowseProfileLabel,
  libraryBrowseProfileOptions
} from '../../../../src/renderer/library/libraryBrowseProfile/types'
import {
  createLocalPreviewModeController,
  defaultLocalPreviewMode,
  localPreviewModeLabel,
  localPreviewModeOptions,
  localPreviewSurfaceLabel,
  mapLocalPreviewModeToLocalBrowseItemFilter
} from '../../../../src/renderer/library/localBrowse/previewMode'

describe('library browse profile', () => {
  it('defaults to audio and switches to playable and allFiles', () => {
    const controller = createLibraryBrowseProfileController()

    expect(controller.profile.value).toBe('audio')
    expect(controller.setProfile('playable')).toBe(true)
    expect(controller.profile.value).toBe('playable')
    expect(controller.setProfile('allFiles')).toBe(true)
    expect(controller.profile.value).toBe('allFiles')
    expect(controller.setProfile('allFiles')).toBe(false)
  })

  it('exposes compact user-facing labels', () => {
    expect(libraryBrowseProfileOptions.map((option) => option.label)).toEqual([
      'Audio',
      'Audio + Video',
      'All Files'
    ])
    expect(libraryBrowseProfileLabel('audio')).toBe('Audio')
    expect(libraryBrowseProfileLabel('playable')).toBe('Audio + Video')
    expect(libraryBrowseProfileLabel('allFiles')).toBe('All Files')
  })

  it('maps registered contents policies by library browse profile', () => {
    expect(mapLibraryBrowseProfileToContentsPolicy('audio')).toEqual({ kind: 'audioBrowse' })
    expect(mapLibraryBrowseProfileToContentsPolicy('playable')).toEqual({
      kind: 'playableMediaBrowse'
    })
    expect(mapLibraryBrowseProfileToContentsPolicy('allFiles')).toEqual({
      kind: 'sourceFileInventory',
      fileClasses: ['audio', 'video', 'image', 'unsupported']
    })
  })

  it('maps empty-state copy by profile', () => {
    expect(libraryBrowseEmptyStateLabel('audio')).toBe('No audio tracks')
    expect(libraryBrowseEmptyStateLabel('playable')).toBe('No playable media')
    expect(libraryBrowseEmptyStateLabel('allFiles')).toBe('No files')
  })
})

describe('local preview mode', () => {
  it('defaults to Music Preview and keeps Inventory explicit', () => {
    const controller = createLocalPreviewModeController()

    expect(defaultLocalPreviewMode).toBe('musicEvidence')
    expect(controller.mode.value).toBe('musicEvidence')
    expect(controller.setMode('advancedInventory')).toBe(true)
    expect(controller.mode.value).toBe('advancedInventory')
    expect(controller.setMode('advancedInventory')).toBe(false)
  })

  it('uses Add Source language and maps to local browse item filter only at the boundary', () => {
    expect(localPreviewModeOptions.map((option) => option.label)).toEqual([
      'Music Preview',
      'Inventory'
    ])
    expect(localPreviewModeLabel('musicEvidence')).toBe('Music Preview')
    expect(localPreviewModeLabel('advancedInventory')).toBe('Inventory')
    expect(localPreviewSurfaceLabel('musicEvidence')).toBe('Source Preview')
    expect(localPreviewSurfaceLabel('advancedInventory')).toBe('Source Inventory')
    expect(mapLocalPreviewModeToLocalBrowseItemFilter('musicEvidence')).toBe('audio')
    expect(mapLocalPreviewModeToLocalBrowseItemFilter('advancedInventory')).toBe('allFiles')
  })
})

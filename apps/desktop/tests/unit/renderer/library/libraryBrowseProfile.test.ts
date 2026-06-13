import { describe, expect, it } from 'vitest'

import { createLibraryBrowseProfileController } from '../../../../src/renderer/library/libraryBrowseProfile/controller'
import {
  libraryBrowseEmptyStateLabel,
  mapLibraryBrowseProfileToContentsPolicy,
  libraryBrowseProfileLabel,
  libraryBrowseProfileOptions
} from '../../../../src/renderer/library/libraryBrowseProfile/types'
import {
  createAddSourceViewController,
  defaultAddSourceView,
  addSourceViewLabel,
  addSourceViewOptions,
  addSourceSurfaceLabel,
  mapAddSourceViewToLocalBrowseItemFilter
} from '../../../../src/renderer/library/addSource/view'

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

describe('Add Source view', () => {
  it('defaults to Preview and keeps Inventory explicit', () => {
    const controller = createAddSourceViewController()

    expect(defaultAddSourceView).toBe('preview')
    expect(controller.view.value).toBe('preview')
    expect(controller.setView('inventory')).toBe(true)
    expect(controller.view.value).toBe('inventory')
    expect(controller.setView('inventory')).toBe(false)
  })

  it('uses Add Source language and maps to local browse item filter only at the boundary', () => {
    expect(addSourceViewOptions.map((option) => option.label)).toEqual(['Preview', 'Inventory'])
    expect(addSourceViewLabel('preview')).toBe('Preview')
    expect(addSourceViewLabel('inventory')).toBe('Inventory')
    expect(addSourceSurfaceLabel('preview')).toBe('Preview')
    expect(addSourceSurfaceLabel('inventory')).toBe('Inventory')
    expect(mapAddSourceViewToLocalBrowseItemFilter('preview')).toBe('audio')
    expect(mapAddSourceViewToLocalBrowseItemFilter('inventory')).toBe('allFiles')
  })
})

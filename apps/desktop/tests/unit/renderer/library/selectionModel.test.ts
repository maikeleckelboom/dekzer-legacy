import { describe, expect, it } from 'vitest'

import {
  clearSelection,
  isValidSelection,
  rowSubject,
  sourceSubject
} from '../../../../src/renderer/library/selection/model'
import type {
  ContentProjection,
  ContentRow
} from '../../../../src/renderer/library/contents/projection'
import type { StatusContext } from '../../../../src/renderer/library/sourceStatus/context'

describe('library primary selection model', () => {
  it('selects only rows with inspectable subjects', () => {
    const selected = rowSubject(row('row-1'))
    const empty = rowSubject({ id: 'more', kind: 'more', label: 'More' })

    expect(selected).toMatchObject({
      kind: 'row',
      rowId: 'row-1',
      subject: {
        kind: 'sourceFile',
        sourceId: '7',
        sourceFileId: 'file-1'
      }
    })
    expect(empty).toEqual(clearSelection())
  })

  it('keeps row selections valid only while the same subject is projected', () => {
    const selected = rowSubject(row('row-1'))

    expect(isValidSelection(selected, projection([row('row-1')]))).toBe(true)
    expect(isValidSelection(selected, projection([row('row-2')]))).toBe(false)
    expect(
      isValidSelection(
        selected,
        projection([
          {
            ...row('row-1'),
            subject: {
              kind: 'sourceFile',
              sourceId: '7',
              sourceFileId: 'file-2',
              label: 'other.wav',
              presence: 'present'
            }
          }
        ])
      )
    ).toBe(false)
  })

  it('does not promote registered tree files to primary source or track selection', () => {
    const fileContext: StatusContext = {
      kind: 'registeredFile',
      title: 'track.wav',
      sourceId: '7',
      fileId: 'file-1',
      nodeId: 'source-file:file-1'
    }

    expect(sourceSubject(fileContext)).toEqual(clearSelection())
  })
})

function row(id: string): ContentRow {
  return {
    id,
    kind: 'file',
    label: 'track.wav',
    subject: {
      kind: 'sourceFile',
      sourceId: '7',
      sourceFileId: 'file-1',
      label: 'track.wav',
      presence: 'present'
    }
  }
}

function projection(rows: readonly ContentRow[]): ContentProjection {
  return {
    surfaceKind: 'indexedContents',
    surfaceLabel: 'Contents',
    header: {
      surfaceLabel: 'Library Browse',
      scopeLabel: 'Source Fixture',
      health: { label: 'Ready', tone: 'ready' }
    },
    kind: 'ready',
    title: 'Source Fixture',
    rows
  }
}

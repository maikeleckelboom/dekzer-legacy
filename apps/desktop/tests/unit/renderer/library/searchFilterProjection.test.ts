import { describe, expect, it } from 'vitest'

import {
  projectSearchFilterResultRow,
  projectSearchFilterResultRows
} from '../../../../src/renderer/library/searchFilter/projection'
import type { SearchFilterResultRow } from '../../../../src/shared/library/searchFilter/read'

describe('search/filter projection', () => {
  it('15. preserves result kind and authority layer from protocol row', () => {
    const projected = projectSearchFilterResultRow(row('source', 'source'))

    expect(projected).toMatchObject({
      resultKind: 'source',
      authorityLayer: 'source',
      label: 'Source'
    })
  })

  it('16. projects source-file results as source-file results', () => {
    const projected = projectSearchFilterResultRow(row('sourceFile', 'sourceFileInventory'))

    expect(projected).toMatchObject({
      resultKind: 'sourceFile',
      authorityLayer: 'sourceFileInventory',
      iconRole: 'sourceFile'
    })
    expect(projected).not.toMatchObject({
      iconRole: 'music'
    })
  })

  it('17. does not apply renderer-side membership filtering', () => {
    const rows = [
      row('source', 'source'),
      row('directory', 'sourceHierarchy'),
      row('sourceFile', 'sourceFileInventory')
    ]

    expect(projectSearchFilterResultRows(rows)).toHaveLength(rows.length)
  })
})

function row(
  resultKind: SearchFilterResultRow['resultKind'],
  authorityLayer: SearchFilterResultRow['authorityLayer']
): SearchFilterResultRow {
  const sourceFileFields =
    resultKind === 'sourceFile'
      ? {
          fileClass: 'audio' as const,
          fileKind: 'audio' as const,
          mediaRelevance: 'audioWorkflow' as const,
          presenceState: 'present' as const
        }
      : {}

  return {
    resultKind,
    authorityLayer,
    stableKey: `${resultKind}:1`,
    sourceId: '1',
    sourceLocationId: resultKind === 'sourceLocation' ? '2' : null,
    sourceDirectoryId: resultKind === 'directory' ? '3' : null,
    parentSourceDirectoryId: null,
    sourceFileId: resultKind === 'sourceFile' ? '4' : null,
    displayLabel: resultKind === 'source' ? 'Source' : 'Result',
    displayPath: 'Library/Result',
    relativePath: 'Result',
    ...sourceFileFields,
    sourceAccessState: 'accessible',
    sourceScanPhase: 'complete',
    hasCurrentBlake3: resultKind === 'sourceFile',
    hasCurrentProbe: false,
    attachmentLinkState: 'notApplicable',
    attachmentId: null,
    evidenceCoverageState: 'indexed',
    matchReason: 'filter',
    updatedAtMs: 100
  }
}

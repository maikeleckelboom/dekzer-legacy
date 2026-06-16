import { describe, expect, it } from 'vitest'

import { projectColumns, type Column } from '../../../../src/renderer/library/contents/columnModel'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'

describe('contents column model', () => {
  it('starts from an empty Library column when no browser projection is available', () => {
    expect(projectColumns({ projection: undefined })).toEqual({
      columns: [{ id: 'column:library', title: 'Library', rows: [] }]
    })
  })

  it('projects loaded tree children without inventing hierarchy', () => {
    const projection = browserProjection([
      node('source:1', 'Music', [node('dir:10', 'Albums'), node('dir:20', 'Singles')])
    ])

    const columns = projectColumns({
      projection,
      selectedNodeId: 'source:1'
    }).columns

    expect(columnLabels(columns)).toEqual([['Music'], ['Albums', 'Singles']])
  })

  it('does not create child columns for unloaded children or from visible contents rows', () => {
    const projection = browserProjection([
      {
        ...node('source:1', 'Music'),
        children: { kind: 'unmaterialized' }
      }
    ])

    const columns = projectColumns({
      projection,
      selectedNodeId: 'source:1'
    }).columns

    expect(columnLabels(columns)).toEqual([['Music']])
  })

  it('activates rows only when backed by an existing binding identity', () => {
    const projection = browserProjection([node('source:1', 'Music'), node('source:2', 'Missing')], {
      omitBindingIds: new Set(['source:2'])
    })

    const rows = projectColumns({ projection }).columns[0]?.rows ?? []

    expect(rows.map((row) => ({ id: row.id, canActivate: row.canActivate }))).toEqual([
      { id: 'source:1', canActivate: true },
      { id: 'source:2', canActivate: false }
    ])
  })
})

function columnLabels(columns: readonly Column[]): readonly (readonly string[])[] {
  return columns.map((column) => column.rows.map((row) => row.label))
}

function browserProjection(
  nodes: readonly BrowserTreeNode[],
  options: { readonly omitBindingIds?: ReadonlySet<string> } = {}
): BrowserProjection {
  const bindingsById = new Map<string, RowBinding>()

  for (const current of flattenNodes(nodes)) {
    if (options.omitBindingIds?.has(current.id)) {
      continue
    }

    bindingsById.set(current.id, binding(current.id))
  }

  return {
    kind: 'tree',
    nodes,
    bindingsById
  }
}

function flattenNodes(nodes: readonly BrowserTreeNode[]): readonly BrowserTreeNode[] {
  const found: BrowserTreeNode[] = []

  for (const current of nodes) {
    found.push(current)

    if (current.children.kind === 'loaded') {
      found.push(...flattenNodes(current.children.nodes))
    }
  }

  return found
}

function node(
  id: string,
  label: string,
  children: readonly BrowserTreeNode[] = []
): BrowserTreeNode {
  return {
    id,
    label,
    role: id.startsWith('source:') ? 'source' : 'literalDirectory',
    icon: id.startsWith('source:') ? 'source' : 'folder',
    children: children.length === 0 ? { kind: 'none' } : { kind: 'loaded', nodes: children }
  }
}

function binding(id: string): RowBinding {
  if (id.startsWith('source:')) {
    return {
      kind: 'source',
      navigationRow: {
        navigationRowId: id,
        stableKey: id,
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: id,
        siblingPosition: 0,
        selectable: true,
        selectorKind: 'source',
        selectorPayload: '1',
        updatedAtMs: 100,
        rowVersion: '1'
      },
      target: {
        navigationRowId: id,
        entryPoint: { kind: 'source', sourceId: '1' },
        label: id
      }
    }
  }

  return {
    kind: 'directory',
    sourceId: '1',
    directoryId: id,
    entryPoint: { kind: 'source', sourceId: '1' },
    label: id
  }
}

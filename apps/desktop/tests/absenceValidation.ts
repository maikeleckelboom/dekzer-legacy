import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { relative } from 'node:path'

import { desktopRoot, sourceRoot, listSourceFiles, normalizePath } from './support/files'

type AllowedNullUse = {
  readonly reason: string
  readonly patterns: readonly RegExp[]
}

const allowedNullUses: ReadonlyMap<string, AllowedNullUse> = new Map([
  [
    'src/shared/libraryBoundary/status.ts',
    {
      reason: 'renderer-visible host status contract',
      patterns: [/readonly lastError: LibraryBoundaryHostStatusError \| null/]
    }
  ],
  [
    'src/main/libraryBoundary/status.ts',
    {
      reason: 'renderer-visible host status contract normalization',
      patterns: [/lastError: lastError \?\? null/]
    }
  ],
  [
    'src/main/libraryHierarchy/mapping.ts',
    {
      reason: 'generated hierarchy row compatibility',
      patterns: [
        /row.sourceDirectoryId === null/,
        /row.sourceFileId === null/,
        /sourceDirectoryId === null/,
        /sourceFileId === null/,
        /row.parentSourceDirectoryId === null/
      ]
    }
  ],
  [
    'src/main/libraryHierarchy/request.ts',
    {
      reason: 'unknown IPC input normalization',
      patterns: [
        /value === null \|\| value === undefined/,
        /typeof value === 'object' && value !== null/
      ]
    }
  ],
  [
    'src/main/libraryHierarchy/target.ts',
    {
      reason: 'generated navigation protocol compatibility',
      patterns: [/parentNavigationRowId: null/, /sourceRow.selectorPayload === null/]
    }
  ],
  [
    'src/main/libraryHierarchy/readChildren.ts',
    {
      reason: 'generated hierarchy protocol compatibility',
      patterns: [
        /parentSourceDirectoryId: normalizedRequest.parentSourceDirectoryId \?\? null/,
        /if \(reply.window === null\)/,
        /\.\.\.\(reply.window.parentSourceDirectoryId === null/
      ]
    }
  ],
  [
    'src/renderer/libraryBrowser/tree/context.ts',
    {
      reason: 'Vue DOM element ref compatibility',
      patterns: [/element: HTMLElement \| null/]
    }
  ],
  [
    'src/renderer/libraryBrowser/tree/controller.ts',
    {
      reason: 'Vue DOM element ref compatibility',
      patterns: [/element: HTMLElement \| null/, /element !== null/]
    }
  ],
  [
    'src/renderer/libraryBrowser/tree/treeItem.vue',
    {
      reason: 'Vue template ref and DOM API compatibility',
      patterns: [
        /ref<HTMLElement \| null>\(null\)/,
        /registerItemElement\(props.item.id, null\)/,
        /closest\('\[data-tree-affordance="true"\]'\) !== null/
      ]
    }
  ]
])

void main()

function main(): void {
  validatesNoCasualSourceNull()
}

function validatesNoCasualSourceNull(): void {
  const violations: string[] = []

  for (const filePath of listSourceFiles(sourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const allowedUse = allowedNullUses.get(relativePath)
    const lines = readFileSync(filePath, 'utf8').split(/\r?\n/)

    lines.forEach((line, index) => {
      if (!/\bnull\b/.test(line)) {
        return
      }

      if (allowedUse?.patterns.some((pattern) => pattern.test(line)) === true) {
        return
      }

      violations.push(`${relativePath}:${index + 1}: ${line.trim()}`)
    })
  }

  assert.deepEqual(violations, [], formatViolations(violations))
}
function formatViolations(violations: readonly string[]): string {
  return [
    'Unexpected app-owned null usage in desktop source.',
    'Use undefined/omitted properties for app-owned absence, or add a narrow boundary allowance.',
    ...violations
  ].join('\n')
}

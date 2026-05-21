import { strict as assert } from 'node:assert'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

type AllowedNullUse = {
  readonly reason: string
  readonly patterns: readonly RegExp[]
}

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const sourceRoot = join(desktopRoot, 'src')
const workspaceRoot = resolve(desktopRoot, '..', '..')

const allowedNullUses: ReadonlyMap<string, AllowedNullUse> = new Map([
  [
    'src/shared/libraryBoundaryStatus.ts',
    {
      reason: 'renderer-visible host status contract',
      patterns: [/readonly lastError: LibraryBoundaryHostStatusError \| null/]
    }
  ],
  [
    'src/main/libraryBoundaryHost.ts',
    {
      reason: 'Electron resources option compatibility',
      patterns: [/readonly resourcesPath\?: string \| null/]
    }
  ],
  [
    'src/main/libraryBoundaryHostConfig.ts',
    {
      reason: 'Electron resources path compatibility',
      patterns: [
        /readonly resourceRoot: string \| null/,
        /readonly resourcesPath\?: string \| null/,
        /function defaultElectronResourcesPath\(\): string \| null/,
        /return typeof resourcesPath === 'string' && resourcesPath.length > 0 \? resourcesPath : null/
      ]
    }
  ],
  [
    'src/main/libraryBoundaryHostErrors.ts',
    {
      reason: 'Electron resources path diagnostic compatibility',
      patterns: [/readonly resourceRoot\?: string \| null/]
    }
  ],
  [
    'src/main/libraryBoundaryHostStatus.ts',
    {
      reason: 'renderer-visible host status contract normalization',
      patterns: [/lastError: lastError \?\? null/]
    }
  ],
  [
    'src/main/libraryHierarchyRead.ts',
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
    'src/main/libraryHierarchyReadMapping.ts',
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
    'src/main/libraryHierarchyReadRequest.ts',
    {
      reason: 'unknown IPC input normalization',
      patterns: [
        /value === null \|\| value === undefined/,
        /typeof value === 'object' && value !== null/
      ]
    }
  ],
  [
    'src/main/libraryHierarchyReadTarget.ts',
    {
      reason: 'generated navigation protocol compatibility',
      patterns: [/parentNavigationRowId: null/, /sourceRow.selectorPayload === null/]
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
  validatesAbsenceValidationIsInChecks()
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

function validatesAbsenceValidationIsInChecks(): void {
  const desktopPackage = readJson(join(desktopRoot, 'package.json'))
  const rootPackage = readJson(join(workspaceRoot, 'package.json'))

  assert.equal(desktopPackage.scripts?.['validate:absence'], 'tsx tests/absenceValidation.ts')
  assert.equal(
    rootPackage.scripts?.['desktop:validate:absence'],
    'pnpm --filter @dekzer/desktop run validate:absence'
  )
  assert.match(rootPackage.scripts?.check ?? '', /pnpm run desktop:validate:absence/)
}

function listSourceFiles(root: string): readonly string[] {
  const files: string[] = []

  for (const entry of readdirSync(root)) {
    const entryPath = join(root, entry)
    const stats = statSync(entryPath)

    if (stats.isDirectory()) {
      files.push(...listSourceFiles(entryPath))
      continue
    }

    if (entryPath.endsWith('.ts') || entryPath.endsWith('.vue')) {
      files.push(entryPath)
    }
  }

  return files
}

function readJson(filePath: string): {
  readonly scripts?: Record<string, string>
} {
  return JSON.parse(readFileSync(filePath, 'utf8')) as {
    readonly scripts?: Record<string, string>
  }
}

function normalizePath(path: string): string {
  return path.split(sep).join('/')
}

function formatViolations(violations: readonly string[]): string {
  return [
    'Unexpected app-owned null usage in desktop source.',
    'Use undefined/omitted properties for app-owned absence, or add a narrow boundary allowance.',
    ...violations
  ].join('\n')
}

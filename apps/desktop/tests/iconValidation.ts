import { strict as assert } from 'node:assert'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const sourceRoot = join(desktopRoot, 'src')
const workspaceRoot = resolve(desktopRoot, '..', '..')
const allowedIconDirectory = normalizePath(
  relative(desktopRoot, join(sourceRoot, 'renderer', 'icons'))
)
const lucideImportPattern = /from\s+['"]@lucide\/vue['"]/

void main()

function main(): void {
  validatesNoDirectLucideImportOutsideIconVocabulary()
  validatesIconValidationIsInChecks()
}

function validatesNoDirectLucideImportOutsideIconVocabulary(): void {
  const violations: string[] = []

  for (const filePath of listSourceFiles(sourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))

    if (relativePath.startsWith(allowedIconDirectory)) {
      continue
    }

    const contents = readFileSync(filePath, 'utf8')

    if (lucideImportPattern.test(contents)) {
      violations.push(relativePath)
    }
  }

  assert.deepEqual(violations, [], formatViolations(violations))
}

function validatesIconValidationIsInChecks(): void {
  const desktopPackage = readJson(join(desktopRoot, 'package.json'))
  const rootPackage = readJson(join(workspaceRoot, 'package.json'))

  assert.equal(desktopPackage.scripts?.['validate:icons'], 'tsx tests/iconValidation.ts')
  assert.equal(
    rootPackage.scripts?.['desktop:validate:icons'],
    'pnpm --filter @dekzer/desktop run validate:icons'
  )
  assert.match(rootPackage.scripts?.check ?? '', /pnpm run desktop:validate:icons/)
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
    'Direct @lucide/vue import outside the allowed icon vocabulary directory.',
    `Only files under ${allowedIconDirectory}/ may import from @lucide/vue.`,
    'Import or re-export from src/renderer/icons instead.',
    ...violations
  ].join('\n')
}

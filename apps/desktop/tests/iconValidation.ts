import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

import { desktopRoot, sourceRoot, listSourceFiles, normalizePath } from './support/files'

const iconDirPrefix = `${normalizePath(
  relative(desktopRoot, join(sourceRoot, 'renderer', 'icons'))
)}/`
const lucideImportPattern = /from\s+['"]@lucide\/vue['"]/

void main()

function main(): void {
  validatesNoDirectLucideImportOutsideIconVocabulary()
}

function validatesNoDirectLucideImportOutsideIconVocabulary(): void {
  const violations: string[] = []

  for (const filePath of listSourceFiles(sourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))

    if (relativePath.startsWith(iconDirPrefix)) {
      continue
    }

    const contents = readFileSync(filePath, 'utf8')

    if (lucideImportPattern.test(contents)) {
      violations.push(relativePath)
    }
  }

  assert.deepEqual(violations, [], formatViolations(violations))
}
function formatViolations(violations: readonly string[]): string {
  return [
    'Direct @lucide/vue import outside the allowed icon vocabulary directory.',
    `Only files under ${iconDirPrefix} may import from @lucide/vue.`,
    'Import or re-export from src/renderer/icons instead.',
    ...violations
  ].join('\n')
}

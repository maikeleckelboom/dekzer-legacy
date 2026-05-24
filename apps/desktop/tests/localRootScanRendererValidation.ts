import { strict as assert } from 'node:assert'
import { readFileSync } from 'node:fs'
import { relative } from 'node:path'

import { desktopRoot, listSourceFiles, normalizePath, rendererSourceRoot } from './support/files'

const approvedRunScanRendererOwner = 'src/renderer/library/boundary/localRootActions.ts'

validatesRendererScanBoundaryOwnership()

function validatesRendererScanBoundaryOwnership(): void {
  const violations: string[] = []
  const forbiddenPatterns = [
    /@dekzer\/library-boundary-client/,
    /@dekzer\/library-boundary-stdio-transport/,
    /\bLibraryBoundaryClient\b/,
    /\bipcRenderer\b/,
    /from ['"]electron['"]/,
    /from ['"]node:fs['"]/,
    /from ['"]fs['"]/,
    /from ['"]node:path['"]/,
    /from ['"]path['"]/,
    /from ['"].*\/main\//,
    /\bshowOpenDialog\b/,
    /\brunRootScan\b/
  ]
  for (const filePath of listSourceFiles(rendererSourceRoot)) {
    const relativePath = normalizePath(relative(desktopRoot, filePath))
    const contents = readFileSync(filePath, 'utf8')

    for (const pattern of forbiddenPatterns) {
      if (pattern.test(contents)) {
        violations.push(`${relativePath}: ${String(pattern)}`)
      }
    }

    if (/\.runScan\(/.test(contents) && relativePath !== approvedRunScanRendererOwner) {
      violations.push(
        `${relativePath}: .runScan is only allowed in ${approvedRunScanRendererOwner}`
      )
    }
  }

  assert.deepEqual(violations, [])
}

import { spawnSync } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const validations = [
  'tests/browserEntryPresentationValidation.ts',
  'tests/hierarchyReadValidation.ts',
  'tests/hostValidation.ts',
  'tests/libraryBrowserViewStateValidation.ts',
  'tests/viewStateValidation.ts',
  'tests/libraryStorageEnvironmentValidation.ts',
  'tests/localRootChoiceValidation.ts',
  'tests/localLibraryRestartValidation.ts',
  'tests/localRootRegistrationValidation.ts',
  'tests/localRootScanValidation.ts',
  'tests/localRootScanRendererValidation.ts',
  'tests/locationSourcesValidation.ts',
  'tests/preloadValidation.ts',
  'tests/treeValidation.ts'
] as const

for (const validation of validations) {
  console.log(`\n> desktop validation: ${validation}`)

  const result = spawnSync(`pnpm exec tsx ${validation}`, {
    cwd: desktopRoot,
    stdio: 'inherit',
    shell: true
  })

  if (result.error !== undefined) {
    throw result.error
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}

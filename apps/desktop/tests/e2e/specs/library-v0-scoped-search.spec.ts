import { test } from '../fixtures/libraryFixture'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import {
  libraryV0ScopedSearchSourceA,
  libraryV0ScopedSearchSourceB
} from '../support/domain/LibraryContracts'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['scoped-search']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('searches within the selected source or folder only', async ({
    electronApp,
    libraryFilesystem,
    libraryV0
  }, testInfo) => {
    const sourceA = libraryV0ScopedSearchSourceA(libraryFilesystem.sourceA.rootPath)
    const sourceB = libraryV0ScopedSearchSourceB(libraryFilesystem.sourceB.rootPath)
    const run = createLibraryV0AcceptanceRun({
      scenario,
      testInfo,
      electronApp,
      page: () => currentElectronPage(electronApp),
      fixturePaths: libraryV0FixtureDiagnosticPaths(libraryFilesystem)
    })

    await run.run(async () => {
      await run.step('launch:ready', async () => {
        await libraryV0.expectLaunchReady()
      })
      const admittedSourceA = await run.step('product-action:admit-source-a', async () =>
        libraryV0.admitAndScanMusicFolder(sourceA)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source A',
          rootPath: admittedSourceA.rootPath,
          sourceId: admittedSourceA.sourceId,
          expectedRows: ['Root Track A.wav', 'Nested Track A.wav', 'Descendant Only A.wav']
        })
      )
      const admittedSourceB = await run.step('product-action:admit-source-b', async () =>
        libraryV0.admitAndScanMusicFolder(sourceB)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source B',
          rootPath: admittedSourceB.rootPath,
          sourceId: admittedSourceB.sourceId,
          expectedRows: ['Only Track B.wav']
        })
      )

      await run.step('assertion:scoped-search', async () => {
        await libraryV0.expectScopedSearchRespectsActiveBrowseScope(
          admittedSourceA,
          admittedSourceB
        )
      })
    })
  })
})

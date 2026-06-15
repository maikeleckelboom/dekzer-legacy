import { test } from '../fixtures/libraryFixture'
import { libraryV0GoldenSource } from '../support/domain/LibraryContracts'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['restart-persistence']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('persists an admitted source and provides an honest reselectable state after restart', async ({
    electronApp,
    libraryFilesystem,
    libraryV0
  }, testInfo) => {
    const sourceA = libraryV0GoldenSource(libraryFilesystem.sourceA.rootPath)
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

      const admittedSource = await run.step('product-action:add-and-scan-source-a', async () =>
        libraryV0.admitAndScanMusicFolder(sourceA)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source A',
          rootPath: admittedSource.rootPath,
          sourceId: admittedSource.sourceId,
          expectedRows: ['Root Track A.wav', 'Nested Track A.wav', 'Descendant Only A.wav']
        })
      )

      await run.step('product-action:select-nested-folder', async () => {
        await libraryV0.expectNestedFolderContents(admittedSource)
      })

      await run.step('product-action:switch-profile-all-files', async () => {
        await libraryV0.selectAllFilesProfile()
        await libraryV0.expectAllFilesProfileSelected()
      })

      await run.step('teardown:close-before-restart', async () => {
        await libraryV0.closeGracefully()
      })

      await run.step('launch:relaunch-same-user-data', async () => {
        await libraryV0.relaunch()
        await libraryV0.expectLibraryReady()
      })

      const persistedSource = await run.step(
        'assertion:source-persists-without-readmission',
        async () => libraryV0.expectAdmittedSourcePresent(sourceA)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source A',
          rootPath: persistedSource.rootPath,
          sourceId: persistedSource.sourceId,
          expectedRows: ['Root Track A.wav', 'Nested Track A.wav', 'Descendant Only A.wav']
        })
      )

      await run.step('assertion:restart-fallback-is-honest', async () => {
        await libraryV0.expectSingleAdmittedPath(persistedSource)
        await libraryV0.expectRemovedSourceContentsInactive(persistedSource)
        await libraryV0.selectSource(persistedSource)
        await libraryV0.expectSourceReady(persistedSource)
        await libraryV0.expectNestedFolderContents(persistedSource)
      })
    })
  })
})

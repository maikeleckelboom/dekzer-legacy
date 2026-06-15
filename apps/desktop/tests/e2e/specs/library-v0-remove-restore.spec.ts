import { test } from '../fixtures/libraryFixture'
import { libraryV0GoldenSource } from '../support/domain/LibraryContracts'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['remove-readd-freshness']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('removes and re-adds the same source path without stale active truth or duplicates', async ({
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

      await run.step('assertion:initial-source-ready', async () => {
        await libraryV0.expectSourceReady(admittedSource)
      })

      await run.step('product-action:remove-source-a', async () => {
        await libraryV0.removeSource(admittedSource)
      })

      await run.step('assertion:removed-source-inactive', async () => {
        await libraryV0.expectRemovedSourceContentsInactive(admittedSource)
      })

      const readdedSource = await run.step('product-action:readd-same-path', async () =>
        libraryV0.admitMusicFolder(sourceA)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source A re-added',
          rootPath: readdedSource.rootPath,
          sourceId: readdedSource.sourceId,
          expectedRows: ['Root Track A.wav', 'Nested Track A.wav', 'Descendant Only A.wav']
        })
      )

      await run.step('assertion:readd-reaches-fresh-ready-state', async () => {
        await libraryV0.expectSourceReady(readdedSource)
      })

      await run.step('teardown:close-before-duplicate-check', async () => {
        await libraryV0.closeGracefully()
      })

      await run.step('launch:relaunch-after-readd', async () => {
        await libraryV0.relaunch()
        await libraryV0.expectLibraryReady()
      })

      await run.step('assertion:no-duplicate-same-path-source', async () => {
        const persistedSource = await libraryV0.expectAdmittedSourcePresent(sourceA)
        await libraryV0.expectSingleAdmittedPath(persistedSource)
      })
    })
  })
})

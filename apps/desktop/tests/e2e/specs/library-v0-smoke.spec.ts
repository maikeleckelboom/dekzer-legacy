import { test } from '../fixtures/libraryFixture'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import { libraryV0GoldenSource } from '../support/domain/LibraryContracts'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['golden-smoke']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('admits, scans, browses, removes, and does not restore a source after relaunch', async ({
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

      const admittedSource = await run.step('product-action:admit-source-a', async () =>
        libraryV0.admitMusicFolder(sourceA)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Source A',
          rootPath: admittedSource.rootPath,
          sourceId: admittedSource.sourceId,
          expectedRows: ['Root Track A.wav', 'Nested Track A.wav', 'Descendant Only A.wav']
        })
      )

      await run.step('assertion:source-ready', async () => {
        await libraryV0.expectSourceReady(admittedSource)
      })
      await run.step('assertion:persisted-hierarchy-disclosure', async () => {
        await libraryV0.expectPersistedHierarchyDisclosure(admittedSource)
      })
      await run.step('assertion:folder-browse', async () => {
        await libraryV0.expectFolderScopedBrowsing(admittedSource)
      })
      await run.step('assertion:profile-filtering', async () => {
        await libraryV0.expectBrowseProfileFiltering(admittedSource)
      })
      await run.step('assertion:empty-folder', async () => {
        await libraryV0.expectEmptyFolderState(admittedSource)
      })

      await run.step('product-action:remove-source', async () => {
        await libraryV0.removeSource(admittedSource)
      })
      await run.step('assertion:removed-after-relaunch', async () => {
        await libraryV0.expectSourceAbsentAfterRelaunch(admittedSource)
      })
    })
  })
})

import { test } from '../fixtures/libraryFixture'
import { libraryV0GoldenSource } from '../support/domain/LibraryContracts'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['missing-source-health']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('marks a missing admitted source unhealthy and recovers after restore', async ({
    electronApp,
    libraryFilesystem,
    libraryV0
  }, testInfo) => {
    const sourceA = libraryV0GoldenSource(libraryFilesystem.sourceA.rootPath)
    let offlineSourceA: string | undefined
    const run = createLibraryV0AcceptanceRun({
      scenario,
      testInfo,
      electronApp,
      page: () => currentElectronPage(electronApp),
      fixturePaths: libraryV0FixtureDiagnosticPaths(libraryFilesystem)
    })

    try {
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

        await run.step('teardown:close-before-offline-simulation', async () => {
          await libraryV0.closeGracefully()
        })

        await run.step('fixture-setup:move-source-a-offline', async () => {
          offlineSourceA = await libraryFilesystem.moveSourceOffline(
            libraryFilesystem.sourceA.rootPath
          )
        })

        await run.step('launch:relaunch-with-missing-source', async () => {
          await libraryV0.relaunch()
          await libraryV0.expectLibraryReady()
        })

        const persistedSource = await run.step(
          'assertion:source-remains-known-while-missing',
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

        await run.step('assertion:missing-source-health-is-honest', async () => {
          await libraryV0.expectSourceHealthUnavailable(persistedSource)
          await libraryV0.expectUnavailableSourceDoesNotShowFalseCompleteRows(persistedSource)
        })

        await run.step('fixture-setup:restore-source-a', async () => {
          if (offlineSourceA === undefined) {
            throw new Error('Offline source path was not captured.')
          }

          await libraryFilesystem.restoreOfflineSource(
            libraryFilesystem.sourceA.rootPath,
            offlineSourceA
          )
          offlineSourceA = undefined
        })

        await run.step('product-action:refresh-and-rescan-restored-source', async () => {
          await libraryV0.refreshSelectedSourceStatus()
          await libraryV0.runSelectedSourceScanIfAvailable()
        })

        await run.step('assertion:restored-source-recovers', async () => {
          await libraryV0.expectSourceReady(persistedSource)
        })
      })
    } finally {
      if (offlineSourceA !== undefined) {
        await libraryFilesystem
          .restoreOfflineSource(libraryFilesystem.sourceA.rootPath, offlineSourceA)
          .catch(() => undefined)
      }
    }
  })
})

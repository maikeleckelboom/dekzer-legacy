import { test } from '../fixtures/libraryFixture'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0DiagnosticSource,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import type { LibraryV0Source } from '../support/domain/LibraryContracts'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios['contents-pagination']

test.describe(libraryV0ScenarioTitle(scenario), () => {
  test('appends the second contents page on the first Load More click', async ({
    electronApp,
    libraryFilesystem,
    libraryV0,
    libraryPanel
  }, testInfo) => {
    const pagedFixture = await libraryFilesystem.createPagedAudioSource('paged-source', 120)
    const pagedSource: LibraryV0Source = {
      rootPath: pagedFixture.rootPath,
      sourceName: /paged-source\b/i,
      audioRows: [/Track 001\.wav/]
    }
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

      const admittedSource = await run.step('product-action:admit-paged-source', async () =>
        libraryV0.admitMusicFolder(pagedSource)
      )
      run.addAdmittedSource(
        libraryV0DiagnosticSource({
          label: 'Paged Source',
          rootPath: admittedSource.rootPath,
          sourceId: admittedSource.sourceId,
          expectedRows: ['Track 001.wav', 'Track 101.wav']
        })
      )

      await run.step('assertion:source-ready', async () => {
        await libraryV0.expectSourceReady(admittedSource)
      })

      await run.step('assertion:load-more-first-click', async () => {
        await libraryPanel.contents.expectRowsVisible([/Track 001\.wav/])
        await libraryPanel.contents.expectRowsHidden([/Track 101\.wav/])
        await libraryPanel.contents.loadMoreOnce()
        await libraryPanel.contents.expectRowsVisible([/Track 101\.wav/])
      })
    })
  })
})

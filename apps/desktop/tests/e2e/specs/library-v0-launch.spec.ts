import { expect, test } from '../fixtures/libraryFixture'
import {
  createLibraryV0AcceptanceRun,
  currentElectronPage,
  libraryV0FixtureDiagnosticPaths
} from '../support/domain/LibraryDiagnostics'
import { libraryV0Scenarios, libraryV0ScenarioTitle } from '../support/scenarios/libraryV0Scenarios'

const scenario = libraryV0Scenarios.launch

test.describe(libraryV0ScenarioTitle(scenario, 'Library V0 launch smoke'), () => {
  test('launches the built app with isolated user data and reaches Add Source', async ({
    electronApp,
    libraryFilesystem,
    libraryV0
  }, testInfo) => {
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
        await libraryV0.openAddSource()
      })

      await run.step('teardown:close', async () => {
        await expect(electronApp.close()).resolves.toMatchObject({
          graceful: true,
          killed: false,
          timedOut: false
        })
      })
    })
  })
})

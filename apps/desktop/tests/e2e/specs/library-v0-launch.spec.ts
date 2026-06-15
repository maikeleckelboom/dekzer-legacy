import { test } from '../fixtures/libraryFixture'
import { expectContentsPanelVisible } from '../assertions/contentsAssertions'
import {
  expectAddSourceEntryReachable,
  expectAddSourceSurfaceVisible,
  expectLibrarySurfaceVisible
} from '../assertions/libraryAssertions'

test.describe('Library V0 launch smoke', () => {
  test('launches the built app with isolated user data and reaches Add Source', async ({
    contentsPanel,
    electronApp,
    libraryPanel
  }) => {
    await expectLibrarySurfaceVisible(libraryPanel)
    await expectAddSourceEntryReachable(libraryPanel)
    await expectContentsPanelVisible(contentsPanel)

    await libraryPanel.openAddSourceIfNeeded()
    await expectAddSourceSurfaceVisible(libraryPanel)

    await electronApp.close()
  })
})

import { test } from '../fixtures/libraryFixture'
import { libraryV0GoldenSource } from '../support/domain/LibraryContracts'

test.describe('Library V0 golden smoke', () => {
  test('admits, scans, browses, removes, and does not restore a source after relaunch', async ({
    libraryFilesystem,
    libraryV0
  }) => {
    const sourceA = libraryV0GoldenSource(libraryFilesystem.sourceA.rootPath)

    await libraryV0.expectLaunchReady()
    const admittedSource = await libraryV0.admitMusicFolder(sourceA)

    await libraryV0.expectSourceReady(admittedSource)
    await libraryV0.expectFolderScopedBrowsing(admittedSource)
    await libraryV0.expectBrowseProfileFiltering(admittedSource)
    await libraryV0.expectEmptyFolderState(admittedSource)

    await libraryV0.removeSource(admittedSource)
    await libraryV0.expectSourceAbsentAfterRelaunch(admittedSource)
  })
})

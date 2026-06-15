import { test } from '../fixtures/libraryFixture'
import {
  libraryV0ScopedSearchSourceA,
  libraryV0ScopedSearchSourceB
} from '../support/domain/LibraryContracts'

test.describe('Library V0 scoped search', () => {
  test('searches within the selected source or folder only', async ({
    libraryFilesystem,
    libraryV0
  }) => {
    const sourceA = libraryV0ScopedSearchSourceA(libraryFilesystem.sourceA.rootPath)
    const sourceB = libraryV0ScopedSearchSourceB(libraryFilesystem.sourceB.rootPath)

    await libraryV0.expectLaunchReady()
    const admittedSourceA = await libraryV0.admitAndScanMusicFolder(sourceA)
    const admittedSourceB = await libraryV0.admitAndScanMusicFolder(sourceB)

    await libraryV0.expectScopedSearchRespectsActiveBrowseScope(admittedSourceA, admittedSourceB)
  })
})

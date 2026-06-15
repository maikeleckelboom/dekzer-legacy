import { expect, test } from '../fixtures/libraryFixture'
import {
  expectAuthoritativeEmptyContents,
  expectContentRowsHidden,
  expectContentRowsVisible,
  expectContentsPanelVisible
} from '../assertions/contentsAssertions'
import {
  expectAddSourceEntryReachable,
  expectAddSourceSurfaceVisible,
  expectAdmittedSourceHidden,
  expectAdmittedSourceVisible,
  expectLibrarySurfaceVisible
} from '../assertions/libraryAssertions'
import {
  expectReadyStatusHasVisibleEvidence,
  expectSourceStatusVisible
} from '../assertions/sourceStatusAssertions'
import { LibraryPanel } from '../pageObjects/LibraryPanel'

const sourceName = /source-a\b/i
const rootTrack = /Root Track A\.wav/
const nestedTrack = /Nested Track A\.wav/
const descendantOnlyTrack = /Descendant Only A\.wav/
const coverImage = /cover\.jpg/
const readmeText = /readme\.txt/
const sourceAudioRows = [rootTrack, nestedTrack, descendantOnlyTrack]

test.describe('Library V0 golden smoke', () => {
  test('admits, scans, browses, removes, and does not restore a source after relaunch', async ({
    contentsPanel,
    dialogs,
    electronApp,
    libraryFilesystem,
    libraryPanel,
    libraryTree,
    sourceStatusPanel
  }) => {
    const sourceA = libraryFilesystem.sourceA

    await expectLibrarySurfaceVisible(libraryPanel)
    await expectAddSourceEntryReachable(libraryPanel)
    await expectContentsPanelVisible(contentsPanel)

    await dialogs.selectDirectory(sourceA.rootPath)
    await libraryPanel.openAddSourceIfNeeded()
    await expectAddSourceSurfaceVisible(libraryPanel)
    await libraryPanel.addMusicFolder()

    const sourceId = await libraryPanel.waitForAdmittedPath(sourceA.rootPath)
    await libraryPanel.openLibraryBrowseIfNeeded()
    await expectAdmittedSourceVisible(libraryPanel, sourceName)

    await libraryTree.select(sourceName)
    await expectSourceStatusVisible(sourceStatusPanel)
    await expectReadyStatusHasVisibleEvidence(sourceStatusPanel, contentsPanel)

    await sourceStatusPanel.runScanIfAvailable()
    await libraryPanel.waitForTerminalSourceReadiness(sourceId)
    await expect(sourceStatusPanel.root).not.toContainText(
      /Scanning source|Indexing|Still indexing|Needs scan/
    )

    await expectContentRowsVisible(contentsPanel, sourceAudioRows)

    await libraryTree.expand(sourceName)
    await libraryTree.select(/nested\b/i)
    await expectContentRowsVisible(contentsPanel, [nestedTrack])

    await libraryTree.select(/descendants-only\b/i)
    await expectContentRowsVisible(contentsPanel, [descendantOnlyTrack])
    await expectContentRowsHidden(contentsPanel, [rootTrack, nestedTrack])

    await libraryTree.select(sourceName)
    await libraryPanel.selectBrowseProfile('Audio + Video')
    await expectContentRowsVisible(contentsPanel, sourceAudioRows)
    await expectContentRowsHidden(contentsPanel, [coverImage, readmeText])

    await libraryPanel.selectBrowseProfile('All Files')
    await expectContentRowsVisible(contentsPanel, [...sourceAudioRows, coverImage, readmeText])

    await libraryTree.expand(sourceName)
    await libraryTree.select(/^empty\b/i)
    await expectAuthoritativeEmptyContents(contentsPanel)

    await libraryPanel.selectBrowseProfile('Audio')
    await libraryTree.select(sourceName)
    await expectContentRowsVisible(contentsPanel, sourceAudioRows)
    await expectContentRowsHidden(contentsPanel, [coverImage, readmeText])

    await sourceStatusPanel.removeSourceWithConfirmation()
    await expectAddSourceSurfaceVisible(libraryPanel)
    await expectContentRowsVisible(contentsPanel, [/Suggested folders/])
    await expect.poll(() => libraryPanel.sourceIdForAdmittedPath(sourceA.rootPath)).toBeUndefined()

    await libraryPanel.openLibraryBrowseIfNeeded()
    await expectAdmittedSourceHidden(libraryPanel, sourceName)

    const relaunchedLibraryPanel = new LibraryPanel(await electronApp.relaunch())
    await expectLibrarySurfaceVisible(relaunchedLibraryPanel)
    await expectAddSourceEntryReachable(relaunchedLibraryPanel)
    await expect
      .poll(() => relaunchedLibraryPanel.sourceIdForAdmittedPath(sourceA.rootPath))
      .toBeUndefined()
    await expectAdmittedSourceHidden(relaunchedLibraryPanel, sourceName)
  })
})

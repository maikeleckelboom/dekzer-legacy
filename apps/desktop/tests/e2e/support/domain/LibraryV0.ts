import type { Page } from '@playwright/test'

import type { DialogHelpers } from '../../fixtures/dialogs.fixture'
import type { ElectronAppHarness } from '../../fixtures/electronApp.fixture'
import {
  type LibraryV0AdmittedSource,
  type LibraryV0GoldenSource,
  type LibraryV0ScopedSearchSourceA,
  type LibraryV0ScopedSearchSourceB,
  type LibraryV0Source
} from './LibraryContracts'
import { LibraryRuntimeProbe } from './LibraryRuntimeProbe'
import { LibraryPanel } from '../screens/LibraryPanel'

export type LibraryV0Options = {
  readonly page: Page
  readonly dialogs: DialogHelpers
  readonly electronApp: ElectronAppHarness
}

export class LibraryV0 {
  private panel: LibraryPanel
  private probe: LibraryRuntimeProbe

  constructor(private readonly options: LibraryV0Options) {
    this.panel = new LibraryPanel(options.page)
    this.probe = new LibraryRuntimeProbe(options.page)
  }

  async expectLaunchReady(): Promise<void> {
    await this.probe.waitForPreloadApi()
    await this.panel.expectVisible()
    await this.panel.expectAddSourceEntryReachable()
    await this.panel.contents.expectVisible()
  }

  async openAddSource(): Promise<void> {
    await this.panel.openAddSourceIfNeeded()
    await this.panel.expectAddSourceSurfaceVisible()
  }

  async admitMusicFolder<Source extends LibraryV0Source>(
    source: Source
  ): Promise<LibraryV0AdmittedSource<Source>> {
    await this.options.dialogs.selectDirectory(source.rootPath)
    await this.openAddSource()
    await this.panel.addMusicFolder()

    const sourceId = await this.probe.waitForAdmittedPath(source.rootPath)
    await this.panel.openLibraryBrowseIfNeeded()
    await this.panel.browse.expectSourceVisible(source.sourceName)

    return { ...source, sourceId }
  }

  async expectSourceReady(source: LibraryV0AdmittedSource<LibraryV0Source>): Promise<void> {
    await this.panel.browse.select(source.sourceName)
    await this.panel.sourceStatus.expectVisible()
    await this.panel.sourceStatus.expectReadyHasVisibleEvidence(this.panel.contents)
    await this.panel.sourceStatus.runScanIfAvailable()
    await this.probe.waitForTerminalSourceReadiness(source.sourceId)
    await this.panel.sourceStatus.expectNoActiveScanCopy()
    await this.panel.contents.expectRowsVisible(source.audioRows)
  }

  async admitAndScanMusicFolder<Source extends LibraryV0Source>(
    source: Source
  ): Promise<LibraryV0AdmittedSource<Source>> {
    const admittedSource = await this.admitMusicFolder(source)

    await this.expectSourceReady(admittedSource)

    return admittedSource
  }

  async expectScopedSearchRespectsActiveBrowseScope(
    sourceA: LibraryV0AdmittedSource<LibraryV0ScopedSearchSourceA>,
    sourceB: LibraryV0AdmittedSource<LibraryV0ScopedSearchSourceB>
  ): Promise<void> {
    await this.panel.browse.select(sourceA.sourceName)

    await this.panel.search.searchFor('Only Track B')
    await this.panel.contents.expectSearchEmpty('source')
    await this.panel.contents.expectRowsHidden([sourceB.sourceBTrack])

    await this.panel.search.searchFor('Root Track A')
    await this.panel.contents.expectRowsVisible([sourceA.rootTrack])

    await this.panel.browse.expand(sourceA.sourceName)
    await this.panel.browse.select(sourceA.nestedFolder)

    await this.panel.search.searchFor('Descendant Only A')
    await this.panel.contents.expectSearchEmpty('folder')
    await this.panel.contents.expectRowsHidden([sourceA.descendantOnlyTrack])

    await this.panel.search.searchFor('Nested Track A')
    await this.panel.contents.expectRowsVisible([sourceA.nestedTrack])

    await this.panel.search.clear()
    await this.panel.contents.expectTitle(sourceA.nestedFolder)
    await this.panel.contents.expectRowsVisible([sourceA.nestedTrack])
  }

  async expectFolderScopedBrowsing(source: LibraryV0GoldenSource): Promise<void> {
    await this.panel.browse.expand(source.sourceName)

    await this.panel.browse.select(/nested\b/i)
    await this.panel.contents.expectRowsVisible([source.nestedTrack])

    await this.panel.browse.select(/descendants-only\b/i)
    await this.panel.contents.expectRowsVisible([source.descendantOnlyTrack])
    await this.panel.contents.expectRowsHidden([source.rootTrack, source.nestedTrack])
  }

  async expectBrowseProfileFiltering(source: LibraryV0GoldenSource): Promise<void> {
    await this.panel.browse.select(source.sourceName)

    await this.panel.selectBrowseProfile('Audio + Video')
    await this.panel.contents.expectRowsVisible(source.audioRows)
    await this.panel.contents.expectRowsHidden([source.coverImage, source.readmeText])

    await this.panel.selectBrowseProfile('All Files')
    await this.panel.contents.expectRowsVisible([
      ...source.audioRows,
      source.coverImage,
      source.readmeText
    ])
  }

  async expectEmptyFolderState(source: LibraryV0GoldenSource): Promise<void> {
    await this.panel.browse.expand(source.sourceName)
    await this.panel.browse.select(/^empty\b/i)
    await this.panel.contents.expectAuthoritativeEmpty()

    await this.panel.selectBrowseProfile('Audio')
    await this.panel.browse.select(source.sourceName)
    await this.panel.contents.expectRowsVisible(source.audioRows)
    await this.panel.contents.expectRowsHidden([source.coverImage, source.readmeText])
  }

  async removeSource(source: LibraryV0GoldenSource): Promise<void> {
    await this.panel.sourceStatus.removeSourceWithConfirmation()
    await this.probe.waitForSourcePathForgotten(source.rootPath)

    await this.panel.openAddSourceIfNeeded()
    await this.panel.expectAddSourceSurfaceVisible()
    await this.panel.contents.expectRowsVisible([/Suggested folders/])

    await this.panel.openLibraryBrowseIfNeeded()
    await this.panel.browse.expectSourceHidden(source.sourceName)
  }

  async expectSourceAbsentAfterRelaunch(source: LibraryV0GoldenSource): Promise<void> {
    this.bindPage(await this.options.electronApp.relaunch())
    await this.expectLaunchReady()
    await this.probe.waitForSourcePathForgotten(source.rootPath)
    await this.panel.openLibraryBrowseIfNeeded()
    await this.panel.browse.expectSourceHidden(source.sourceName)
  }

  private bindPage(page: Page): void {
    this.panel = new LibraryPanel(page)
    this.probe = new LibraryRuntimeProbe(page)
  }
}

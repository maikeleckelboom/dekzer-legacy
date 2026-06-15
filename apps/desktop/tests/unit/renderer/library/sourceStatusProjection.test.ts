import { describe, expect, it } from 'vitest'

import { createLibraryBrowseProfileController } from '../../../../src/renderer/library/libraryBrowseProfile/controller'
import type { StatusContext } from '../../../../src/renderer/library/sourceStatus/context'
import { projectStatusContext } from '../../../../src/renderer/library/sourceStatus/context'
import {
  projectStatusView,
  sourceStatusDiagnosticTrace,
  type StatusViewInput
} from '../../../../src/renderer/library/sourceStatus/projection'
import type { BrowserProjection } from '../../../../src/renderer/library/tree/projection'
import type { BrowserTreeNode } from '../../../../src/renderer/library/tree/types'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type {
  LocalBrowseEntryPoint,
  LocalBrowseOperation
} from '../../../../src/shared/library/localBrowse/entryPoints'
import type { LocalBrowseItem } from '../../../../src/shared/library/localBrowse/items'
import type { SourceLifecycleRecord } from '../../../../src/shared/library/source/lifecycle'
import type { LibraryBrowseProfile } from '../../../../src/renderer/library/libraryBrowseProfile/types'
import type {
  ReadSourceActivityReply,
  ReadSourceIntegrityReply,
  ReadSourceMaintenanceReply
} from '@dekzer/library-boundary-contract'

describe('source status projection', () => {
  it('no selected row shows no source status actions', () => {
    const view = statusView(
      projectStatusContext({ projection: sourceProjection(), selectedTitle: 'Library contents' })
    )

    expect(view.role).toBe('none')
    expect(view.actions).toEqual([])
  })

  it('local browse folder with admission shows Add as music source only', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseEntryPoint',
        entry: localEntry([admission('selectedDirectory', 'C:/Music')]),
        target: {
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          label: 'Music'
        }
      },
      'Music'
    )
    const context = projectStatusContext({
      projection,
      selectedNodeId: 'selected',
      selectedTitle: 'Music'
    })
    const view = statusView(context)

    expect(view.badge).toBe('Ready to add')
    expect(view.actions).toEqual([
      expect.objectContaining({ kind: 'addLocalPath', label: 'Add as music source' })
    ])
    expect(view.actions).toHaveLength(1)
    expect(view.actions.some((action) => action.kind === 'scanSource')).toBe(false)
    expect(view.actions.some((action) => action.kind === 'runMaintenance')).toBe(false)
  })

  it('Add Source root shows guidance without source-specific actions or unknown badge', () => {
    const projection = localBrowseProjection({ kind: 'addSourceSection' }, 'Add Source')
    const view = statusView(
      projectStatusContext({
        projection,
        selectedNodeId: 'selected',
        selectedTitle: 'Local browse'
      })
    )

    expect(view.role).toBe('navigation')
    expect(view.badge).toBeUndefined()
    expect(view.detail).toBe('Choose a folder to add as a music source.')
    expect(view.actions).toEqual([])
  })

  it('local browse media file with parent admission shows Add parent as music source only', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: localItem([admission('parentDirectory', 'C:/Music/Album')], 'mediaFile')
      },
      'track.flac'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'track.flac' })
    )

    expect(view.actions).toEqual([
      expect.objectContaining({ kind: 'addLocalPath', label: 'Add parent as music source' })
    ])
    expect(view.actions.some((action) => action.kind === 'scanSource')).toBe(false)
    expect(view.actions.some((action) => action.kind === 'runMaintenance')).toBe(false)
  })

  it('local browse folders without admission avoid hostile copy', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: localItem([{ kind: 'browseChildren' }, { kind: 'chooseDescendant' }], 'directory'),
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Empty',
          label: 'Empty'
        }
      },
      'Empty'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'Empty' })
    )

    expect(view.badge).toBe('Choose a music folder')
    expect(view.detail).toBe('Choose a folder that contains music files.')
    expect(`${view.badge} ${view.detail}`).not.toContain('not a music-source candidate')
    expect(view.actions).toEqual([])
  })

  it('duplicate local browse row without matched source identity has no add or show-source action', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: {
          ...localItem([], 'directory'),
          status: 'duplicateOfAdmittedSource'
        },
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Duplicate',
          label: 'Duplicate'
        }
      },
      'Duplicate'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'Duplicate' })
    )

    expect(view.actions).toEqual([])
  })

  it('duplicate local browse row with matched source identity shows Show source only', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: {
          ...localItem([], 'directory'),
          status: 'duplicateOfAdmittedSource',
          matchedSourceId: '7'
        },
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Duplicate',
          label: 'Duplicate'
        }
      },
      'Duplicate'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'Duplicate' })
    )

    expect(view.badge).toBe('Already added')
    expect(view.actions).toEqual([
      expect.objectContaining({ kind: 'showSource', label: 'Show source', sourceId: '7' })
    ])
    expect(view.actions.some((action) => action.kind === 'addLocalPath')).toBe(false)
  })

  it('restorable local browse row offers restore instead of active duplicate actions', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: {
          ...localItem([admission('selectedDirectory', 'C:/Music/Removed')], 'directory'),
          status: 'restorableSource',
          matchedSourceId: '7'
        },
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Removed',
          label: 'Removed'
        }
      },
      'Removed'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'Removed' })
    )

    expect(view.badge).toBe('Restore source')
    expect(view.actions).toEqual([
      expect.objectContaining({
        kind: 'addLocalPath',
        label: 'Restore source',
        resolvedPath: 'C:/Music/Removed'
      })
    ])
    expect(view.actions.some((action) => action.kind === 'showSource')).toBe(false)
  })

  it('does not offer broad system root admission from a loose media file', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: {
          ...localItem([admission('parentDirectory', 'C:/')], 'mediaFile'),
          identity: {
            entryPointKind: 'systemDriveRoot',
            resolvedRootPath: 'C:/',
            resolvedItemPath: 'C:/loose.flac'
          }
        }
      },
      'loose.flac'
    )
    const view = statusView(
      projectStatusContext({ projection, selectedNodeId: 'selected', selectedTitle: 'loose.flac' })
    )

    expect(view.badge).toBe('Choose a specific folder')
    expect(view.detail).toBe('Choose a specific folder inside this drive.')
    expect(view.actions).toEqual([])
  })

  it('registered source row shows scan/rescan, remove, and maintenance when allowed', () => {
    const view = registeredView({
      sourceLifecycle: lifecycle({ lastSuccessfulScanAtMs: 20 }),
      sourceMaintenance: maintenance({ remainingHashCandidates: 3 })
    })

    expect(view.badge).toBe('Maintenance needed')
    expect(view.detail).toContain(
      'Maintenance needed: hash 3. Run maintenance processes a bounded batch.'
    )
    expect(view.actions).toEqual([
      expect.objectContaining({ kind: 'scanSource', label: 'Rescan source', enabled: true }),
      expect.objectContaining({ kind: 'runMaintenance', label: 'Run maintenance', enabled: true }),
      expect.objectContaining({ kind: 'refreshStatus', enabled: true }),
      expect.objectContaining({ kind: 'removeSource', label: 'Remove source', enabled: true })
    ])
  })

  it('selected source scan running disables conflicting source actions with selected-source copy', () => {
    const view = registeredView({
      activeSourceOperation: {
        kind: 'scan',
        scope: 'source',
        sourceId: '7'
      },
      scanStatus: 'scanning',
      sourceLifecycle: lifecycle({ scanPhase: 'scanning' }),
      sourceMaintenance: maintenance({ remainingHashCandidates: 1 })
    })

    expect(view.badge).toBe('Indexing')
    expect(view.actions.find((action) => action.kind === 'scanSource')).toMatchObject({
      enabled: false,
      reason: 'A source scan is still running.'
    })
    expect(view.actions.find((action) => action.kind === 'runMaintenance')).toMatchObject({
      enabled: false,
      reason: 'Wait for scan to finish.'
    })
    expect(view.actions.find((action) => action.kind === 'removeSource')).toMatchObject({
      enabled: false,
      reason: 'A source scan is still running.'
    })
  })

  it('does not mark a source as indexing from another source scan', () => {
    const view = registeredView({
      scanStatus: 'idle',
      sourceLifecycle: lifecycle({ scanPhase: 'complete' }),
      sourceIntegrity: integrity({ coverage: 'complete' })
    })

    expect(view.badge).toBe('Ready')
    expect(view.actions.find((action) => action.kind === 'scanSource')).toMatchObject({
      enabled: true
    })
    expect(view.actions.find((action) => action.kind === 'removeSource')).toMatchObject({
      enabled: true
    })
  })

  it('blocks scan and remove honestly when another source scan owns the global scan controller', () => {
    const view = registeredView({
      activeSourceOperation: {
        kind: 'scan',
        scope: 'source',
        sourceId: '8'
      },
      scanStatus: 'scanning',
      sourceLifecycle: lifecycle({ scanPhase: 'complete' }),
      sourceIntegrity: integrity({ coverage: 'complete' }),
      sourceMaintenance: maintenance({ remainingHashCandidates: 1 })
    })

    expect(view.badge).toBe('Maintenance needed')
    expect(view.actions.find((action) => action.kind === 'scanSource')).toMatchObject({
      enabled: false,
      reason: 'Another source scan is running.'
    })
    expect(view.actions.find((action) => action.kind === 'removeSource')).toMatchObject({
      enabled: false,
      reason: 'Another source scan is running.'
    })
    expect(view.actions.find((action) => action.kind === 'runMaintenance')).toMatchObject({
      enabled: true
    })
  })

  it('blocks scan and remove honestly when the active scan source is unknown', () => {
    const view = registeredView({
      activeSourceOperation: {
        kind: 'scan',
        scope: 'global'
      },
      scanStatus: 'scanning',
      sourceLifecycle: lifecycle({ scanPhase: 'complete' }),
      sourceIntegrity: integrity({ coverage: 'complete' }),
      sourceMaintenance: maintenance({ remainingHashCandidates: 1 })
    })

    expect(view.badge).toBe('Maintenance needed')
    expect(view.actions.find((action) => action.kind === 'scanSource')).toMatchObject({
      enabled: false,
      reason: 'A source scan is already running.'
    })
    expect(view.actions.find((action) => action.kind === 'removeSource')).toMatchObject({
      enabled: false,
      reason: 'A source scan is already running.'
    })
    expect(view.actions.find((action) => action.kind === 'runMaintenance')).toMatchObject({
      enabled: true
    })
  })

  it('summarizes maintenance backlog by product categories', () => {
    const view = registeredView({
      sourceMaintenance: maintenance({
        remainingHashCandidates: 2,
        remainingProbeCandidates: 1,
        remainingPlayableMediaPromotionCandidates: 4,
        remainingTrackIdentityCandidateProductionCandidates: 3,
        remainingTrackIdentityDecisionProductionCandidates: 5,
        attachmentLinks: {
          currentLinksCount: 0,
          staleLinksCount: 6,
          sourceFilesWithCurrentBlake3ObservationsCount: 0,
          sourceFilesWithAttachmentLinksCount: 0,
          sourceFilesMissingAttachmentLinksCount: 7,
          unmaterializedBlake3ObservationsCount: 0
        }
      })
    })

    expect(view.detail).toContain(
      'Maintenance needed: hash 2, probe 1, promotion 4, identity 8, attachment 13. Run maintenance processes a bounded batch.'
    )
    expect(view.detail).not.toContain('maintenance items pending')
  })

  it('uses integrity fallback only when no maintenance snapshot exists', () => {
    const fallback = registeredView({
      sourceIntegrity: integrity({
        evidenceAndMaintenance: {
          remainingHashCandidates: 0,
          remainingProbeCandidates: 8,
          remainingPlayableMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0
        }
      })
    })

    expect(fallback.badge).toBe('Maintenance needed')
    expect(fallback.detail).toContain(
      'Maintenance needed: probe 8. Run maintenance processes a bounded batch.'
    )

    const freshMaintenance = registeredView({
      sourceIntegrity: integrity({
        evidenceAndMaintenance: {
          remainingHashCandidates: 0,
          remainingProbeCandidates: 8,
          remainingPlayableMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0
        }
      }),
      sourceMaintenance: maintenance({
        remainingHashCandidates: 1
      })
    })

    expect(freshMaintenance.badge).toBe('Maintenance needed')
    expect(freshMaintenance.detail).toContain(
      'Maintenance needed: hash 1. Run maintenance processes a bounded batch.'
    )
    expect(freshMaintenance.detail).not.toContain('integrity fallback')
  })

  it('clears maintenance backlog from source status when fresh maintenance has no remaining work', () => {
    const view = registeredView({
      sourceIntegrity: integrity({
        evidenceAndMaintenance: {
          remainingHashCandidates: 9,
          remainingProbeCandidates: 8,
          remainingPlayableMediaPromotionCandidates: 7,
          remainingTrackIdentityCandidateProductionCandidates: 6,
          remainingTrackIdentityDecisionProductionCandidates: 5
        },
        attachmentIntegrity: {
          currentLinksCount: 0,
          staleLinksCount: 2,
          missingLinksCount: 3,
          sourceFilesWithCurrentBlake3ObservationsCount: 0,
          sourceFilesWithAttachmentLinksCount: 0,
          unmaterializedBlake3ObservationsCount: 0
        }
      }),
      sourceMaintenance: maintenance()
    })

    expect(view.badge).toBe('Ready')
    expect(view.detail).toBe('Source status is current.')
  })

  it('uses run-result maintenance counts after completed maintenance still has bounded work', () => {
    const view = registeredView({
      maintenanceRunState: 'completed',
      sourceMaintenance: maintenance({
        remainingHashCandidates: 3131,
        remainingProbeCandidates: 2322,
        attachmentLinks: {
          currentLinksCount: 0,
          staleLinksCount: 4000,
          sourceFilesWithCurrentBlake3ObservationsCount: 0,
          sourceFilesWithAttachmentLinksCount: 0,
          sourceFilesMissingAttachmentLinksCount: 1457,
          unmaterializedBlake3ObservationsCount: 0
        },
        lastRun: maintenanceLastRun({
          remainingHashCandidates: 3131,
          remainingProbeCandidates: 2322,
          remainingPlayableMediaPromotionCandidates: 0,
          remainingTrackIdentityCandidateProductionCandidates: 0,
          remainingTrackIdentityDecisionProductionCandidates: 0
        })
      })
    })

    expect(view.badge).toBe('Maintenance needed')
    expect(view.detail).toContain(
      'Maintenance completed; pending work remains: hash 3131, probe 2322, attachment 5457. Run maintenance processes a bounded batch.'
    )
  })

  it('run-result maintenance snapshot with zero remaining work clears maintenance needed', () => {
    const view = registeredView({
      maintenanceRunState: 'completed',
      sourceIntegrity: integrity({ coverage: 'complete' }),
      sourceMaintenance: maintenance({
        lastRun: maintenanceLastRun()
      })
    })

    expect(view.badge).toBe('Ready')
    expect(view.detail).toBe('Source status is current.')
  })

  it('maps running source activity to scanning copy without percentages', () => {
    const view = registeredView({
      sourceActivity: sourceActivity({
        scan: {
          state: 'running',
          counters: {
            directoriesVisited: 3,
            filesDiscovered: 12,
            queuedWorkItems: 4
          }
        }
      })
    })

    expect(view.badge).toBe('Scanning source')
    expect(view.detail).toContain('Scanning source: 12 files discovered, 3 folders visited')
    expect(view.detail).toContain('4 work items queued')
    expect(`${view.badge} ${view.detail}`).not.toContain('%')
  })

  it('does not let ready rows clear source preparation activity', () => {
    const view = registeredView({
      sourceReadiness: {
        kind: 'ready',
        sourceNodeId: 'selected',
        detail: 'The source hierarchy is ready.'
      },
      sourceActivity: sourceActivity({
        preparation: {
          state: 'idle',
          backlog: {
            hash: 2,
            probe: 1,
            attachment: 3,
            promotion: 4,
            identity: 5
          }
        }
      })
    })

    expect(view.badge).toBe('Maintenance needed')
    expect(view.detail).toContain(
      'Maintenance needed: hash 2, probe 1, attachment 3, promotion 4, identity 5.'
    )
  })

  it('keeps completed maintenance with remaining activity distinct from current maintenance', () => {
    const view = registeredView({
      sourceActivity: sourceActivity({
        preparation: {
          state: 'completedWithRemainingWork',
          provenance: 'runResult',
          lastRunStatus: 'completed',
          backlog: {
            hash: 7,
            probe: 0,
            attachment: 0,
            promotion: 0,
            identity: 0
          }
        }
      })
    })

    expect(view.badge).toBe('Maintenance completed; pending work remains')
    expect(view.detail).toContain('Maintenance completed; pending work remains: hash 7.')
  })

  it('source activity complete clears maintenance warning state', () => {
    const view = registeredView({
      sourceActivity: sourceActivity({
        preparation: {
          state: 'complete'
        }
      })
    })

    expect(view.badge).toBe('Ready')
    expect(view.tone).toBe('ready')
    expect(view.detail).toBe('Maintenance current.')
  })

  it('does not let stale integrity attachment counts keep backlog alive after a zero run result', () => {
    const view = registeredView({
      maintenanceRunState: 'completed',
      sourceIntegrity: integrity({
        coverage: 'complete',
        attachmentIntegrity: {
          currentLinksCount: 0,
          staleLinksCount: 8,
          missingLinksCount: 13,
          sourceFilesWithCurrentBlake3ObservationsCount: 0,
          sourceFilesWithAttachmentLinksCount: 0,
          unmaterializedBlake3ObservationsCount: 0
        }
      }),
      sourceMaintenance: maintenance({
        lastRun: maintenanceLastRun()
      })
    })

    expect(view.badge).toBe('Ready')
    expect(view.detail).toBe('Source status is current.')
  })

  it('keeps running, failed, and unavailable maintenance states distinct', () => {
    const running = registeredView({
      maintenanceRunState: 'running',
      sourceMaintenance: maintenance({ remainingHashCandidates: 4 })
    })
    expect(running.badge).toBe('Maintenance running')
    expect(running.detail).toContain('Maintenance running for this source.')

    const failed = registeredView({
      maintenanceRunState: 'failed',
      sourceMaintenance: maintenance({ status: 'failed' })
    })
    expect(failed.badge).toBe('Maintenance unavailable')
    expect(failed.detail).toContain('Maintenance status could not be read.')

    const unavailable = registeredView({
      sourceMaintenance: maintenance({ status: 'unavailable' })
    })
    expect(unavailable.badge).toBe('Maintenance unavailable')
    expect(unavailable.detail).toContain('Maintenance is unavailable for this source.')
  })

  it('builds development diagnostic trace data from admission and maintenance truth', () => {
    const context: StatusContext = {
      kind: 'localBrowse',
      title: 'Removed',
      itemRole: 'folder',
      localState: 'restorable',
      detail: 'C:/Music/Removed',
      matchedSourceId: '7',
      admission: {
        resolvedPath: 'C:/Music/Removed',
        label: 'Restore source'
      }
    }
    const trace = sourceStatusDiagnosticTrace({
      context,
      sourcePath: 'C:/Music/Removed',
      sourceMaintenance: maintenance({
        remainingHashCandidates: 2,
        lastRun: maintenanceLastRun({
          status: 'completed',
          remainingHashCandidates: 2
        })
      }),
      maintenanceRunState: 'completed',
      canAddLocalPath: true,
      scanStatus: 'idle',
      removeSourceStatus: 'idle',
      refreshStatus: 'idle',
      canScan: true,
      canRemove: true,
      canRunMaintenance: true
    })

    expect(trace).toEqual({
      sourceId: '7',
      sourcePath: 'C:/Music/Removed',
      admission: 'restorable',
      duplicateStatus: 'restorable',
      maintenanceSnapshotSource: 'runResult',
      backlogCounts: [{ label: 'hash', count: 2 }],
      backlogTotal: 2,
      provenance: 'runResult',
      lastRunStatus: 'completed'
    })
  })

  it('development diagnostic trace includes source activity truth and provenance', () => {
    const trace = sourceStatusDiagnosticTrace({
      context: {
        kind: 'registeredSource',
        title: 'Source Fixture',
        sourceId: '7',
        nodeId: 'selected'
      },
      sourcePath: 'C:/Music',
      sourceActivity: sourceActivity({
        admission: 'active',
        browse: {
          state: 'ready',
          detail: 'Source is ready to browse.'
        },
        scan: {
          state: 'running',
          counters: {
            filesDiscovered: 9
          },
          scanRunId: 'scan-1'
        },
        preparation: {
          state: 'completedWithRemainingWork',
          provenance: 'runResult',
          lastRunStatus: 'completed',
          backlog: {
            hash: 0,
            probe: 2,
            attachment: 0,
            promotion: 0,
            identity: 1
          }
        }
      }),
      canAddLocalPath: true,
      scanStatus: 'idle',
      removeSourceStatus: 'idle',
      refreshStatus: 'idle',
      canScan: true,
      canRemove: true,
      canRunMaintenance: true
    })

    expect(trace).toMatchObject({
      sourceId: '7',
      sourcePath: 'C:/Music',
      admission: 'active',
      browse: { state: 'ready' },
      scan: { state: 'running', counters: { filesDiscovered: 9 } },
      preparation: {
        state: 'completedWithRemainingWork',
        provenance: 'runResult',
        lastRunStatus: 'completed'
      },
      backlogCounts: [
        { label: 'probe', count: 2 },
        { label: 'identity', count: 1 }
      ],
      backlogTotal: 3,
      provenance: 'runResult',
      lastRunStatus: 'completed'
    })
  })

  it('missing, unavailable, and blocked sources project compact badges', () => {
    expect(registeredView({ sourceIntegrity: integrity({ availability: 'missing' }) }).badge).toBe(
      'Missing'
    )
    expect(
      registeredView({ sourceIntegrity: integrity({ availability: 'unavailable' }) }).badge
    ).toBe('Offline/unavailable')
    expect(registeredView({ sourceIntegrity: integrity({ availability: 'blocked' }) }).badge).toBe(
      'Blocked'
    )
  })

  it('partial and incomplete health projects Partial', () => {
    expect(
      registeredView({
        sourceIntegrity: integrity({ availability: 'partial', coverage: 'incomplete' })
      }).badge
    ).toBe('Partial')
  })

  it.each([
    {
      profile: 'audio',
      badge: 'No audio tracks in this view',
      detail: 'No audio tracks in this view.'
    },
    {
      profile: 'playable',
      badge: 'No playable media in this view',
      detail: 'No playable media in this view.'
    },
    {
      profile: 'allFiles',
      badge: 'No files in this source inventory view',
      detail: 'No files in this source inventory view.'
    }
  ] satisfies readonly {
    readonly profile: LibraryBrowseProfile
    readonly badge: string
    readonly detail: string
  }[])('empty registered source uses $profile browse profile status copy', (expected) => {
    const view = registeredView({
      libraryBrowseProfile: expected.profile,
      sourceReadiness: {
        kind: 'empty',
        sourceNodeId: 'selected',
        detail: 'No visible rows.'
      }
    })

    expect(view.badge).toBe(expected.badge)
    expect(view.detail).toBe(expected.detail)
  })

  it('registered source fallback is concrete while rows are unresolved', () => {
    const view = registeredView()

    expect(view.badge).toBe('Still indexing')
  })

  it('does not treat local browse rows as durable source rows', () => {
    const projection = localBrowseProjection(
      {
        kind: 'localBrowseItem',
        item: localItem([], 'directory'),
        target: {
          addSourceView: 'preview',
          entryPointKind: 'music',
          resolvedRootPath: 'C:/Music',
          resolvedParentPath: 'C:/Music/Albums',
          label: 'Albums'
        }
      },
      'Albums'
    )
    const context = projectStatusContext({
      projection,
      selectedNodeId: 'selected',
      selectedTitle: 'Albums'
    })
    const view = statusView(context)

    expect(context.kind).toBe('localBrowse')
    expect(view.actions.some((action) => action.kind === 'scanSource')).toBe(false)
    expect(view.actions.some((action) => action.kind === 'removeSource')).toBe(false)
  })

  it('does not change browse profile state', () => {
    const profile = createLibraryBrowseProfileController()
    profile.setProfile('allFiles')

    registeredView({ sourceIntegrity: integrity({ coverage: 'complete' }) })

    expect(profile.profile.value).toBe('allFiles')
  })

  it('status context remains selected-tree scoped while search is active elsewhere', () => {
    const context = projectStatusContext({
      projection: sourceProjection(),
      selectedNodeId: 'selected',
      selectedTitle: 'Source Fixture'
    })
    const view = statusView(context, { sourceIntegrity: integrity({ coverage: 'complete' }) })

    expect(view.role).toBe('registeredSource')
    expect(view.title).toBe('Source Fixture')
    expect(view.title).not.toBe('Search results')
  })

  it('registered directory context uses source-scoped actions without directory maintenance claims', () => {
    const projection = projectionForBinding(
      {
        kind: 'directory',
        sourceId: '7',
        directoryId: '12',
        entryPoint: { kind: 'source', sourceId: '7' },
        label: 'Source Fixture'
      },
      'Album'
    )
    const context = projectStatusContext({
      projection,
      selectedNodeId: 'selected',
      selectedTitle: 'Album'
    })
    const view = statusView(context, {
      sourceLifecycle: lifecycle(),
      sourceIntegrity: integrity({ coverage: 'pending' })
    })

    expect(context.kind).toBe('registeredDirectory')
    expect(view.badge).toBe('Needs scan')
    expect(view.detail).toContain('Folder status follows its registered source.')
    expect(view.actions.find((action) => action.kind === 'runMaintenance')).toMatchObject({
      label: 'Run maintenance',
      sourceId: '7'
    })
  })
})

function registeredView(
  options: Partial<StatusViewInput> = {}
): ReturnType<typeof projectStatusView> {
  return statusView(
    {
      kind: 'registeredSource',
      title: 'Source Fixture',
      sourceId: '7',
      nodeId: 'selected'
    },
    {
      sourceLifecycle: lifecycle(),
      ...options
    }
  )
}

function statusView(
  context: StatusContext,
  options: Partial<StatusViewInput> = {}
): ReturnType<typeof projectStatusView> {
  return projectStatusView({
    context,
    canAddLocalPath: true,
    scanStatus: 'idle',
    removeSourceStatus: 'idle',
    refreshStatus: 'idle',
    canScan: true,
    canRemove: true,
    canRunMaintenance: true,
    ...options
  })
}

function sourceProjection(): BrowserProjection {
  return projectionForBinding(
    {
      kind: 'source',
      navigationRow: {
        navigationRowId: '7',
        stableKey: 'source:7',
        parentNavigationRowId: null,
        family: 'sources',
        rowKind: 'source',
        displayName: 'Source Fixture',
        siblingPosition: 0,
        selectable: true,
        selectorKind: 'source',
        selectorPayload: '7',
        updatedAtMs: 100,
        rowVersion: '1'
      },
      target: {
        navigationRowId: '7',
        entryPoint: { kind: 'source', sourceId: '7' },
        label: 'Source Fixture'
      }
    },
    'Source Fixture'
  )
}

function localBrowseProjection(binding: RowBinding, label: string): BrowserProjection {
  return projectionForBinding(binding, label)
}

function projectionForBinding(binding: RowBinding, label: string): BrowserProjection {
  return {
    kind: 'tree',
    nodes: [node(label)],
    bindingsById: new Map([['selected', binding]])
  }
}

function node(label: string): BrowserTreeNode {
  return {
    id: 'selected',
    role: 'source',
    label,
    children: { kind: 'none' }
  }
}

function admission(
  requestKind: Extract<
    LocalBrowseOperation,
    { readonly kind: 'requestSourceAdmission' }
  >['requestKind'],
  resolvedPath: string
): LocalBrowseOperation {
  return {
    kind: 'requestSourceAdmission',
    requestKind,
    resolvedPath
  }
}

function localEntry(operations: readonly LocalBrowseOperation[]): LocalBrowseEntryPoint {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedPath: 'C:/Music'
    },
    displayName: 'Music',
    status: 'available',
    platform: 'windows',
    availableOperations: operations,
    failure: null
  }
}

function localItem(
  operations: readonly LocalBrowseOperation[],
  itemKind: LocalBrowseItem['itemKind']
): LocalBrowseItem {
  return {
    identity: {
      entryPointKind: 'music',
      resolvedRootPath: 'C:/Music',
      resolvedItemPath: itemKind === 'mediaFile' ? 'C:/Music/Album/track.flac' : 'C:/Music/Album'
    },
    itemKind,
    displayName: itemKind === 'mediaFile' ? 'track.flac' : 'Album',
    status: 'available',
    platform: 'windows',
    fileKind: itemKind === 'mediaFile' ? 'audio' : null,
    mediaRelevance: itemKind === 'mediaFile' ? 'mediaRelevant' : null,
    availableOperations: operations,
    failure: null
  }
}

function lifecycle(overrides: Partial<SourceLifecycleRecord> = {}): SourceLifecycleRecord {
  return {
    sourceId: '7',
    sourceClass: 'externalMounted',
    isUserVisible: true,
    mountStatus: 'mounted',
    accessState: 'accessible',
    scanPhase: 'complete',
    lastSuccessfulScanAtMs: 20,
    updatedAtMs: 100,
    ...overrides
  }
}

function sourceActivity(
  overrides: {
    readonly admission?: ReadSourceActivityReply['admission']
    readonly browse?: Partial<ReadSourceActivityReply['browse']>
    readonly scan?: Partial<ReadSourceActivityReply['scan']>
    readonly preparation?: Partial<ReadSourceActivityReply['preparation']>
  } = {}
): ReadSourceActivityReply {
  const base: ReadSourceActivityReply = {
    sourceId: '7',
    admission: 'active',
    browse: {
      state: 'ready',
      detail: 'Source is ready to browse.'
    },
    scan: {
      state: 'completed',
      counters: {},
      detail: 'Scan completed.'
    },
    preparation: {
      state: 'complete',
      backlog: {
        hash: 0,
        probe: 0,
        attachment: 0,
        promotion: 0,
        identity: 0
      },
      provenance: 'maintenanceSnapshot',
      boundedBatch: true
    }
  }

  const preparation =
    overrides.preparation === undefined
      ? base.preparation
      : {
          ...base.preparation,
          ...overrides.preparation,
          backlog: {
            ...base.preparation.backlog,
            ...(overrides.preparation.backlog ?? {})
          }
        }

  return {
    ...base,
    ...(overrides.admission === undefined ? {} : { admission: overrides.admission }),
    browse:
      overrides.browse === undefined
        ? base.browse
        : {
            ...base.browse,
            ...overrides.browse
          },
    scan:
      overrides.scan === undefined
        ? base.scan
        : {
            ...base.scan,
            ...overrides.scan,
            counters: {
              ...base.scan.counters,
              ...(overrides.scan.counters ?? {})
            }
          },
    preparation
  }
}

function integrity(
  options: {
    readonly availability?: ReadSourceIntegrityReply['sourceAvailability']['state']
    readonly coverage?: ReadSourceIntegrityReply['coverageIntegrity']['state']
    readonly evidenceAndMaintenance?: ReadSourceIntegrityReply['evidenceAndMaintenance']
    readonly attachmentIntegrity?: ReadSourceIntegrityReply['attachmentIntegrity']
  } = {}
): ReadSourceIntegrityReply {
  return {
    sourceId: '7',
    sourceAvailability: {
      state: options.availability ?? 'mounted'
    },
    coverageIntegrity: {
      state: options.coverage ?? 'complete',
      subtreeCoverageComplete: options.coverage === 'incomplete' ? false : true,
      emptyResultAuthoritative: true,
      totalDirectoriesCount: 1,
      missingDirectoriesCount: options.availability === 'missing' ? 1 : 0,
      pendingDirectoriesCount: options.coverage === 'pending' ? 1 : 0,
      scanningDirectoriesCount: options.coverage === 'scanning' ? 1 : 0,
      blockedDirectoriesCount: options.availability === 'blocked' ? 1 : 0,
      failedDirectoriesCount: 0
    },
    evidenceAndMaintenance: options.evidenceAndMaintenance ?? {
      remainingHashCandidates: 0,
      remainingProbeCandidates: 0,
      remainingPlayableMediaPromotionCandidates: 0,
      remainingTrackIdentityCandidateProductionCandidates: 0,
      remainingTrackIdentityDecisionProductionCandidates: 0
    },
    ...(options.attachmentIntegrity === undefined
      ? {}
      : { attachmentIntegrity: options.attachmentIntegrity }),
    runtimeMaintenance: {
      state: 'idle'
    }
  }
}

function maintenance(
  overrides: Partial<ReadSourceMaintenanceReply> = {}
): ReadSourceMaintenanceReply {
  return {
    sourceId: '7',
    status: 'idle',
    remainingHashCandidates: 0,
    remainingProbeCandidates: 0,
    remainingPlayableMediaPromotionCandidates: 0,
    remainingTrackIdentityCandidateProductionCandidates: 0,
    remainingTrackIdentityDecisionProductionCandidates: 0,
    ...overrides
  }
}

function maintenanceLastRun(
  overrides: Partial<NonNullable<ReadSourceMaintenanceReply['lastRun']>> = {}
): NonNullable<ReadSourceMaintenanceReply['lastRun']> {
  return {
    status: 'completed',
    hash: {
      effectiveLimit: 1,
      hashedCount: 1,
      skippedCount: 0,
      failedCount: 0,
      remainingCandidates: 0
    },
    attachmentMaterialization: {
      effectiveLimit: 1,
      attachmentsCreated: 0,
      attachmentsRefreshed: 0,
      linksCreated: 0,
      linksReplaced: 0,
      linksRefreshed: 0,
      skippedStaleObservations: 0,
      skippedNoBlake3: 0,
      skippedNoObservations: 0,
      remainingCandidates: 0
    },
    probe: {
      effectiveLimit: 1,
      probedCount: 1,
      skippedCount: 0,
      failedCount: 0,
      remainingCandidates: 0
    },
    playableMediaPromotion: {
      effectiveLimit: 0,
      promotedCount: 0,
      refreshedCount: 0,
      skippedUnusableSource: 0,
      skippedUnsupportedMediaKind: 0,
      skippedNoObservations: 0,
      skippedStaleObservations: 0,
      skippedNoBlake3: 0,
      skippedNoProbeObservations: 0,
      skippedMissingAttachmentLink: 0,
      skippedStaleAttachmentLink: 0,
      remainingCandidates: 0
    },
    trackIdentityCandidates: {
      effectiveLimit: 0,
      candidatesCreated: 0,
      candidatesRefreshed: 0,
      membersCreated: 0,
      membersRefreshed: 0,
      evidenceCreated: 0,
      evidenceRefreshed: 0,
      candidatesMarkedStale: 0,
      skippedStalePlayableMedia: 0,
      remainingCandidates: 0
    },
    trackIdentityDecisions: {
      effectiveLimit: 0,
      decisionsCreated: 0,
      decisionEvidenceCreated: 0,
      skippedStaleCandidates: 0,
      skippedExistingCurrentDecisions: 0,
      skippedUserBlockedCandidates: 0,
      remainingCandidates: 0
    },
    remainingHashCandidates: 0,
    remainingProbeCandidates: 0,
    remainingPlayableMediaPromotionCandidates: 0,
    remainingTrackIdentityCandidateProductionCandidates: 0,
    remainingTrackIdentityDecisionProductionCandidates: 0,
    ...overrides
  }
}

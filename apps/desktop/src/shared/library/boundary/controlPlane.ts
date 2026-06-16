export const libraryControlChannels = {
  attachmentIdentity: {
    readSourceFileAttachment: 'desktop:library-attachment-identity:read-source-file-attachment',
    readAttachmentSourceFiles: 'desktop:library-attachment-identity:read-attachment-source-files',
    readSourceAttachmentSummary:
      'desktop:library-attachment-identity:read-source-attachment-summary'
  },
  boundary: {
    getStatus: 'desktop:library-boundary:get-status',
    events: {
      subscribe: 'desktop:library-boundary:events:subscribe',
      unsubscribe: 'desktop:library-boundary:events:unsubscribe'
    }
  },
  contents: {
    read: 'desktop:library-contents:read'
  },
  searchFilter: {
    read: 'desktop:library-search-filter:read'
  },
  hierarchy: {
    read: 'desktop:library-hierarchy:read-children'
  },
  localBrowse: {
    entryPoints: {
      read: 'desktop:library-local-browse-entry-points:read'
    },
    items: {
      read: 'desktop:library-local-browse-items:read'
    }
  },
  musicalAnalysis: {
    analyzePlayableMedia: 'desktop:library-musical-analysis:analyze-playable-media'
  },
  navigation: {
    read: 'desktop:library-navigation:read-rows'
  },
  roots: {
    cancel: 'desktop:library-roots:cancel-scan',
    chooseLocal: 'desktop:library-roots:choose-and-register-local',
    registerLocalPath: 'desktop:library-roots:register-local-path',
    read: 'desktop:library-roots:read-local-roots',
    scan: 'desktop:library-roots:run-scan',
    unregister: 'desktop:library-roots:unregister-local-root'
  },
  source: {
    activity: 'desktop:library-source-activity:read-source-activity',
    fileHashing: 'desktop:library-source-file-hashing:hash-source-files-blake3',
    integrity: 'desktop:library-source-integrity:read-source-integrity',
    lifecycle: 'desktop:library-source-lifecycle:read-source-lifecycle',
    maintenance: {
      run: 'desktop:library-source-maintenance:run-source-maintenance',
      read: 'desktop:library-source-maintenance:read-source-maintenance'
    }
  },
  trackIdentity: {
    candidates: {
      read: 'desktop:library-track-identity-review:read-candidates'
    },
    decisions: {
      accept: 'desktop:library-track-identity-decisions:accept-track-identity-candidate',
      reject: 'desktop:library-track-identity-decisions:reject-track-identity-candidate',
      defer: 'desktop:library-track-identity-decisions:defer-track-identity-candidate'
    }
  },
  viewState: {
    read: 'desktop:library:view-state:read',
    write: 'desktop:library:view-state:write'
  }
} as const

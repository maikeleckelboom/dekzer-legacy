import type { BrowserTreeFixture } from './tree/types'

export const libraryHierarchyFixtureTree: BrowserTreeFixture = {
  name: 'Library hierarchy fixture',
  detail: 'Renderer-only demo input for tree interactions; not scanned and not live.',
  nodes: [
    {
      id: 'fixture-root',
      label: 'Fixture Library Root',
      kind: 'fixtureRoot',
      detail: 'Demo root for renderer tree behavior.',
      childrenState: {
        kind: 'loaded',
        children: [
          {
            id: 'fixture-artists',
            label: 'Artists',
            kind: 'folder',
            detail: 'Fixture grouping placeholder.',
            childrenState: { kind: 'leaf' }
          },
          {
            id: 'fixture-albums',
            label: 'Albums',
            kind: 'folder',
            detail: 'Fixture grouping placeholder.',
            childrenState: { kind: 'leaf' }
          },
          {
            id: 'fixture-tracks',
            label: 'Tracks',
            kind: 'trackGroup',
            detail: 'Fixture grouping placeholder.',
            childrenState: {
              kind: 'loaded',
              children: [
                {
                  id: 'fixture-tracks-group',
                  label: 'Fixture Track Group',
                  kind: 'trackGroup',
                  detail: 'Demo child row for nested hierarchy rendering.',
                  childrenState: { kind: 'leaf' }
                }
              ]
            }
          },
          {
            id: 'fixture-playlists',
            label: 'Playlists',
            kind: 'playlistGroup',
            detail: 'Fixture grouping placeholder.',
            childrenState: {
              kind: 'loaded',
              children: [
                {
                  id: 'fixture-playlist-group',
                  label: 'Fixture Playlist Group',
                  kind: 'playlistGroup',
                  detail: 'Demo child row for nested hierarchy rendering.',
                  childrenState: { kind: 'leaf' }
                }
              ]
            }
          },
          {
            id: 'fixture-preparation',
            label: 'Preparation',
            kind: 'preparation',
            detail: 'Fixture workflow placeholder.',
            childrenState: { kind: 'leaf' }
          },
          {
            id: 'fixture-history',
            label: 'History',
            kind: 'history',
            detail: 'Fixture history placeholder.',
            childrenState: { kind: 'leaf' }
          }
        ]
      }
    }
  ]
}

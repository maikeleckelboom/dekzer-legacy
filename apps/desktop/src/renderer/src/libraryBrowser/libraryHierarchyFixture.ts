import type { TreeFixture } from './tree/types'

export const libraryHierarchyFixtureTree: TreeFixture = {
  name: 'Library hierarchy fixture',
  detail: 'Renderer-only demo input for tree interactions; not scanned and not live.',
  nodes: [
    {
      id: 'fixture-root',
      label: 'Fixture Library Root',
      kind: 'fixtureRoot',
      detail: 'Demo root for renderer tree behavior.',
      children: [
        {
          id: 'fixture-artists',
          label: 'Artists',
          kind: 'folder',
          detail: 'Fixture grouping placeholder.'
        },
        {
          id: 'fixture-albums',
          label: 'Albums',
          kind: 'folder',
          detail: 'Fixture grouping placeholder.'
        },
        {
          id: 'fixture-tracks',
          label: 'Tracks',
          kind: 'trackGroup',
          detail: 'Fixture grouping placeholder.',
          children: [
            {
              id: 'fixture-tracks-group',
              label: 'Fixture Track Group',
              kind: 'trackGroup',
              detail: 'Demo child row for nested hierarchy rendering.'
            }
          ]
        },
        {
          id: 'fixture-playlists',
          label: 'Playlists',
          kind: 'playlistGroup',
          detail: 'Fixture grouping placeholder.',
          children: [
            {
              id: 'fixture-playlist-group',
              label: 'Fixture Playlist Group',
              kind: 'playlistGroup',
              detail: 'Demo child row for nested hierarchy rendering.'
            }
          ]
        },
        {
          id: 'fixture-preparation',
          label: 'Preparation',
          kind: 'preparation',
          detail: 'Fixture workflow placeholder.'
        },
        {
          id: 'fixture-history',
          label: 'History',
          kind: 'history',
          detail: 'Fixture history placeholder.'
        }
      ]
    }
  ]
}

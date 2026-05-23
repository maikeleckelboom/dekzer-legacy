import { strict as assert } from 'node:assert'

import {
  adaptLocationSourceDescriptor,
  formatSourceDisplayName,
  getLocationSourceIcon,
  getLocationSourceIconBadge,
  getLocationSourceKindLabel,
  getLocationSourcePresentation,
  type LocationSourceDescriptor,
  type LocationSourceIcon,
  type LocationSourceIconBadge,
  type LocationSourceKind,
  type LocationSourcePresentation
} from '../src/renderer/libraryBrowser/projection/sourcePresentation'

const allKinds: readonly LocationSourceKind[] = [
  'localLibrary',
  'localFolder',
  'localVolume',
  'externalVolume',
  'usbDrive',
  'sdCard',
  'networkShare',
  'cloudMirror',
  'missingSource'
]

void main()

function main(): void {
  validatesEveryKindHasLabel()
  validatesEveryKindMapsToIcon()
  validatesAvailabilityBadges()
  validatesHealthBasedBadges()
  validatesRoleAndAvailabilityBadges()
  validatesPresentationStability()
  validatesKindNamesDoNotEncodeCombinations()
  validatesAdapter()
  validatesSourceDisplayName()
}

function validatesEveryKindHasLabel(): void {
  for (const kind of allKinds) {
    const label = getLocationSourceKindLabel(kind)
    assert.ok(typeof label === 'string', `kind ${kind} should have a label`)
    assert.ok(label.length > 0, `kind ${kind} label should not be empty`)
  }
}

function validatesEveryKindMapsToIcon(): void {
  const expectedIcons: Readonly<Record<LocationSourceKind, LocationSourceIcon>> = {
    localLibrary: 'library',
    localFolder: 'folder',
    localVolume: 'driveInternal',
    externalVolume: 'driveExternal',
    usbDrive: 'usbDrive',
    sdCard: 'sdCard',
    networkShare: 'networkShare',
    cloudMirror: 'cloudMirror',
    missingSource: 'missingSource'
  }

  for (const kind of allKinds) {
    const icon = getLocationSourceIcon(kind)
    assert.equal(icon, expectedIcons[kind], `kind ${kind} should map to ${expectedIcons[kind]}`)
  }
}

function validatesAvailabilityBadges(): void {
  const base: Pick<LocationSourceDescriptor, 'availability' | 'health' | 'role'> = {
    availability: 'available',
    health: 'healthy',
    role: 'primaryLibrary'
  }

  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'missing' }),
    'missing' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'offline' }),
    'offline' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'ejected' }),
    'offline' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'permissionDenied' }),
    'locked' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'stale' }),
    'stale' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'scanning' }),
    'scanning' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, availability: 'indexing' }),
    'scanning' satisfies LocationSourceIconBadge
  )
}

function validatesHealthBasedBadges(): void {
  const base: Pick<LocationSourceDescriptor, 'availability' | 'health' | 'role'> = {
    availability: 'available',
    health: 'healthy',
    role: 'watchedRoot'
  }

  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'warning' }),
    'warning' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'degraded' }),
    'warning' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'missingFiles' }),
    'warning' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'unsupportedFormat' }),
    'warning' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'readOnly' }),
    'readOnly' satisfies LocationSourceIconBadge
  )
  assert.equal(
    getLocationSourceIconBadge({ ...base, health: 'permissionIssue' }),
    'readOnly' satisfies LocationSourceIconBadge
  )
}

function validatesRoleAndAvailabilityBadges(): void {
  const availableDescriptor: Pick<LocationSourceDescriptor, 'availability' | 'health' | 'role'> = {
    availability: 'available',
    health: 'healthy',
    role: 'watchedRoot'
  }

  assert.equal(
    getLocationSourceIconBadge(availableDescriptor),
    'persisted' satisfies LocationSourceIconBadge | undefined
  )

  const primaryLibraryDescriptor: Pick<
    LocationSourceDescriptor,
    'availability' | 'health' | 'role'
  > = {
    availability: 'offline',
    health: 'healthy',
    role: 'primaryLibrary'
  }

  assert.equal(
    getLocationSourceIconBadge(primaryLibraryDescriptor),
    'offline' satisfies LocationSourceIconBadge | undefined,
    'offline takes precedence over role'
  )

  const noRoleNoAvailableDescriptor: Pick<
    LocationSourceDescriptor,
    'availability' | 'health' | 'role'
  > = {
    availability: 'offline',
    health: 'healthy',
    role: 'temporary'
  }

  assert.equal(
    getLocationSourceIconBadge(noRoleNoAvailableDescriptor),
    'offline' satisfies LocationSourceIconBadge | undefined,
    'availability offline should return offline badge regardless of role'
  )

  const healthyAvailableNonPrimary: Pick<
    LocationSourceDescriptor,
    'availability' | 'health' | 'role'
  > = {
    availability: 'available',
    health: 'healthy',
    role: 'importSource'
  }

  assert.equal(
    getLocationSourceIconBadge(healthyAvailableNonPrimary),
    'persisted' satisfies LocationSourceIconBadge | undefined,
    'available non-primary should return persisted badge'
  )
}

function validatesPresentationStability(): void {
  const descriptor: LocationSourceDescriptor = {
    kind: 'localLibrary',
    medium: 'unknown',
    connectivity: 'internal',
    role: 'primaryLibrary',
    availability: 'available',
    health: 'healthy'
  }

  const presentation: LocationSourcePresentation = getLocationSourcePresentation(descriptor)

  assert.equal(presentation.label, 'Local Library')
  assert.equal(presentation.icon, 'library')
  assert.equal(presentation.badge, 'persisted')

  const descriptor2: LocationSourceDescriptor = {
    kind: 'missingSource',
    medium: 'unknown',
    connectivity: 'unknown',
    role: 'watchedRoot',
    availability: 'missing',
    health: 'missingFiles'
  }

  const presentation2 = getLocationSourcePresentation(descriptor2)

  assert.equal(presentation2.label, 'Missing Source')
  assert.equal(presentation2.icon, 'missingSource')
  assert.equal(presentation2.badge, 'missing')

  const descriptor3: LocationSourceDescriptor = {
    kind: 'usbDrive',
    medium: 'flash',
    connectivity: 'usb',
    role: 'importSource',
    availability: 'available',
    health: 'healthy'
  }

  const presentation3 = getLocationSourcePresentation(descriptor3)

  assert.equal(presentation3.label, 'USB Drive')
  assert.equal(presentation3.icon, 'usbDrive')
  assert.equal(presentation3.badge, 'persisted')
}

function validatesKindNamesDoNotEncodeCombinations(): void {
  for (const kind of allKinds) {
    assert.ok(
      !kind.includes('Ssd') &&
        !kind.includes('Hdd') &&
        !kind.includes('Performance') &&
        !kind.includes('CloudSynced') &&
        !kind.includes('Stale'),
      `kind "${kind}" should not encode medium/connectivity/role/status in its name`
    )
  }
}

function validatesAdapter(): void {
  const sourceRowResult = adaptLocationSourceDescriptor({
    rowKind: 'source',
    selectorKind: 'source',
    sourceStateKind: 'loaded',
    sourceStateFailed: false
  })

  assert.equal(sourceRowResult.kind, 'localLibrary')
  assert.equal(sourceRowResult.role, 'primaryLibrary')
  assert.equal(sourceRowResult.availability, 'available')
  assert.equal(sourceRowResult.health, 'healthy')

  const locationRowResult = adaptLocationSourceDescriptor({
    rowKind: 'location',
    selectorKind: 'sourceLocation',
    sourceStateKind: 'unloaded',
    sourceStateFailed: false
  })

  assert.equal(locationRowResult.kind, 'localFolder')
  assert.equal(locationRowResult.role, 'watchedRoot')
  assert.equal(locationRowResult.availability, 'scanning')
  assert.equal(locationRowResult.health, 'healthy')

  const failedRowResult = adaptLocationSourceDescriptor({
    rowKind: 'source',
    selectorKind: 'source',
    sourceStateKind: 'failed',
    sourceStateFailed: true
  })

  assert.equal(failedRowResult.kind, 'localLibrary')
  assert.equal(failedRowResult.availability, 'missing')
  assert.equal(failedRowResult.health, 'warning')

  const locationGroupResult = adaptLocationSourceDescriptor({
    rowKind: 'locationGroup',
    selectorKind: 'sourceLocation',
    sourceStateKind: 'loading',
    sourceStateFailed: false
  })

  assert.equal(locationGroupResult.kind, 'localVolume')
  assert.equal(locationGroupResult.role, 'watchedRoot')
  assert.equal(locationGroupResult.availability, 'scanning')
  assert.equal(locationGroupResult.health, 'healthy')
}

console.log('Location sources validation passed.')

function validatesSourceDisplayName(): void {
  assert.equal(
    formatSourceDisplayName('\\\\?\\C:\\Users\\Maikel\\Music'),
    'Music',
    'Windows extended-length local path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('C:\\Users\\Maikel\\Music'),
    'Music',
    'Windows local path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('/Users/maikel/Music'),
    'Music',
    'Unix path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('/home/maikel/Music'),
    'Music',
    'Unix home path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('Music'),
    'Music',
    'Plain name should pass through unchanged'
  )
  assert.equal(
    formatSourceDisplayName('\\\\?\\C:\\'),
    'C:',
    'Windows extended-length drive root should show drive letter'
  )
  assert.equal(formatSourceDisplayName('C:\\'), 'C:', 'Windows drive root should show drive letter')
  assert.equal(
    formatSourceDisplayName('\\\\?\\UNC\\server\\share\\Music'),
    'Music',
    'Windows extended-length UNC path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('\\\\?\\UNC\\server\\share'),
    'share',
    'Windows extended-length UNC share root should show share name'
  )
  assert.equal(
    formatSourceDisplayName('\\\\server\\share\\Music'),
    'Music',
    'Windows UNC path should show final segment'
  )
  assert.equal(
    formatSourceDisplayName('\\\\server\\share'),
    'share',
    'Windows UNC share root should show share name'
  )
  assert.equal(
    formatSourceDisplayName(''),
    '',
    'Empty string should return empty string without error'
  )
  assert.equal(
    formatSourceDisplayName('\\\\?\\'),
    '\\\\?\\',
    'Bare extended-length prefix with no path should fall back to raw label'
  )
  assert.equal(
    formatSourceDisplayName('\\\\?\\C:'),
    'C:',
    'Windows extended-length drive letter without trailing slash should show drive letter'
  )
}

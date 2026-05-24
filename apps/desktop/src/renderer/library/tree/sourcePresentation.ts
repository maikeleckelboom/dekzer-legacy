export type LocationSourceKind =
  | 'localLibrary'
  | 'localFolder'
  | 'localVolume'
  | 'externalVolume'
  | 'usbDrive'
  | 'sdCard'
  | 'networkShare'
  | 'cloudMirror'
  | 'missingSource'

export type StorageMedium =
  | 'unknown'
  | 'ssd'
  | 'hdd'
  | 'flash'
  | 'sdCard'
  | 'network'
  | 'cloud'
  | 'virtual'

export type SourceConnectivity =
  | 'internal'
  | 'usb'
  | 'thunderbolt'
  | 'sdSlot'
  | 'usbCardReader'
  | 'network'
  | 'cloudSync'
  | 'unknown'

export type LocationSourceRole =
  | 'primaryLibrary'
  | 'watchedRoot'
  | 'importSource'
  | 'performanceMedia'
  | 'backup'
  | 'archive'
  | 'cloudMirror'
  | 'temporary'

export type LocationSourceAvailability =
  | 'available'
  | 'offline'
  | 'missing'
  | 'ejected'
  | 'permissionDenied'
  | 'stale'
  | 'scanning'
  | 'indexing'

export type LocationSourceHealth =
  | 'healthy'
  | 'warning'
  | 'degraded'
  | 'missingFiles'
  | 'readOnly'
  | 'permissionIssue'
  | 'unsupportedFormat'
  | 'unknown'

export type LocationSourceIcon =
  | 'library'
  | 'folder'
  | 'drive'
  | 'driveInternal'
  | 'driveExternal'
  | 'usbDrive'
  | 'sdCard'
  | 'networkShare'
  | 'nas'
  | 'cloud'
  | 'cloudMirror'
  | 'mobileDevice'
  | 'djDevice'
  | 'disc'
  | 'archive'
  | 'missingSource'

export type LocationSourceIconBadge =
  | 'available'
  | 'syncing'
  | 'stale'
  | 'offline'
  | 'missing'
  | 'warning'
  | 'locked'
  | 'readOnly'
  | 'scanning'
  | 'persisted'

export interface LocationSourcePresentation {
  readonly label: string
  readonly icon: LocationSourceIcon
  readonly badge?: LocationSourceIconBadge
}

export interface LocationSourceDescriptor {
  readonly kind: LocationSourceKind
  readonly medium: StorageMedium
  readonly connectivity: SourceConnectivity
  readonly role: LocationSourceRole
  readonly availability: LocationSourceAvailability
  readonly health: LocationSourceHealth
}

const locationSourceKindLabels: Readonly<Record<LocationSourceKind, string>> = {
  localLibrary: 'Local Library',
  localFolder: 'Folder',
  localVolume: 'Local Drive',
  externalVolume: 'External Drive',
  usbDrive: 'USB Drive',
  sdCard: 'SD Card',
  networkShare: 'Network Share',
  cloudMirror: 'Cloud Mirror',
  missingSource: 'Missing Source'
}

const locationSourceIconMap: Readonly<Record<LocationSourceKind, LocationSourceIcon>> = {
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

export function getLocationSourceKindLabel(kind: LocationSourceKind): string {
  return locationSourceKindLabels[kind]
}

export function getLocationSourceIcon(kind: LocationSourceKind): LocationSourceIcon {
  return locationSourceIconMap[kind]
}

export function getLocationSourceIconBadge(
  descriptor: Pick<LocationSourceDescriptor, 'availability' | 'health' | 'role'>
): LocationSourceIconBadge | undefined {
  const { availability, health, role } = descriptor

  switch (availability) {
    case 'missing':
      return 'missing'
    case 'offline':
      return 'offline'
    case 'ejected':
      return 'offline'
    case 'permissionDenied':
      return 'locked'
    case 'stale':
      return 'stale'
    case 'scanning':
    case 'indexing':
      return 'scanning'
  }

  switch (health) {
    case 'warning':
    case 'degraded':
    case 'missingFiles':
    case 'unsupportedFormat':
      return 'warning'
    case 'readOnly':
    case 'permissionIssue':
      return 'readOnly'
  }

  if (role === 'primaryLibrary' || availability === 'available') {
    return 'persisted'
  }

  return undefined
}

export function getLocationSourcePresentation(
  descriptor: LocationSourceDescriptor
): LocationSourcePresentation {
  const badge = getLocationSourceIconBadge(descriptor)

  return {
    label: getLocationSourceKindLabel(descriptor.kind),
    icon: getLocationSourceIcon(descriptor.kind),
    ...(badge === undefined ? {} : { badge })
  }
}

export interface SourceAdapterInput {
  readonly rowKind: string
  readonly selectorKind: string | null
  readonly sourceStateKind?: string | undefined
  readonly sourceStateFailed?: boolean | undefined
}

const fallbackDescriptor: LocationSourceDescriptor = {
  kind: 'localFolder',
  medium: 'unknown',
  connectivity: 'internal',
  role: 'watchedRoot',
  availability: 'available',
  health: 'healthy'
}

export function adaptLocationSourceDescriptor(input: SourceAdapterInput): LocationSourceDescriptor {
  const kind = resolveSourceKindFromRow(input.rowKind, input.selectorKind)
  const availability = resolveAvailabilityFromSourceState(
    input.sourceStateKind,
    input.sourceStateFailed
  )
  const health = resolveHealthFromSourceState(input.sourceStateKind, input.sourceStateFailed)
  const role = resolveRoleFromRowKind(input.rowKind)

  return {
    kind,
    medium: 'unknown',
    connectivity: 'internal',
    role,
    availability,
    health
  }
}

function resolveSourceKindFromRow(
  rowKind: string,
  selectorKind: string | null
): LocationSourceKind {
  if (rowKind === 'source' && selectorKind === 'source') {
    return 'localLibrary'
  }
  if (rowKind === 'location' && selectorKind === 'sourceLocation') {
    return 'localFolder'
  }
  if (rowKind === 'source') {
    return 'localLibrary'
  }
  if (rowKind === 'location') {
    return 'localFolder'
  }

  return fallbackDescriptor.kind
}

function resolveAvailabilityFromSourceState(
  stateKind: string | undefined,
  failed: boolean | undefined
): LocationSourceAvailability {
  if (failed) {
    return 'missing'
  }
  if (stateKind === 'loading') {
    return 'scanning'
  }
  if (stateKind === 'unloaded') {
    return 'scanning'
  }

  return 'available'
}

function resolveHealthFromSourceState(
  _stateKind: string | undefined,
  failed: boolean | undefined
): LocationSourceHealth {
  if (failed) {
    return 'warning'
  }

  return 'healthy'
}

function resolveRoleFromRowKind(rowKind: string): LocationSourceRole {
  if (rowKind === 'source') {
    return 'primaryLibrary'
  }

  return 'watchedRoot'
}

const windowsExtendedPrefix = '\\\\?\\'
const windowsExtendedUncPrefix = '\\\\?\\UNC\\'

export function formatSourceDisplayName(rawLabel: string): string {
  if (rawLabel === '') {
    return rawLabel
  }

  let parseInput = rawLabel

  if (parseInput.startsWith(windowsExtendedUncPrefix)) {
    parseInput = '\\\\' + parseInput.slice(windowsExtendedUncPrefix.length)
  } else if (parseInput.startsWith(windowsExtendedPrefix)) {
    parseInput = parseInput.slice(windowsExtendedPrefix.length)
  }

  const segments = parseInput.split(/[/\\]/)

  for (let i = segments.length - 1; i >= 0; --i) {
    const segment = segments[i]
    if (segment !== '' && segment !== undefined) {
      return segment
    }
  }

  return rawLabel
}

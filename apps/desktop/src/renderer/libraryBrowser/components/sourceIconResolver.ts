import type { LocationSourceIcon, LocationSourceIconBadge } from '../projection/sourcePresentation'
import type { IconComponent } from '../../icons/types'
import {
  ArchiveIcon,
  CircleCheckIcon,
  CircleDotIcon,
  CircleOffIcon,
  CircleXIcon,
  CloudIcon,
  CloudSyncIcon,
  DiscIcon,
  FolderIcon,
  HardDriveIcon,
  LockIcon,
  NetworkIcon,
  RefreshCwIcon,
  ScanIcon,
  ServerIcon,
  SmartphoneIcon,
  SourceIcon,
  TriangleAlertIcon,
  UsbIcon
} from '../../icons/lucide'
import { SdCardIcon } from '../../icons/custom'

export function resolveSourceIcon(icon: LocationSourceIcon): IconComponent {
  switch (icon) {
    case 'library':
      return SourceIcon
    case 'folder':
      return FolderIcon
    case 'drive':
      return HardDriveIcon
    case 'driveInternal':
      return HardDriveIcon
    case 'driveExternal':
      return HardDriveIcon
    case 'usbDrive':
      return UsbIcon
    case 'sdCard':
      return SdCardIcon
    case 'networkShare':
      return NetworkIcon
    case 'nas':
      return ServerIcon
    case 'cloud':
      return CloudIcon
    case 'cloudMirror':
      return CloudSyncIcon
    case 'mobileDevice':
      return SmartphoneIcon
    case 'djDevice':
      return DiscIcon
    case 'disc':
      return DiscIcon
    case 'archive':
      return ArchiveIcon
    case 'missingSource':
      return TriangleAlertIcon
  }
}

export function resolveSourceIconBadge(
  badge: LocationSourceIconBadge | undefined
): IconComponent | undefined {
  switch (badge) {
    case 'available':
      return CircleCheckIcon
    case 'persisted':
      return CircleDotIcon
    case 'syncing':
      return RefreshCwIcon
    case 'stale':
      return TriangleAlertIcon
    case 'offline':
      return CircleOffIcon
    case 'missing':
      return CircleXIcon
    case 'warning':
      return TriangleAlertIcon
    case 'locked':
      return LockIcon
    case 'readOnly':
      return LockIcon
    case 'scanning':
      return ScanIcon
    default:
      return undefined
  }
}

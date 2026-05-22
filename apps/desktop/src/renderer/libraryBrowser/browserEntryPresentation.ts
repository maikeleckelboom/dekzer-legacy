export type LibraryEntryRole =
  | 'folder'
  | 'audio'
  | 'video'
  | 'cueSheet'
  | 'playlist'
  | 'artwork'
  | 'metadata'
  | 'unknown'
  | 'nonMedia'

export type BrowserEntryVisibility = 'defaultVisible' | 'companionVisible' | 'hiddenNonMedia'

export interface BrowserEntryPresentation {
  readonly role: LibraryEntryRole
  readonly visibility: BrowserEntryVisibility
  readonly badgeLabel: string
}

const audioExtensions = new Set([
  '.mp3',
  '.flac',
  '.wav',
  '.aiff',
  '.aif',
  '.m4a',
  '.aac',
  '.ogg',
  '.opus',
  '.wma',
  '.alac'
])

const videoExtensions = new Set(['.mp4', '.mov', '.mkv', '.avi', '.webm', '.m4v'])

const cueSheetExtensions = new Set(['.cue'])

const playlistExtensions = new Set(['.m3u', '.m3u8', '.pls'])

const artworkExtensions = new Set(['.jpg', '.jpeg', '.png', '.webp'])

const metadataExtensions = new Set(['.nfo'])

const hiddenSystemExtensions = new Set([
  '.bin',
  '.dll',
  '.exe',
  '.tmp',
  '.temp',
  '.log',
  '.ini',
  '.db',
  '.' + 'sq' + 'lite',
  '.bak',
  '.old',
  '.part',
  '.crdownload',
  '.lnk',
  '.sys',
  '.dat',
  '.idx',
  '.xml'
])

const hiddenSystemFullNames = new Set([
  '.ds_store',
  'thumbs.db',
  'desktop.ini',
  '.directory',
  '$recycle.bin',
  'autorun.inf',
  '.spotlight-v100',
  '.trashes',
  '.fseventsd',
  '.temporaryitems'
])

export function classifyLibraryEntryName(
  name: string,
  options?: { readonly isDirectory?: boolean }
): BrowserEntryPresentation {
  if (options?.isDirectory) {
    return { role: 'folder', visibility: 'defaultVisible', badgeLabel: 'Folder' }
  }

  const normalized = name.trim()
  const lower = normalized.toLowerCase()

  if (hiddenSystemFullNames.has(lower)) {
    return { role: 'nonMedia', visibility: 'hiddenNonMedia', badgeLabel: 'System' }
  }

  const lastDotIndex = normalized.lastIndexOf('.')

  if (lastDotIndex <= 0 || lastDotIndex === normalized.length - 1) {
    const role = lastDotIndex === 0 ? 'nonMedia' : 'unknown'

    return { role, visibility: 'hiddenNonMedia', badgeLabel: 'File' }
  }

  const ext = normalized.slice(lastDotIndex).toLowerCase()

  if (audioExtensions.has(ext)) {
    return { role: 'audio', visibility: 'defaultVisible', badgeLabel: 'Audio' }
  }

  if (videoExtensions.has(ext)) {
    return { role: 'video', visibility: 'defaultVisible', badgeLabel: 'Video' }
  }

  if (cueSheetExtensions.has(ext)) {
    return { role: 'cueSheet', visibility: 'defaultVisible', badgeLabel: 'Cue' }
  }

  if (playlistExtensions.has(ext)) {
    return { role: 'playlist', visibility: 'companionVisible', badgeLabel: 'Playlist' }
  }

  if (artworkExtensions.has(ext)) {
    return { role: 'artwork', visibility: 'companionVisible', badgeLabel: 'Artwork' }
  }

  if (metadataExtensions.has(ext)) {
    return { role: 'metadata', visibility: 'companionVisible', badgeLabel: 'Metadata' }
  }

  if (hiddenSystemExtensions.has(ext)) {
    return { role: 'nonMedia', visibility: 'hiddenNonMedia', badgeLabel: 'System' }
  }

  return { role: 'unknown', visibility: 'hiddenNonMedia', badgeLabel: 'File' }
}

export function isDefaultVisibleLibraryEntry(
  name: string,
  options?: { readonly isDirectory?: boolean }
): boolean {
  return classifyLibraryEntryName(name, options).visibility === 'defaultVisible'
}

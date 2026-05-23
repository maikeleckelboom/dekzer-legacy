import { strict as assert } from 'node:assert'

import {
  classifyLibraryEntryName,
  isDefaultVisibleLibraryEntry
} from '../src/renderer/libraryBrowser/projection/entryPresentation'

void main()

function main(): void {
  validatesFolderClassification()
  validatesAudioClassification()
  validatesVideoClassification()
  validatesCueSheetClassification()
  validatesPlaylistClassification()
  validatesArtworkClassification()
  validatesMetadataClassification()
  validatesHiddenSystemFiles()
  validatesHiddenSystemFullNames()
  validatesMultiDotFilenames()
  validatesCaseInsensitivity()
  validatesWhitespaceHandling()
  validatesUnknownFilesAreHidden()
  validatesDefaultVisibilityHelper()
  validatesNoExtensionFilesAreHidden()
}

function validatesFolderClassification(): void {
  const result = classifyLibraryEntryName('My Music', { isDirectory: true })

  assert.equal(result.role, 'folder')
  assert.equal(result.visibility, 'defaultVisible')
  assert.equal(isDefaultVisibleLibraryEntry('My Music', { isDirectory: true }), true)
}

function validatesAudioClassification(): void {
  const audioFiles = [
    'track.mp3',
    'song.flac',
    'recording.wav',
    'master.aiff',
    'sample.aif',
    'podcast.m4a',
    'voice.aac',
    'live.ogg',
    'ambient.opus',
    'old.wma',
    'lossless.alac'
  ] as const

  for (const name of audioFiles) {
    const result = classifyLibraryEntryName(name)

    assert.equal(result.role, 'audio', `Expected ${name} to be audio, got ${result.role}`)
    assert.equal(result.visibility, 'defaultVisible', `Expected ${name} to be defaultVisible`)
    assert.equal(isDefaultVisibleLibraryEntry(name), true, `Expected ${name} to be default visible`)
  }
}

function validatesVideoClassification(): void {
  const videoFiles = [
    'movie.mp4',
    'clip.mov',
    'video.mkv',
    'old.avi',
    'stream.webm',
    'itunes.m4v'
  ] as const

  for (const name of videoFiles) {
    const result = classifyLibraryEntryName(name)

    assert.equal(result.role, 'video', `Expected ${name} to be video, got ${result.role}`)
    assert.equal(result.visibility, 'defaultVisible', `Expected ${name} to be defaultVisible`)
    assert.equal(isDefaultVisibleLibraryEntry(name), true, `Expected ${name} to be default visible`)
  }
}

function validatesCueSheetClassification(): void {
  const result = classifyLibraryEntryName('album.cue')

  assert.equal(result.role, 'cueSheet')
  assert.equal(result.visibility, 'defaultVisible')
  assert.equal(isDefaultVisibleLibraryEntry('album.cue'), true)
}

function validatesPlaylistClassification(): void {
  const playlistFiles = ['mix.m3u', 'set.m3u8', 'radio.pls'] as const

  for (const name of playlistFiles) {
    const result = classifyLibraryEntryName(name)

    assert.equal(result.role, 'playlist', `Expected ${name} to be playlist, got ${result.role}`)
    assert.equal(result.visibility, 'companionVisible', `Expected ${name} to be companionVisible`)
  }
}

function validatesArtworkClassification(): void {
  const artworkFiles = ['cover.jpg', 'folder.png', 'artwork.webp', 'albumart.jpeg'] as const

  for (const name of artworkFiles) {
    const result = classifyLibraryEntryName(name)

    assert.equal(result.role, 'artwork', `Expected ${name} to be artwork, got ${result.role}`)
    assert.equal(result.visibility, 'companionVisible', `Expected ${name} to be companionVisible`)
  }
}

function validatesMetadataClassification(): void {
  const result = classifyLibraryEntryName('album.nfo')

  assert.equal(result.role, 'metadata')
  assert.equal(result.visibility, 'companionVisible')
}

function validatesHiddenSystemFiles(): void {
  const hiddenFiles = [
    'data.bin',
    'library.dll',
    'setup.exe',
    'temp.tmp',
    'cache.temp',
    'debug.log',
    'config.ini',
    'library.db',
    'storage.sqlite',
    'backup.bak',
    'archive.old',
    'download.part',
    'download.crdownload',
    'shortcut.lnk',
    'driver.sys',
    'index.dat',
    'index.idx',
    'metadata.xml'
  ] as const

  for (const name of hiddenFiles) {
    const result = classifyLibraryEntryName(name)

    assert.equal(
      result.visibility,
      'hiddenNonMedia',
      `Expected ${name} to be hiddenNonMedia, got ${result.visibility}`
    )
    assert.equal(
      isDefaultVisibleLibraryEntry(name),
      false,
      `Expected ${name} to NOT be default visible`
    )
  }
}

function validatesHiddenSystemFullNames(): void {
  const hiddenNames = [
    '.DS_Store',
    'Thumbs.db',
    'desktop.ini',
    '.directory',
    'autorun.inf',
    '.Spotlight-V100'
  ] as const

  for (const name of hiddenNames) {
    const result = classifyLibraryEntryName(name)

    assert.equal(
      result.visibility,
      'hiddenNonMedia',
      `Expected "${name}" to be hiddenNonMedia, got ${result.visibility}`
    )
    assert.equal(
      isDefaultVisibleLibraryEntry(name),
      false,
      `Expected "${name}" to NOT be default visible`
    )
  }
}

function validatesMultiDotFilenames(): void {
  assert.equal(classifyLibraryEntryName('artist - track.flac').role, 'audio')
  assert.equal(classifyLibraryEntryName('backup.tar.gz').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('cover.art.jpg').role, 'artwork')
  assert.equal(classifyLibraryEntryName('video.final.mp4').role, 'video')
  assert.equal(classifyLibraryEntryName('something.a.b.c.ini').visibility, 'hiddenNonMedia')
}

function validatesCaseInsensitivity(): void {
  assert.equal(classifyLibraryEntryName('TRACK.MP3').role, 'audio')
  assert.equal(classifyLibraryEntryName('Movie.MP4').role, 'video')
  assert.equal(classifyLibraryEntryName('Album.CUE').role, 'cueSheet')
  assert.equal(classifyLibraryEntryName('Playlist.M3U').role, 'playlist')
  assert.equal(classifyLibraryEntryName('Cover.JPG').role, 'artwork')
  assert.equal(classifyLibraryEntryName('Album.NFO').role, 'metadata')
  assert.equal(classifyLibraryEntryName('Data.BIN').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('Debug.LOG').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('Config.INI').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('Thumbs.DB').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('Desktop.INI').visibility, 'hiddenNonMedia')
}

function validatesWhitespaceHandling(): void {
  assert.equal(classifyLibraryEntryName('  track.mp3  ').role, 'audio')
  assert.equal(classifyLibraryEntryName('  album.cue  ').role, 'cueSheet')
  assert.equal(classifyLibraryEntryName('  cover.jpg  ').role, 'artwork')
}

function validatesUnknownFilesAreHidden(): void {
  const result = classifyLibraryEntryName('random.file')

  assert.equal(result.visibility, 'hiddenNonMedia')
  assert.equal(result.role, 'unknown')
  assert.equal(isDefaultVisibleLibraryEntry('random.file'), false)
}

function validatesDefaultVisibilityHelper(): void {
  assert.equal(isDefaultVisibleLibraryEntry('track.mp3'), true)
  assert.equal(isDefaultVisibleLibraryEntry('movie.mp4'), true)
  assert.equal(isDefaultVisibleLibraryEntry('album.cue'), true)
  assert.equal(isDefaultVisibleLibraryEntry('cover.jpg'), false)
  assert.equal(isDefaultVisibleLibraryEntry('mix.m3u'), false)
  assert.equal(isDefaultVisibleLibraryEntry('album.nfo'), false)
  assert.equal(isDefaultVisibleLibraryEntry('data.bin'), false)
  assert.equal(isDefaultVisibleLibraryEntry('Thumbs.db'), false)
  assert.equal(isDefaultVisibleLibraryEntry('random.xyz'), false)
}

function validatesNoExtensionFilesAreHidden(): void {
  assert.equal(classifyLibraryEntryName('README').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('LICENSE').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('.gitignore').visibility, 'hiddenNonMedia')
  assert.equal(classifyLibraryEntryName('.hidden').role, 'nonMedia')
}

import { test as base, expect } from '@playwright/test'
import { mkdir, rename, rm, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'

export type LibrarySourceAFixture = {
  readonly rootPath: string
  readonly rootTrackPath: string
  readonly nestedTrackPath: string
  readonly descendantOnlyTrackPath: string
  readonly coverPath: string
  readonly readmePath: string
  readonly emptyPath: string
}

export type LibrarySourceBFixture = {
  readonly rootPath: string
  readonly rootTrackPath: string
}

export type LibraryFilesystem = {
  readonly rootDir: string
  readonly userDataPath: string
  readonly mediaRootPath: string
  readonly albumPath: string
  readonly audioFilePath: string
  readonly sourceA: LibrarySourceAFixture
  readonly sourceB: LibrarySourceBFixture
  readonly createTinyWav: (path: string) => Promise<void>
  readonly createPagedAudioSource: (
    sourceName: string,
    trackCount: number
  ) => Promise<{ readonly rootPath: string }>
  readonly moveSourceOffline: (sourceRootPath: string) => Promise<string>
  readonly restoreOfflineSource: (sourceRootPath: string, offlinePath: string) => Promise<void>
}

export type FilesystemFixtures = {
  readonly libraryFilesystem: LibraryFilesystem
}

export const test = base.extend<FilesystemFixtures>({
  // Playwright fixture discovery requires object destructuring for the first argument.
  // eslint-disable-next-line no-empty-pattern
  libraryFilesystem: async ({}, use, testInfo) => {
    const rootDir = testInfo.outputPath('filesystem')
    const userDataPath = join(rootDir, 'user-data')
    const mediaRootPath = join(rootDir, 'media')
    const albumPath = join(mediaRootPath, 'Album A')
    const audioFilePath = join(albumPath, '01-tone.wav')
    const sourceA = sourceAFixture(mediaRootPath)
    const sourceB = sourceBFixture(mediaRootPath)

    await rm(rootDir, { recursive: true, force: true })
    await mkdir(albumPath, { recursive: true })
    await createTinyWav(audioFilePath)
    await createSourceAFixture(sourceA)
    await createSourceBFixture(sourceB)

    await use({
      rootDir,
      userDataPath,
      mediaRootPath,
      albumPath,
      audioFilePath,
      sourceA,
      sourceB,
      createTinyWav,
      createPagedAudioSource: (sourceName, trackCount) =>
        createPagedAudioSource(mediaRootPath, sourceName, trackCount),
      moveSourceOffline,
      restoreOfflineSource
    })
  }
})

export { expect }

async function createTinyWav(path: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true })
  await writeFile(path, tinyWavBuffer())
}

async function createSourceAFixture(sourceA: LibrarySourceAFixture): Promise<void> {
  await createTinyWav(sourceA.rootTrackPath)
  await createTinyWav(sourceA.nestedTrackPath)
  await createTinyWav(sourceA.descendantOnlyTrackPath)
  await mkdir(dirname(sourceA.coverPath), { recursive: true })
  await writeFile(sourceA.coverPath, tinyJpegBuffer())
  await writeFile(sourceA.readmePath, 'Generated Library V0 smoke fixture.\n')
  await mkdir(sourceA.emptyPath, { recursive: true })
}

async function createSourceBFixture(sourceB: LibrarySourceBFixture): Promise<void> {
  await createTinyWav(sourceB.rootTrackPath)
}

async function createPagedAudioSource(
  mediaRootPath: string,
  sourceName: string,
  trackCount: number
): Promise<{ readonly rootPath: string }> {
  const rootPath = join(mediaRootPath, sourceName)

  await rm(rootPath, { recursive: true, force: true })
  await mkdir(rootPath, { recursive: true })

  for (let index = 1; index <= trackCount; index += 1) {
    await createTinyWav(join(rootPath, `Track ${String(index).padStart(3, '0')}.wav`))
  }

  return { rootPath }
}

async function moveSourceOffline(sourceRootPath: string): Promise<string> {
  const offlinePath = `${sourceRootPath}.offline`

  await rm(offlinePath, { recursive: true, force: true })
  await mkdir(dirname(offlinePath), { recursive: true })
  await rename(sourceRootPath, offlinePath)

  return offlinePath
}

async function restoreOfflineSource(sourceRootPath: string, offlinePath: string): Promise<void> {
  await rm(sourceRootPath, { recursive: true, force: true })
  await mkdir(dirname(sourceRootPath), { recursive: true })
  await rename(offlinePath, sourceRootPath)
}

function sourceAFixture(mediaRootPath: string): LibrarySourceAFixture {
  const rootPath = join(mediaRootPath, 'source-a')

  return {
    rootPath,
    rootTrackPath: join(rootPath, 'Root Track A.wav'),
    nestedTrackPath: join(rootPath, 'nested', 'Nested Track A.wav'),
    descendantOnlyTrackPath: join(rootPath, 'descendants-only', 'deeper', 'Descendant Only A.wav'),
    coverPath: join(rootPath, 'mixed', 'cover.jpg'),
    readmePath: join(rootPath, 'mixed', 'readme.txt'),
    emptyPath: join(rootPath, 'empty')
  }
}

function sourceBFixture(mediaRootPath: string): LibrarySourceBFixture {
  const rootPath = join(mediaRootPath, 'source-b')

  return {
    rootPath,
    rootTrackPath: join(rootPath, 'Only Track B.wav')
  }
}

function tinyWavBuffer(): Buffer {
  const channelCount = 1
  const sampleRate = 8_000
  const bitsPerSample = 16
  const sampleCount = 800
  const blockAlign = (channelCount * bitsPerSample) / 8
  const byteRate = sampleRate * blockAlign
  const dataSize = sampleCount * blockAlign
  const buffer = Buffer.alloc(44 + dataSize)

  buffer.write('RIFF', 0)
  buffer.writeUInt32LE(36 + dataSize, 4)
  buffer.write('WAVE', 8)
  buffer.write('fmt ', 12)
  buffer.writeUInt32LE(16, 16)
  buffer.writeUInt16LE(1, 20)
  buffer.writeUInt16LE(channelCount, 22)
  buffer.writeUInt32LE(sampleRate, 24)
  buffer.writeUInt32LE(byteRate, 28)
  buffer.writeUInt16LE(blockAlign, 32)
  buffer.writeUInt16LE(bitsPerSample, 34)
  buffer.write('data', 36)
  buffer.writeUInt32LE(dataSize, 40)

  for (let sample = 0; sample < sampleCount; sample += 1) {
    const amplitude = Math.round(Math.sin((sample / sampleRate) * 440 * Math.PI * 2) * 4_000)
    buffer.writeInt16LE(amplitude, 44 + sample * blockAlign)
  }

  return buffer
}

function tinyJpegBuffer(): Buffer {
  return Buffer.from([
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48,
    0x00, 0x48, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0x03, 0x02, 0x02, 0x03, 0x02, 0x02, 0x03,
    0x03, 0x03, 0x03, 0x04, 0x03, 0x03, 0x04, 0x05, 0x08, 0x05, 0x05, 0x04, 0x04, 0x05, 0x0a, 0x07,
    0x07, 0x06, 0x08, 0x0c, 0x0a, 0x0c, 0x0c, 0x0b, 0x0a, 0x0b, 0x0b, 0x0d, 0x0e, 0x12, 0x10, 0x0d,
    0x0e, 0x11, 0x0e, 0x0b, 0x0b, 0x10, 0x16, 0x10, 0x11, 0x13, 0x14, 0x15, 0x15, 0x15, 0x0c, 0x0f,
    0x17, 0x18, 0x16, 0x14, 0x18, 0x12, 0x14, 0x15, 0x14, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x00, 0x01,
    0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xff, 0xc4, 0x00, 0x14,
    0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x7f, 0xff, 0xd9
  ])
}

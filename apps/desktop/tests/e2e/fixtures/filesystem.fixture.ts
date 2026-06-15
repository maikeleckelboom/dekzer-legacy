import { test as base, expect } from '@playwright/test'
import { mkdir, rm, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'

export type LibraryFilesystem = {
  readonly rootDir: string
  readonly userDataPath: string
  readonly mediaRootPath: string
  readonly albumPath: string
  readonly audioFilePath: string
  readonly createTinyWav: (path: string) => Promise<void>
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

    await rm(rootDir, { recursive: true, force: true })
    await mkdir(albumPath, { recursive: true })
    await createTinyWav(audioFilePath)

    await use({
      rootDir,
      userDataPath,
      mediaRootPath,
      albumPath,
      audioFilePath,
      createTinyWav
    })
  }
})

export { expect }

async function createTinyWav(path: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true })
  await writeFile(path, tinyWavBuffer())
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

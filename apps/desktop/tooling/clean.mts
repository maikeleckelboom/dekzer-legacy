import { rm } from 'node:fs/promises'
import { dirname, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const toolingDirectory = dirname(fileURLToPath(import.meta.url))
const desktopRoot = resolve(toolingDirectory, '..')
const outDirectory = resolve(desktopRoot, 'out')

function assertInsideDesktopRoot(path: string): void {
  const relativePath = relative(desktopRoot, path)

  if (relativePath === '' || relativePath.startsWith('..') || relativePath.includes(':')) {
    throw new Error(`Refusing to remove path outside desktop root: ${path}`)
  }
}

assertInsideDesktopRoot(outDirectory)

await rm(outDirectory, {
  recursive: true,
  force: true
})

console.log(`Removed ${relative(desktopRoot, outDirectory)}`)

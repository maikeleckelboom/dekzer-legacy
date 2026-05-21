import { readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

export const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..')
export const workspaceRoot = resolve(desktopRoot, '..', '..')
export const sourceRoot = join(desktopRoot, 'src')
export const rendererSourceRoot = join(desktopRoot, 'src', 'renderer')

export function normalizePath(path: string): string {
  return path.split(sep).join('/')
}

export function listSourceFiles(root: string): readonly string[] {
  const files: string[] = []

  for (const entry of readdirSync(root)) {
    const entryPath = join(root, entry)
    const stats = statSync(entryPath)

    if (stats.isDirectory()) {
      files.push(...listSourceFiles(entryPath))
      continue
    }

    if (entryPath.endsWith('.ts') || entryPath.endsWith('.vue')) {
      files.push(entryPath)
    }
  }

  return files
}

export function readJson<T = unknown>(filePath: string): T {
  return JSON.parse(readFileSync(filePath, 'utf8')) as T
}

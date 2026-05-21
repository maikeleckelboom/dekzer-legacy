import { rmSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const outDir = resolve(__dirname, '..', 'out')

rmSync(outDir, { recursive: true, force: true })

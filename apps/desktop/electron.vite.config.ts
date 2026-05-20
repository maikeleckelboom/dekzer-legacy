import { resolve } from 'path'
import { defineConfig } from 'electron-vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  main: {
    resolve: {
      alias: {
        '@dekzer/library-boundary-client': resolve(
          '../../packages/library-boundary-client/src/index.ts'
        ),
        '@dekzer/library-boundary-contract': resolve(
          '../../packages/library-boundary-contract/index.ts'
        ),
        '@dekzer/library-boundary-stdio-transport': resolve(
          '../../packages/library-boundary-stdio-transport/src/index.ts'
        )
      }
    }
  },
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src')
      }
    },
    plugins: [vue()]
  }
})

import { describe, expect, it } from 'vitest'

import { createContentsReadController } from '../../../../src/renderer/library/boundary/contentsRead'
import type { RowBinding } from '../../../../src/renderer/library/state'
import type {
  ContentsReadRequest,
  ContentsReadResult
} from '../../../../src/shared/libraryContents/read'
import type { LibraryContentsApi } from '../../../../src/shared/rendererApi'

describe('createContentsReadController', () => {
  it('requests recursive audio-only source-file contents for selected directories', async () => {
    let capturedRequest: ContentsReadRequest | undefined
    const contentsApi: LibraryContentsApi = {
      async read(request): Promise<ContentsReadResult> {
        capturedRequest = request
        return {
          state: 'ready',
          result: {
            state: 'empty',
            scope: request.scope,
            policy: request.policy,
            recursion: request.recursion,
            rows: [],
            coverage: {
              state: 'complete',
              recursiveScopeComplete: true,
              emptyResultAuthoritative: true
            }
          }
        }
      }
    }
    const controller = createContentsReadController(contentsApi)

    controller.start()
    await controller.readForBinding(directoryBinding())

    expect(capturedRequest).toEqual({
      scope: {
        kind: 'directory',
        sourceId: '7',
        sourceDirectoryId: '11'
      },
      policy: {
        mediaClasses: ['audio'],
        rowProfile: { kind: 'sourceFile' }
      },
      recursion: 'recursive',
      limit: 100
    })
    expect(capturedRequest?.policy.mediaClasses).not.toContain('image')
    expect(capturedRequest?.policy.mediaClasses).not.toContain('video')
    expect(capturedRequest?.policy.mediaClasses).not.toContain('unsupported')
  })
})

function directoryBinding(): RowBinding {
  return {
    kind: 'directory',
    sourceId: '7',
    directoryId: '11',
    entryPoint: {
      kind: 'source',
      sourceId: '7'
    },
    label: 'Album'
  }
}

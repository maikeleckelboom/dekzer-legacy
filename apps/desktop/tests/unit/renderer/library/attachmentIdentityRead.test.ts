import { describe, expect, it } from 'vitest'

import {
  readAttachmentSourceFiles,
  readSourceAttachmentSummary,
  readSourceFileAttachment
} from '../../../../src/renderer/library/boundary/attachmentIdentityRead'
import type { LibraryAttachmentIdentityApi } from '../../../../src/renderer/library/boundary/attachmentIdentityRead'

describe('attachment identity renderer boundary', () => {
  it('forwards typed read requests without owning attachment state', async () => {
    const received: unknown[] = []
    const api: LibraryAttachmentIdentityApi = {
      readSourceFileAttachment: async (request) => {
        received.push(request)
        return {
          state: 'notFound',
          reply: {
            status: 'notFound'
          }
        }
      },
      readAttachmentSourceFiles: async (request) => {
        received.push(request)
        return {
          state: 'notFound',
          reply: {
            status: 'notFound',
            sourceFileLinks: [],
            effectiveLimit: 25,
            remainingSourceFileLinks: 0
          }
        }
      },
      readSourceAttachmentSummary: async (request) => {
        received.push(request)
        return {
          state: 'notFound',
          reply: {
            status: 'notFound'
          }
        }
      }
    }

    await expect(readSourceFileAttachment({ sourceFileId: '11' }, api)).resolves.toMatchObject({
      state: 'notFound'
    })
    await expect(
      readAttachmentSourceFiles({ attachmentId: '7', limit: 25 }, api)
    ).resolves.toMatchObject({
      state: 'notFound'
    })
    await expect(readSourceAttachmentSummary({ sourceId: '3' }, api)).resolves.toMatchObject({
      state: 'notFound'
    })
    expect(received).toEqual([
      { sourceFileId: '11' },
      { attachmentId: '7', limit: 25 },
      { sourceId: '3' }
    ])
  })
})

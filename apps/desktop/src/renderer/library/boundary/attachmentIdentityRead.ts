import type {
  ReadAttachmentSourceFilesRequest,
  ReadAttachmentSourceFilesResult,
  ReadSourceAttachmentSummaryRequest,
  ReadSourceAttachmentSummaryResult,
  ReadSourceFileAttachmentRequest,
  ReadSourceFileAttachmentResult
} from '../../../shared/libraryAttachmentIdentity/read'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibraryAttachmentIdentityApi = RendererApi['library']['attachmentIdentity']

export function readSourceFileAttachment(
  request: ReadSourceFileAttachmentRequest,
  attachmentIdentityApi: LibraryAttachmentIdentityApi = getRendererApi().library.attachmentIdentity
): Promise<ReadSourceFileAttachmentResult> {
  return attachmentIdentityApi.readSourceFileAttachment(request)
}

export function readAttachmentSourceFiles(
  request: ReadAttachmentSourceFilesRequest,
  attachmentIdentityApi: LibraryAttachmentIdentityApi = getRendererApi().library.attachmentIdentity
): Promise<ReadAttachmentSourceFilesResult> {
  return attachmentIdentityApi.readAttachmentSourceFiles(request)
}

export function readSourceAttachmentSummary(
  request: ReadSourceAttachmentSummaryRequest,
  attachmentIdentityApi: LibraryAttachmentIdentityApi = getRendererApi().library.attachmentIdentity
): Promise<ReadSourceAttachmentSummaryResult> {
  return attachmentIdentityApi.readSourceAttachmentSummary(request)
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

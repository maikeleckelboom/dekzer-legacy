import type {
  HashSourceFilesBlake3Request,
  HashSourceFilesBlake3Result
} from '../../../shared/librarySourceFileHashing/hashSourceFilesBlake3'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibrarySourceFileHashingApi = RendererApi['library']['hashing']

export function hashSourceFilesBlake3(
  request: HashSourceFilesBlake3Request,
  hashingApi: LibrarySourceFileHashingApi = getRendererApi().library.hashing
): Promise<HashSourceFilesBlake3Result> {
  return hashingApi.hashSourceFilesBlake3(request)
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

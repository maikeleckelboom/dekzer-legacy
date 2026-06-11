import type {
  SearchFilterReadRequest,
  SearchFilterReadResult
} from '../../../shared/library/searchFilter/read'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibrarySearchFilterApi = RendererApi['library']['searchFilter']

export function readSearchFilter(
  request: SearchFilterReadRequest,
  searchFilterApi: LibrarySearchFilterApi = getRendererApi().library.searchFilter
): Promise<SearchFilterReadResult> {
  return searchFilterApi.read(request)
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

import type {
  ReadSourceMaintenanceRequest,
  ReadSourceMaintenanceResult,
  RunSourceMaintenanceRequest,
  RunSourceMaintenanceResult
} from '../../../shared/library/source/maintenance'
import type { RendererApi } from '../../../shared/rendererApi'

export type LibrarySourceMaintenanceApi = RendererApi['library']['sourceMaintenance']

export function runSourceMaintenance(
  request: RunSourceMaintenanceRequest,
  sourceMaintenanceApi: LibrarySourceMaintenanceApi = getRendererApi().library.sourceMaintenance
): Promise<RunSourceMaintenanceResult> {
  return sourceMaintenanceApi.runSourceMaintenance(request)
}

export function readSourceMaintenance(
  request: ReadSourceMaintenanceRequest,
  sourceMaintenanceApi: LibrarySourceMaintenanceApi = getRendererApi().library.sourceMaintenance
): Promise<ReadSourceMaintenanceResult> {
  return sourceMaintenanceApi.readSourceMaintenance(request)
}

function getRendererApi(): RendererApi {
  return (window as unknown as { readonly dekzer: RendererApi }).dekzer
}

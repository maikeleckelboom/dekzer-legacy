import { onMounted, onUnmounted, ref } from 'vue'
import type { Ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundaryStatus'
import type { LibraryHierarchyReadResult } from '../../../shared/libraryHierarchyRead'

export function useLibraryHierarchyRead(): {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | null>
  readonly hierarchyReadResult: Ref<LibraryHierarchyReadResult | null>
  readonly hierarchyReadRequestError: Ref<string | null>
  readonly hierarchyReadIsLoading: Ref<boolean>
} {
  const hostStatus = ref<LibraryBoundaryHostStatus | null>(null)
  const hierarchyReadResult = ref<LibraryHierarchyReadResult | null>(null)
  const hierarchyReadRequestError = ref<string | null>(null)
  const hierarchyReadIsLoading = ref(false)
  let hasRequestedHierarchyRead = false
  let unsubscribeFromHostStatus: (() => void) | null = null

  onMounted(() => {
    void window.dekzer.libraryBoundary
      .getStatus()
      .then((status) => {
        hostStatus.value = status
        hierarchyReadRequestError.value = null
        requestHierarchyReadIfStarted(status)
      })
      .catch(() => {
        hierarchyReadRequestError.value = 'Unable to read library boundary host status.'
      })

    unsubscribeFromHostStatus = window.dekzer.libraryBoundary.onStatusChanged((status) => {
      hostStatus.value = status
      hierarchyReadRequestError.value = null
      requestHierarchyReadIfStarted(status)
    })
  })

  onUnmounted(() => {
    unsubscribeFromHostStatus?.()
    unsubscribeFromHostStatus = null
  })

  function requestHierarchyReadIfStarted(status: LibraryBoundaryHostStatus): void {
    if (status.state !== 'started' || hasRequestedHierarchyRead) {
      return
    }

    hasRequestedHierarchyRead = true
    void readFirstAvailableSourceHierarchy()
  }

  async function readFirstAvailableSourceHierarchy(): Promise<void> {
    hierarchyReadIsLoading.value = true
    hierarchyReadRequestError.value = null

    try {
      hierarchyReadResult.value = await window.dekzer.libraryBoundary.readLiteralHierarchyChildren({
        target: {
          kind: 'firstAvailableSource'
        },
        parentSourceDirectoryId: null,
        offset: 0,
        limit: 50
      })
    } catch {
      hierarchyReadRequestError.value = 'Unable to request library hierarchy children.'
    } finally {
      hierarchyReadIsLoading.value = false
    }
  }

  return {
    hostStatus,
    hierarchyReadResult,
    hierarchyReadRequestError,
    hierarchyReadIsLoading
  }
}

import { onMounted, onUnmounted, ref } from 'vue'
import type { Ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../../shared/libraryBoundaryStatus'
import type { LibraryHierarchyReadResult } from '../../../shared/libraryHierarchyRead'

export function useLibraryHierarchyRead(): {
  readonly hostStatus: Ref<LibraryBoundaryHostStatus | undefined>
  readonly hierarchyReadResult: Ref<LibraryHierarchyReadResult | undefined>
  readonly hierarchyReadRequestError: Ref<string | undefined>
  readonly hierarchyReadIsLoading: Ref<boolean>
} {
  const hostStatus = ref<LibraryBoundaryHostStatus>()
  const hierarchyReadResult = ref<LibraryHierarchyReadResult>()
  const hierarchyReadRequestError = ref<string>()
  const hierarchyReadIsLoading = ref(false)
  let hasRequestedHierarchyRead = false
  let unsubscribeFromHostStatus: (() => void) | undefined

  onMounted(() => {
    void window.dekzer.libraryBoundary
      .getStatus()
      .then((status) => {
        hostStatus.value = status
        hierarchyReadRequestError.value = undefined
        requestHierarchyReadIfStarted(status)
      })
      .catch(() => {
        hierarchyReadRequestError.value = 'Unable to read library boundary host status.'
      })

    unsubscribeFromHostStatus = window.dekzer.libraryBoundary.onStatusChanged((status) => {
      hostStatus.value = status
      hierarchyReadRequestError.value = undefined
      requestHierarchyReadIfStarted(status)
    })
  })

  onUnmounted(() => {
    unsubscribeFromHostStatus?.()
    unsubscribeFromHostStatus = undefined
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
    hierarchyReadRequestError.value = undefined

    try {
      hierarchyReadResult.value = await window.dekzer.libraryBoundary.readLiteralHierarchyChildren({
        target: {
          kind: 'firstAvailableSource'
        },
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

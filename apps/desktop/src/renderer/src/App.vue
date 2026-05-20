<template>
  <main class="shell" aria-labelledby="dekzer-title">
    <section class="identity">
      <p class="kicker">Dekzer Desktop</p>
      <h1 id="dekzer-title">Dekzer</h1>
      <p class="status inline-flex rounded-sm border border-white/10 px-3 py-2">
        Desktop app ready. Product wiring is not active yet.
      </p>
    </section>

    <section class="state" aria-label="Workspace state">
      <div>
        <span>Desktop app</span>
        <strong class="is-ready">Ready</strong>
      </div>
      <div>
        <span>Library boundary host</span>
        <strong :class="hostStateClass">{{ hostStateLabel }}</strong>
      </div>
      <div>
        <span>Boundary environment</span>
        <strong>{{ hostEnvironmentLabel }}</strong>
      </div>
      <div v-if="hostStatus?.lastError" class="error-row">
        <span>Startup error</span>
        <strong>{{ hostStatus.lastError.code }}: {{ hostStatus.lastError.message }}</strong>
      </div>
      <div>
        <span>Product wiring</span>
        <strong class="is-pending">Not active yet</strong>
      </div>
      <div>
        <span>Library UI</span>
        <strong class="is-pending">Not wired</strong>
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundaryStatus'

const hostStatus = ref<LibraryBoundaryHostStatus | null>(null)
let unsubscribeFromHostStatus: (() => void) | null = null

const hostStateLabel = computed(() => {
  if (hostStatus.value === null) {
    return 'Loading'
  }

  return formatHostState(hostStatus.value.state)
})

const hostEnvironmentLabel = computed(() => {
  if (hostStatus.value === null) {
    return 'Unknown'
  }

  return formatEnvironment(hostStatus.value.environment)
})

const hostStateClass = computed(() => {
  if (hostStatus.value?.state === 'started') {
    return 'is-ready'
  }

  if (hostStatus.value?.state === 'failed') {
    return 'is-failed'
  }

  return 'is-pending'
})

onMounted(() => {
  void window.desktop.libraryBoundary.getStatus().then((status) => {
    hostStatus.value = status
  })

  unsubscribeFromHostStatus = window.desktop.libraryBoundary.onStatusChanged((status) => {
    hostStatus.value = status
  })
})

onUnmounted(() => {
  unsubscribeFromHostStatus?.()
  unsubscribeFromHostStatus = null
})

function formatHostState(state: LibraryBoundaryHostStatus['state']): string {
  switch (state) {
    case 'idle':
      return 'Idle'
    case 'starting':
      return 'Starting'
    case 'started':
      return 'Started'
    case 'stopping':
      return 'Stopping'
    case 'stopped':
      return 'Stopped'
    case 'failed':
      return 'Failed'
  }
}

function formatEnvironment(environment: LibraryBoundaryHostStatus['environment']): string {
  switch (environment) {
    case 'development':
      return 'Development'
    case 'production':
      return 'Production'
  }
}
</script>

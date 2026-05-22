<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../shared/libraryBoundary/status'
import LibraryBrowserPanel from './libraryBrowser/panel.vue'

const hostStatus = ref<LibraryBoundaryHostStatus>()
const hostStatusLoadError = ref<string>()
let unsubscribeFromHostStatus: (() => void) | undefined
const stateRowClass =
  'grid gap-1.5 border-b border-(--color-border) px-5 py-[18px] last:border-b-0 min-[861px]:grid-cols-[170px_1fr] min-[861px]:gap-6'
const stateLabelClass = 'text-[13px] text-(--color-text-muted)'
const stateValueClass = 'text-[15px] font-[650] text-(--color-text)'

const hostStateLabel = computed(() => {
  if (hostStatus.value === undefined) {
    return 'Loading'
  }

  return formatHostState(hostStatus.value.state)
})

const hostEnvironmentLabel = computed(() => {
  if (hostStatus.value === undefined) {
    return 'Unknown'
  }

  return formatEnvironment(hostStatus.value.environment)
})

const hostStateClass = computed(() => {
  if (hostStatus.value?.state === 'started') {
    return 'text-(--color-accent)'
  }

  if (hostStatus.value?.state === 'failed') {
    return 'text-(--color-danger)'
  }

  return 'text-(--color-warning)'
})

onMounted(() => {
  void window.dekzer.library.host
    .getStatus()
    .then((status) => {
      hostStatus.value = status
      hostStatusLoadError.value = undefined
    })
    .catch(() => {
      hostStatusLoadError.value = 'Unable to read library boundary host status.'
    })

  unsubscribeFromHostStatus = window.dekzer.library.host.onStatusChanged((status) => {
    hostStatus.value = status
    hostStatusLoadError.value = undefined
  })
})

onUnmounted(() => {
  unsubscribeFromHostStatus?.()
  unsubscribeFromHostStatus = undefined
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

<template>
  <main
    class="grid h-svh overflow-y-hidden gap-8 overflow-auto bg-(--color-background) p-8 text-(--color-text)"
    aria-labelledby="dekzer-title"
  >
    <div class="grid gap-5 self-center">
      <section
        class="border border-(--color-border) bg-(--color-surface)"
        aria-label="Workspace state"
      >
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Desktop app</span>
          <strong :class="[stateValueClass, 'text-(--color-accent)']">Ready</strong>
        </div>
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Library boundary host</span>
          <strong :class="[stateValueClass, hostStateClass]">{{ hostStateLabel }}</strong>
        </div>
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Boundary environment</span>
          <strong :class="stateValueClass">{{ hostEnvironmentLabel }}</strong>
        </div>
        <div v-if="hostStatus?.lastError" :class="stateRowClass">
          <span :class="stateLabelClass">Startup error</span>
          <strong :class="[stateValueClass, 'wrap-anywhere text-(--color-danger)']">
            {{ hostStatus.lastError.code }}: {{ hostStatus.lastError.message }}
          </strong>
        </div>
        <div v-if="hostStatusLoadError" :class="stateRowClass">
          <span :class="stateLabelClass">Status bridge</span>
          <strong :class="[stateValueClass, 'wrap-anywhere text-(--color-danger)']">
            {{ hostStatusLoadError }}
          </strong>
        </div>
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Product wiring</span>
          <strong :class="[stateValueClass, 'text-(--color-accent)']"
            >Library browser active</strong
          >
        </div>
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Library UI</span>
          <strong :class="[stateValueClass, 'text-(--color-accent)']">
            Persisted navigation
          </strong>
        </div>
      </section>

      <LibraryBrowserPanel />
    </div>
  </main>
</template>

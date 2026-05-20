<template>
  <main
    class="grid min-h-screen max-h-screen gap-8 overflow-auto bg-(--color-background) p-8 text-(--color-text) max-[1060px]:max-h-none min-[861px]:p-14 min-[1061px]:grid-cols-[minmax(280px,0.8fr)_minmax(480px,1.2fr)]"
    aria-labelledby="dekzer-title"
  >
    <section class="self-center">
      <p class="mb-3 text-[13px] font-bold uppercase tracking-normal text-(--color-accent)">
        Dekzer Desktop
      </p>
      <h1
        id="dekzer-title"
        class="mb-[18px] text-5xl font-extrabold leading-none min-[861px]:text-[64px]"
      >
        Dekzer
      </h1>
      <p
        class="inline-flex max-w-[460px] rounded-sm border border-white/10 px-3 py-2 text-lg text-(--color-text-muted)"
      >
        Desktop app ready. Product wiring is not active yet.
      </p>
    </section>

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
          <strong :class="[stateValueClass, 'text-(--color-warning)']">Not active yet</strong>
        </div>
        <div :class="stateRowClass">
          <span :class="stateLabelClass">Library UI</span>
          <strong :class="[stateValueClass, 'text-(--color-warning)']">
            Read-only path guarded
          </strong>
        </div>
      </section>

      <LibraryBrowserPanel />
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'

import type { LibraryBoundaryHostStatus } from '../../shared/libraryBoundaryStatus'
import LibraryBrowserPanel from './libraryBrowser/libraryBrowserPanel.vue'

const hostStatus = ref<LibraryBoundaryHostStatus | null>(null)
const hostStatusLoadError = ref<string | null>(null)
let unsubscribeFromHostStatus: (() => void) | null = null
const stateRowClass =
  'grid gap-1.5 border-b border-(--color-border) px-5 py-[18px] last:border-b-0 min-[861px]:grid-cols-[170px_1fr] min-[861px]:gap-6'
const stateLabelClass = 'text-[13px] text-(--color-text-muted)'
const stateValueClass = 'text-[15px] font-[650] text-(--color-text)'

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
    return 'text-(--color-accent)'
  }

  if (hostStatus.value?.state === 'failed') {
    return 'text-(--color-danger)'
  }

  return 'text-(--color-warning)'
})

onMounted(() => {
  void window.dekzer.libraryBoundary
    .getStatus()
    .then((status) => {
      hostStatus.value = status
      hostStatusLoadError.value = null
    })
    .catch(() => {
      hostStatusLoadError.value = 'Unable to read library boundary host status.'
    })

  unsubscribeFromHostStatus = window.dekzer.libraryBoundary.onStatusChanged((status) => {
    hostStatus.value = status
    hostStatusLoadError.value = null
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

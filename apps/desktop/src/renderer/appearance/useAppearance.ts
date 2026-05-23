import { computed, ref } from 'vue'
import type { ComputedRef } from 'vue'
import type { AppearancePreference, ResolvedAppearance } from './types'
import {
  getAppearanceSnapshot,
  subscribeAppearance,
  setAppearancePreference,
  cycleAppearancePreference
} from './controller'

const appearancePreference = ref<AppearancePreference>('system')
const resolvedAppearance = ref<ResolvedAppearance>('dark')

// Initialize refs from controller snapshot on first composable use
function initializeFromSnapshot(): void {
  const snapshot = getAppearanceSnapshot()
  appearancePreference.value = snapshot.preference
  resolvedAppearance.value = snapshot.resolved
}

// Subscribe to controller state changes
let unsubscribe: (() => void) | null = null

function setupSubscription(): void {
  if (unsubscribe !== null) {
    return // Already subscribed
  }

  unsubscribe = subscribeAppearance((snapshot) => {
    appearancePreference.value = snapshot.preference
    resolvedAppearance.value = snapshot.resolved
  })
}

// Track if subscription has been setup
let subscriptionSetup = false

export function useAppearance(): {
  preference: ComputedRef<AppearancePreference>
  resolved: ComputedRef<ResolvedAppearance>
  isDark: ComputedRef<boolean>
  isLight: ComputedRef<boolean>
  setPreference: (preference: AppearancePreference) => void
  cycle: () => AppearancePreference
} {
  if (!subscriptionSetup) {
    initializeFromSnapshot()
    setupSubscription()
    subscriptionSetup = true
  }

  const preference = computed(() => appearancePreference.value)
  const resolved = computed(() => resolvedAppearance.value)
  const isDark = computed(() => resolvedAppearance.value === 'dark')
  const isLight = computed(() => resolvedAppearance.value === 'light')

  return {
    preference,
    resolved,
    isDark,
    isLight,
    setPreference: setAppearancePreference,
    cycle: cycleAppearancePreference
  }
}

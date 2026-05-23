import type { AppearancePreference, ResolvedAppearance } from './types'
import { readStoredAppearancePreference, writeStoredAppearancePreference } from './storage'

export type AppearanceSnapshot = {
  readonly preference: AppearancePreference
  readonly resolved: ResolvedAppearance
}

let currentPreference: AppearancePreference = 'system'
let currentResolved: ResolvedAppearance = 'dark'
let mediaQuery: MediaQueryList | null = null
let mediaQueryListener: ((e: MediaQueryListEvent) => void) | null = null
const subscribers = new Set<(snapshot: AppearanceSnapshot) => void>()

function resolveAppearance(preference: AppearancePreference): ResolvedAppearance {
  if (preference === 'light' || preference === 'dark') {
    return preference
  }

  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function publish(snapshot: AppearanceSnapshot): void {
  // Apply to DOM (output projection)
  document.documentElement.dataset.appearancePreference = snapshot.preference
  document.documentElement.dataset.appearance = snapshot.resolved

  // Notify subscribers
  subscribers.forEach((listener) => listener(snapshot))
}

function setupSystemMediaQueryListener(): void {
  // Remove existing listener if any
  if (mediaQuery !== null && mediaQueryListener !== null) {
    mediaQuery.removeEventListener('change', mediaQueryListener)
    mediaQueryListener = null
  }

  // Only listen if preference is 'system'
  if (currentPreference === 'system') {
    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

    mediaQueryListener = (e: MediaQueryListEvent) => {
      const newResolved = e.matches ? 'dark' : 'light'
      currentResolved = newResolved

      // Publish only if preference is still 'system'
      if (currentPreference === 'system') {
        publish({
          preference: currentPreference,
          resolved: currentResolved
        })
      }
    }

    mediaQuery.addEventListener('change', mediaQueryListener)
  } else {
    mediaQuery = null
  }
}

export function initializeAppearance(): void {
  // Read stored preference
  currentPreference = readStoredAppearancePreference()

  // Resolve appearance
  currentResolved = resolveAppearance(currentPreference)

  // Publish initial state
  publish({
    preference: currentPreference,
    resolved: currentResolved
  })

  // Setup system media query listener if needed
  setupSystemMediaQueryListener()
}

export function getAppearanceSnapshot(): AppearanceSnapshot {
  return {
    preference: currentPreference,
    resolved: currentResolved
  }
}

export function subscribeAppearance(listener: (snapshot: AppearanceSnapshot) => void): () => void {
  subscribers.add(listener)

  // Return unsubscribe function
  return () => {
    subscribers.delete(listener)
  }
}

export function getCurrentAppearancePreference(): AppearancePreference {
  return currentPreference
}

export function getCurrentResolvedAppearance(): ResolvedAppearance {
  return currentResolved
}

export function setAppearancePreference(preference: AppearancePreference): void {
  currentPreference = preference

  // Resolve new appearance
  currentResolved = resolveAppearance(preference)

  // Persist to storage
  writeStoredAppearancePreference(preference)

  // Publish state change
  publish({
    preference: currentPreference,
    resolved: currentResolved
  })

  // Update system media query listener
  setupSystemMediaQueryListener()
}

export function cycleAppearancePreference(): AppearancePreference {
  const cycle: AppearancePreference[] = ['system', 'light', 'dark']
  const currentIndex = cycle.indexOf(currentPreference)
  const nextIndex = ((currentIndex >= 0 ? currentIndex : 0) + 1) % cycle.length
  const nextPreference = cycle[nextIndex] ?? 'system'

  setAppearancePreference(nextPreference)

  return nextPreference
}

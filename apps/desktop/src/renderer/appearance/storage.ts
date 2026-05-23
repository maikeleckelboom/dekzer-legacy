import type { AppearancePreference } from './types'

const APPEARANCE_PREFERENCE_STORAGE_KEY = 'dekzer.appearance.preference'

export function readStoredAppearancePreference(): AppearancePreference {
  const value = window.localStorage.getItem(APPEARANCE_PREFERENCE_STORAGE_KEY)

  if (value === 'light' || value === 'dark' || value === 'system') {
    return value
  }

  return 'system'
}

export function writeStoredAppearancePreference(preference: AppearancePreference): void {
  window.localStorage.setItem(APPEARANCE_PREFERENCE_STORAGE_KEY, preference)
}

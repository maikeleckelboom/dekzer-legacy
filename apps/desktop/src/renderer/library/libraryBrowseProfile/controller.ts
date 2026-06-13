import { ref } from 'vue'
import type { Ref } from 'vue'

import {
  defaultLibraryBrowseProfile,
  isLibraryBrowseProfile,
  type LibraryBrowseProfile
} from './types'

export type LibraryBrowseProfileController = {
  readonly profile: Ref<LibraryBrowseProfile>
  readonly setProfile: (profile: LibraryBrowseProfile) => boolean
  readonly restoreProfile: (profile: unknown) => boolean
}

export function createLibraryBrowseProfileController(
  initialProfile: LibraryBrowseProfile = defaultLibraryBrowseProfile
): LibraryBrowseProfileController {
  const profile = ref<LibraryBrowseProfile>(initialProfile)

  function setProfile(nextProfile: LibraryBrowseProfile): boolean {
    if (profile.value === nextProfile) {
      return false
    }

    profile.value = nextProfile
    return true
  }

  function restoreProfile(value: unknown): boolean {
    return isLibraryBrowseProfile(value) ? setProfile(value) : false
  }

  return {
    profile,
    setProfile,
    restoreProfile
  }
}

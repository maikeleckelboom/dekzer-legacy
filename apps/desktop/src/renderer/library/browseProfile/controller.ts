import { ref } from 'vue'
import type { Ref } from 'vue'

import { defaultProfile, isProfileKey, type ProfileKey } from './types'

export type ProfileController = {
  readonly profile: Ref<ProfileKey>
  readonly setProfile: (profile: ProfileKey) => boolean
  readonly restoreProfile: (profile: unknown) => boolean
}

export function createProfileController(
  initialProfile: ProfileKey = defaultProfile
): ProfileController {
  const profile = ref<ProfileKey>(initialProfile)

  function setProfile(nextProfile: ProfileKey): boolean {
    if (profile.value === nextProfile) {
      return false
    }

    profile.value = nextProfile
    return true
  }

  function restoreProfile(value: unknown): boolean {
    return isProfileKey(value) ? setProfile(value) : false
  }

  return {
    profile,
    setProfile,
    restoreProfile
  }
}

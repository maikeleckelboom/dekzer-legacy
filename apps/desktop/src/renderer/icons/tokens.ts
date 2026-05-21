import type { IconSize, IconTone } from './types'

export const iconSizeClass: Record<IconSize, string> = {
  xs: 'app-icon--xs',
  sm: 'app-icon--sm',
  md: 'app-icon--md',
  lg: 'app-icon--lg'
}

export const iconToneClass: Record<IconTone, string> = {
  inherit: 'app-icon--inherit',
  muted: 'app-icon--muted',
  primary: 'app-icon--primary',
  warning: 'app-icon--warning',
  danger: 'app-icon--danger'
}

export type IconRole =
  | 'disclosure.closed'
  | 'disclosure.open'
  | 'folder.plain'
  | 'source.local'
  | 'media.audio'
  | 'media.video'
  | 'media.image'
  | 'media.cueSheet'
  | 'media.playlist'
  | 'media.metadata'
  | 'state.loading'
  | 'state.warning'
  | 'state.unknown'
  | 'action.more'
  | 'action.scan'
  | 'action.remove'
  | 'navigation.collection'
  | 'navigation.view'

export type IconSize = 'xs' | 'sm' | 'md' | 'lg'

export type IconTone = 'inherit' | 'muted' | 'primary' | 'warning' | 'danger'

export type IconFrame = 'none' | 'square'

type HiddenIconProps = {
  role: IconRole
  size?: IconSize
  tone?: IconTone
  frame?: IconFrame
  accessibility?: 'hidden'
  label?: never
}

type LabeledIconProps = {
  role: IconRole
  size?: IconSize
  tone?: IconTone
  frame?: IconFrame
  accessibility: 'labelled'
  label: string
}

export type IconProps = HiddenIconProps | LabeledIconProps

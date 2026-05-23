import type { Component } from 'vue'
import type { LucideIcon } from '@lucide/vue'

export type IconComponent = LucideIcon | Component

export type IconSize = 'xs' | 'sm' | 'md' | 'lg'

export type IconTone = 'inherit' | 'muted' | 'primary' | 'warning' | 'danger'

export type IconFrame = 'none' | 'square'

type HiddenIconProps = {
  icon: IconComponent
  size?: IconSize
  tone?: IconTone
  frame?: IconFrame
  accessibility?: 'hidden'
  label?: never
}

type LabeledIconProps = {
  icon: IconComponent
  size?: IconSize
  tone?: IconTone
  frame?: IconFrame
  accessibility: 'labelled'
  label: string
}

export type IconProps = HiddenIconProps | LabeledIconProps

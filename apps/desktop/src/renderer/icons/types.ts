import type { Component } from 'vue'
import type { LucideIcon } from '@lucide/vue'

export type IconComponent = LucideIcon | Component

export type IconSize = 'xs' | 'sm' | 'md' | 'lg'

export type IconTone = 'inherit' | 'muted' | 'primary' | 'warning' | 'danger'

type DecorativeIconProps = {
  icon: IconComponent
  size?: IconSize
  tone?: IconTone
  decorative?: true
  label?: never
}

type LabeledIconProps = {
  icon: IconComponent
  size?: IconSize
  tone?: IconTone
  decorative: false
  label: string
}

export type IconProps = DecorativeIconProps | LabeledIconProps

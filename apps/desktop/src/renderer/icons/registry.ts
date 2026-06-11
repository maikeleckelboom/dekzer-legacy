import { defineComponent, h, type Component } from 'vue'

import chevronDown20Regular from './vendor/chevron_down_20_regular.svg?raw'
import chevronRight20Regular from './vendor/chevron_right_20_regular.svg?raw'
import folder20Regular from './vendor/folder_20_regular.svg?raw'
import library20Regular from './vendor/library_20_regular.svg?raw'
import musicNote220Regular from './vendor/music_note_2_20_regular.svg?raw'
import video20Regular from './vendor/video_20_regular.svg?raw'
import image20Regular from './vendor/image_20_regular.svg?raw'
import documentText20Regular from './vendor/document_text_20_regular.svg?raw'
import listBar20Regular from './vendor/list_bar_20_regular.svg?raw'
import spinnerIos20Regular from './vendor/spinner_ios_20_regular.svg?raw'
import warning20Regular from './vendor/warning_20_regular.svg?raw'
import info20Regular from './vendor/info_20_regular.svg?raw'
import moreHorizontal20Regular from './vendor/more_horizontal_20_regular.svg?raw'
import scan20Regular from './vendor/scan_20_regular.svg?raw'
import dismissCircle20Regular from './vendor/dismiss_circle_20_regular.svg?raw'
import compassNorthwest20Regular from './vendor/compass_northwest_20_regular.svg?raw'
import appsListDetail20Regular from './vendor/apps_list_detail_20_regular.svg?raw'
import type { IconRole } from './types'

export type IconAsset = {
  readonly component: Component
}

const iconAsset = (svg: string): IconAsset => ({
  component: defineSvgComponent(svg)
})

// Source material: @fluentui/svg-icons@1.1.329. Product code must not reference these assets.
const iconRegistry = {
  'disclosure.closed': iconAsset(chevronRight20Regular),
  'disclosure.open': iconAsset(chevronDown20Regular),
  'folder.plain': iconAsset(folder20Regular),
  'source.local': iconAsset(library20Regular),
  'media.audio': iconAsset(musicNote220Regular),
  'media.video': iconAsset(video20Regular),
  'media.image': iconAsset(image20Regular),
  'media.cueSheet': iconAsset(documentText20Regular),
  'media.playlist': iconAsset(listBar20Regular),
  'media.metadata': iconAsset(documentText20Regular),
  'state.loading': iconAsset(spinnerIos20Regular),
  'state.warning': iconAsset(warning20Regular),
  'state.unknown': iconAsset(info20Regular),
  'action.more': iconAsset(moreHorizontal20Regular),
  'action.scan': iconAsset(scan20Regular),
  'action.remove': iconAsset(dismissCircle20Regular),
  'navigation.collection': iconAsset(compassNorthwest20Regular),
  'navigation.view': iconAsset(appsListDetail20Regular)
} satisfies Record<IconRole, IconAsset>

export function resolveIconAsset(role: IconRole): IconAsset {
  return iconRegistry[role]
}

function normalizeSvg(svg: string): string {
  return svg
    .replace(/\s(?:width|height)="[^"]*"/g, '')
    .replace(/fill="(?!none")[^"]*"/g, 'fill="currentColor"')
    .replace(/stroke="(?!none")[^"]*"/g, 'stroke="currentColor"')
}

function defineSvgComponent(svg: string): Component {
  const normalizedSvg = normalizeSvg(svg)
  const viewBox = normalizedSvg.match(/viewBox="([^"]+)"/)?.[1] ?? '0 0 20 20'
  const paths = [...normalizedSvg.matchAll(/<path d="([^"]+)"/g)].map((match) => match[1])

  return defineComponent({
    name: 'RegistryIcon',
    setup() {
      return () =>
        h(
          'svg',
          {
            xmlns: 'http://www.w3.org/2000/svg',
            viewBox,
            focusable: 'false'
          },
          paths.map((d) => h('path', { d, fill: 'currentColor' }))
        )
    }
  })
}

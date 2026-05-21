import { h, defineComponent } from 'vue'

const svgAttrs = {
  xmlns: 'http://www.w3.org/2000/svg',
  width: '24',
  height: '24',
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  'stroke-width': '2',
  'stroke-linecap': 'round',
  'stroke-linejoin': 'round'
} as const

export const SdCardIcon = defineComponent({
  name: 'SdCardIcon',
  inheritAttrs: false,
  setup(_, { attrs }) {
    return () =>
      h('svg', { ...svgAttrs, ...attrs }, [
        h('rect', { width: '18', height: '18', x: '3', y: '3', rx: '2', ry: '2' }),
        h('line', { x1: '7', x2: '7', y1: '3', y2: '9' }),
        h('line', { x1: '11', x2: '11', y1: '3', y2: '9' }),
        h('line', { x1: '15', x2: '15', y1: '3', y2: '9' }),
        h('path', { d: 'M3 11h18' })
      ])
  }
})

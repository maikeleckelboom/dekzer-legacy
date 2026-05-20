import type { ComputedRef, ComponentPublicInstance, Ref } from 'vue'

import { createContext } from '../../rendererFoundation/context'
import type { TreeNodeId, TreeVisibleItem } from './types'

export type TreeContext = {
  readonly visibleItems: ComputedRef<readonly TreeVisibleItem[]>
  readonly activeNodeId: Ref<TreeNodeId | null>
  readonly getItemTabIndex: (nodeId: TreeNodeId) => 0 | -1
  readonly setActiveNode: (nodeId: TreeNodeId) => void
  readonly registerItemElement: (
    nodeId: TreeNodeId,
    element: Element | ComponentPublicInstance | null
  ) => void
  readonly handleItemClick: (item: TreeVisibleItem, event: MouseEvent) => void
  readonly handleItemKeydown: (item: TreeVisibleItem, event: KeyboardEvent) => void
}

const treeContext = createContext<TreeContext>('Library browser tree')

export const provideTreeContext = treeContext.provideContext
export const useTreeContext = treeContext.injectContext

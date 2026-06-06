import type { ComputedRef, Ref } from 'vue'

import { createRequiredContext } from '../../vue/context'
import type { TreeKeyboardIntent } from './keys'
import type { BrowserTreeNodeId, BrowserTreeVisibleItem } from './types'

export type TreeContext = {
  readonly visibleItems: ComputedRef<readonly BrowserTreeVisibleItem[]>
  readonly activeNodeId: Ref<BrowserTreeNodeId | undefined>
  readonly getItemTabIndex: (nodeId: BrowserTreeNodeId) => 0 | -1
  readonly registerItemElement: (nodeId: BrowserTreeNodeId, element?: HTMLElement) => void
  readonly setActiveNode: (nodeId: BrowserTreeNodeId) => void
  readonly focusNode: (nodeId: BrowserTreeNodeId) => void
  readonly scrollNodeIntoView: (nodeId: BrowserTreeNodeId) => void
  readonly prepareNode: (nodeId: BrowserTreeNodeId) => void
  readonly cancelPrepareNode: (nodeId: BrowserTreeNodeId) => void
  readonly selectNode: (nodeId: BrowserTreeNodeId) => void
  readonly toggleNode: (nodeId: BrowserTreeNodeId) => void
  readonly revealNode: (nodeId: BrowserTreeNodeId) => void
  readonly activateAction: (nodeId: BrowserTreeNodeId) => void
  readonly activatePrimary: (nodeId: BrowserTreeNodeId) => void
  readonly resolveKeyboardIntent: (item: BrowserTreeVisibleItem, key: string) => TreeKeyboardIntent
}

const treeContext = createRequiredContext<TreeContext>({
  contextName: 'library.tree',
  providerName: 'TreeRoot'
})

export const provideTreeContext = treeContext.provide
export const useTreeContext = treeContext.inject

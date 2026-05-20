<template>
  <p
    v-if="nodes.length === 0 && !isChildGroup"
    class="rounded-sm border border-dashed border-(--color-border) px-4 py-5 text-sm text-(--color-text-muted)"
    role="status"
  >
    No fixture hierarchy nodes to display.
  </p>

  <ul
    v-else-if="nodes.length > 0"
    :aria-label="isChildGroup ? undefined : 'Fixture library hierarchy tree'"
    :class="
      isChildGroup ? 'mt-1 ml-9 space-y-1 border-l border-(--color-border) pl-3' : 'space-y-1'
    "
    :role="isChildGroup ? 'group' : 'tree'"
  >
    <li
      v-for="node in nodes"
      :key="node.id"
      :aria-expanded="getNodeChildren(node).length > 0 ? isNodeExpanded(node.id) : undefined"
      :aria-selected="selectedNodeId === node.id"
      role="treeitem"
    >
      <div class="flex min-w-0 items-start gap-2">
        <button
          v-if="getNodeChildren(node).length > 0"
          type="button"
          class="mt-1 grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-(--color-border) text-xs font-bold text-(--color-text-muted) transition hover:border-(--color-accent) hover:text-(--color-text) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
          :aria-label="getToggleLabel(node)"
          @click="emit('toggle', node.id)"
        >
          <span aria-hidden="true">{{ isNodeExpanded(node.id) ? '-' : '+' }}</span>
        </button>

        <span
          v-else
          class="mt-1 grid h-7 w-7 shrink-0 place-items-center text-[10px] text-(--color-border)"
          aria-hidden="true"
        >
          -
        </span>

        <button
          type="button"
          :class="getNodeButtonClass(node.id, getNodeChildren(node).length > 0)"
          @click="emit('select', node.id)"
          @keydown="handleNodeKeydown(node, $event)"
        >
          <span class="min-w-0">
            <span class="block truncate text-sm font-semibold leading-5">{{ node.label }}</span>
            <span
              v-if="node.detail"
              class="block truncate text-xs leading-5 text-(--color-text-muted)"
            >
              {{ node.detail }}
            </span>
          </span>
          <span
            class="shrink-0 rounded-sm border border-(--color-border) px-2 py-0.5 text-[11px] font-semibold uppercase leading-4 text-(--color-text-muted)"
          >
            {{ formatNodeKind(node.kind) }}
          </span>
        </button>
      </div>

      <LibraryHierarchyTree
        v-if="getNodeChildren(node).length > 0 && isNodeExpanded(node.id)"
        :expanded-node-ids="expandedNodeIds"
        :is-child-group="true"
        :nodes="getNodeChildren(node)"
        :selected-node-id="selectedNodeId"
        @select="emit('select', $event)"
        @toggle="emit('toggle', $event)"
      />
    </li>
  </ul>
</template>

<script setup lang="ts">
import type {
  LibraryHierarchyNodeId,
  LibraryHierarchyNodeKind,
  LibraryHierarchyTreeNode
} from './libraryHierarchyTypes'

defineOptions({
  name: 'LibraryHierarchyTree'
})

const props = withDefaults(
  defineProps<{
    nodes: readonly LibraryHierarchyTreeNode[]
    selectedNodeId: LibraryHierarchyNodeId | null
    expandedNodeIds: ReadonlySet<LibraryHierarchyNodeId>
    isChildGroup?: boolean
  }>(),
  {
    isChildGroup: false
  }
)

const emit = defineEmits<{
  select: [nodeId: LibraryHierarchyNodeId]
  toggle: [nodeId: LibraryHierarchyNodeId]
}>()

function getNodeChildren(node: LibraryHierarchyTreeNode): readonly LibraryHierarchyTreeNode[] {
  return node.children ?? []
}

function isNodeExpanded(nodeId: LibraryHierarchyNodeId): boolean {
  return props.expandedNodeIds.has(nodeId)
}

function getToggleLabel(node: LibraryHierarchyTreeNode): string {
  return `${isNodeExpanded(node.id) ? 'Collapse' : 'Expand'} ${node.label}`
}

function getNodeButtonClass(
  nodeId: LibraryHierarchyNodeId,
  hasChildren: boolean
): readonly string[] {
  const baseClass =
    'flex min-w-0 flex-1 items-center justify-between gap-3 rounded-sm border-l-2 px-3 py-2 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)'
  const selectedClass = 'border-l-(--color-accent) bg-(--color-surface-strong) text-(--color-text)'
  const idleClass =
    'border-l-transparent text-(--color-text-muted) hover:bg-white/5 hover:text-(--color-text)'
  const branchClass = hasChildren ? 'font-semibold' : ''

  return [baseClass, props.selectedNodeId === nodeId ? selectedClass : idleClass, branchClass]
}

function handleNodeKeydown(node: LibraryHierarchyTreeNode, event: KeyboardEvent): void {
  const hasChildren = getNodeChildren(node).length > 0

  if (!hasChildren) {
    return
  }

  if (event.key === 'ArrowRight' && !isNodeExpanded(node.id)) {
    event.preventDefault()
    emit('toggle', node.id)
  }

  if (event.key === 'ArrowLeft' && isNodeExpanded(node.id)) {
    event.preventDefault()
    emit('toggle', node.id)
  }
}

function formatNodeKind(kind: LibraryHierarchyNodeKind): string {
  switch (kind) {
    case 'fixtureRoot':
      return 'Fixture root'
    case 'folder':
      return 'Folder'
    case 'playlistGroup':
      return 'Playlist group'
    case 'preparation':
      return 'Preparation'
    case 'history':
      return 'History'
    case 'trackGroup':
      return 'Track group'
  }
}
</script>

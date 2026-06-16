<script setup lang="ts">
import { computed } from 'vue'

import { Icon, type IconRole } from '../../icons'
import { resolveBrowserTreeRowIcon } from '../tree/presentation'
import type { BrowserProjection } from '../tree/projection'
import type { BrowserTreeNodeId } from '../tree/types'
import ContentsTable from './table.vue'
import { projectColumns, type ColRow } from './columnModel'
import type { ContentProjection, ContentRow } from './projection'
import type { StatusAction, StatusView } from '../sourceStatus/projection'
import type {
  SourceAdmissionHandoffAction,
  SourceAdmissionHandoffProjection
} from '../runtime/sourceAdmissionHandoff'

defineOptions({
  name: 'ContentsColumns'
})

const props = defineProps<{
  projection: ContentProjection
  statusView?: StatusView
  sourceAdmissionHandoff: SourceAdmissionHandoffProjection | undefined
  selectedRowId?: string
  activateRowAction: (row: ContentRow) => void
  activateStatusAction?: (action: StatusAction) => void
  activateSourceAdmissionHandoffAction?: (action: SourceAdmissionHandoffAction) => void
  columnProjection: BrowserProjection | undefined
  selectedNodeId?: BrowserTreeNodeId
  selectNode: (nodeId: BrowserTreeNodeId) => void
}>()

const emit = defineEmits<{
  'select-row': [row: ContentRow]
}>()

const view = computed(() =>
  projectColumns({
    projection: props.columnProjection,
    ...(props.selectedNodeId === undefined ? {} : { selectedNodeId: props.selectedNodeId })
  })
)

const tableProps = computed(() => ({
  projection: props.projection,
  sourceAdmissionHandoff: props.sourceAdmissionHandoff,
  ...(props.selectedRowId === undefined ? {} : { selectedRowId: props.selectedRowId }),
  activateRowAction: props.activateRowAction,
  ...(props.statusView === undefined ? {} : { statusView: props.statusView }),
  ...(props.activateStatusAction === undefined
    ? {}
    : { activateStatusAction: props.activateStatusAction }),
  ...(props.activateSourceAdmissionHandoffAction === undefined
    ? {}
    : { activateSourceAdmissionHandoffAction: props.activateSourceAdmissionHandoffAction })
}))

function activateRow(row: ColRow): void {
  if (!row.canActivate) {
    return
  }

  props.selectNode(row.id)
}

function rowIcon(row: ColRow): IconRole | undefined {
  return resolveBrowserTreeRowIcon({
    id: row.id,
    label: row.label,
    role: row.role,
    ...(row.detail === undefined ? {} : { detail: row.detail }),
    ...(row.icon === undefined ? {} : { icon: row.icon }),
    children: { kind: 'none' }
  })
}
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-sm border border-(--color-border) bg-(--color-background)"
    role="region"
    aria-label="Column view"
  >
    <div class="flex min-h-0 flex-1 overflow-hidden">
      <div
        class="flex min-h-0 min-w-0 shrink-0 overflow-x-auto border-r border-(--color-border)"
        aria-label="Browse columns"
      >
        <section
          v-for="column in view.columns"
          :key="column.id"
          class="flex min-h-0 w-56 shrink-0 flex-col border-r border-(--color-border) last:border-r-0"
          :aria-label="column.title"
        >
          <header
            class="shrink-0 border-b border-(--color-border) px-3 py-2 text-xs font-bold uppercase tracking-normal text-(--color-text-muted)"
          >
            <span class="block truncate" :title="column.title">{{ column.title }}</span>
          </header>
          <div class="min-h-0 flex-1 overflow-y-auto py-1">
            <button
              v-for="row in column.rows"
              :key="row.id"
              type="button"
              class="flex min-h-9 w-full min-w-0 items-center gap-2 border-l-2 px-3 py-2 text-left text-sm transition focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
              :class="[
                row.active
                  ? 'border-l-(--color-accent) bg-(--color-surface-strong) font-bold text-(--color-text)'
                  : row.inPath
                    ? 'border-l-(--color-accent) bg-(--color-surface) text-(--color-text)'
                    : 'border-l-transparent text-(--color-text-muted)',
                row.canActivate
                  ? 'hover:bg-(--color-surface) hover:text-(--color-text)'
                  : 'cursor-not-allowed opacity-60'
              ]"
              :disabled="!row.canActivate"
              :title="row.detail"
              :aria-current="row.active ? 'true' : undefined"
              @click="activateRow(row)"
            >
              <span class="grid size-5 shrink-0 place-items-center" aria-hidden="true">
                <Icon v-if="rowIcon(row)" :role="rowIcon(row) ?? 'state.unknown'" size="sm" />
              </span>
              <span class="min-w-0 flex-1 truncate">{{ row.label }}</span>
            </button>
          </div>
        </section>
      </div>

      <div class="min-h-0 min-w-[28rem] flex-1 overflow-hidden p-2">
        <ContentsTable v-bind="tableProps" @select-row="emit('select-row', $event)" />
      </div>
    </div>
  </section>
</template>

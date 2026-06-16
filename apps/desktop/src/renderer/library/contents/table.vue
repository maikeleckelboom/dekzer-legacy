<script setup lang="ts">
import { computed } from 'vue'

import { Icon, type IconRole, type IconTone } from '../../icons'
import { type ContentProjection, type ContentRow, type ContentRowIcon } from './projection'
import {
  projectList,
  rowSubject,
  shouldSelectRowForKey,
  type Cell,
  type colKey,
  type ListRow
} from './listModel'
import { type StatusAction, type StatusView } from '../sourceStatus/projection'
import {
  type SourceAdmissionHandoffAction,
  type SourceAdmissionHandoffProjection
} from '../runtime/sourceAdmissionHandoff'

defineOptions({
  name: 'ContentsTable'
})

const props = defineProps<{
  projection: ContentProjection
  statusView?: StatusView
  sourceAdmissionHandoff: SourceAdmissionHandoffProjection | undefined
  selectedRowId?: string
  activateRowAction: (row: ContentRow) => void
  activateStatusAction?: (action: StatusAction) => void
  activateSourceAdmissionHandoffAction?: (action: SourceAdmissionHandoffAction) => void
}>()

const emit = defineEmits<{
  'select-row': [row: ContentRow]
}>()

const list = computed(() => projectList(props.projection))
const musicalKeys = ['artist', 'album', 'bpm', 'key', 'time', 'rating'] satisfies readonly colKey[]

function canSelectRow(row: ListRow): boolean {
  return rowSubject(row) !== undefined
}

function selectRow(row: ListRow): void {
  if (!canSelectRow(row)) {
    return
  }

  emit('select-row', row.base)
}

function selectRowByKey(event: KeyboardEvent, row: ListRow): void {
  if (!shouldSelectRowForKey(event.key)) {
    return
  }

  event.preventDefault()
  selectRow(row)
}

function resolveContentRowIcon(icon: ContentRowIcon | undefined): IconRole | undefined {
  switch (icon) {
    case 'folder':
      return 'folder.plain'
    case 'music':
      return 'media.audio'
    case 'video':
      return 'media.video'
    case 'image':
      return 'media.image'
    case 'cueSheet':
      return 'media.cueSheet'
    case 'metadata':
      return 'media.metadata'
    case 'more':
      return 'action.more'
    case 'loading':
      return 'state.loading'
    case 'warning':
      return 'state.warning'
    case 'state':
      return 'state.unknown'
    default:
      return undefined
  }
}

function iconToneForRow(row: ContentRow): IconTone {
  if (row.kind === 'state') {
    if (row.state === 'empty' || row.state === 'notLoaded') return 'muted'
    if (row.state === 'attention' || row.state === 'failed' || row.state === 'unsupported')
      return 'warning'
    return 'muted'
  }

  if (row.presence === 'missing') return 'warning'
  if (row.presence === 'removed') return 'danger'

  const icon = row.icon
  switch (icon) {
    case 'music':
    case 'video':
    case 'image':
      return 'primary'
    case 'cueSheet':
    case 'metadata':
      return 'muted'
    case 'warning':
      return 'warning'
    default:
      return 'inherit'
  }
}

function labelClassForRow(row: ContentRow): string {
  if (row.kind === 'state') {
    return row.state === 'attention' ? 'text-(--color-warning)' : 'text-(--color-text-muted)'
  }
  if (row.presence === 'missing') return 'text-(--color-warning)'
  if (row.presence === 'removed') return 'text-(--color-danger)'

  const icon = row.icon
  switch (icon) {
    case 'music':
    case 'video':
    case 'image':
      return 'text-(--color-text)'
    case 'cueSheet':
    case 'metadata':
      return 'text-(--color-text-muted)'
    default:
      return 'text-(--color-text)'
  }
}

function cellClass(cell: Cell): string {
  switch (cell.tone) {
    case 'warning':
      return 'text-(--color-warning)'
    case 'danger':
      return 'text-(--color-danger)'
    case 'muted':
      return 'text-(--color-text-muted)'
    case 'normal':
      return 'text-(--color-text)'
  }
}

function colClass(key: colKey): string {
  switch (key) {
    case 'index':
      return 'w-12 text-right'
    case 'title':
      return 'w-[30%]'
    case 'artist':
      return 'w-[14%]'
    case 'album':
      return 'w-[14%]'
    case 'bpm':
      return 'w-16 text-right'
    case 'key':
      return 'w-16'
    case 'time':
      return 'w-16 text-right'
    case 'rating':
      return 'w-20'
    case 'ready':
      return 'w-32'
    case 'source':
      return 'w-[18%]'
  }
}

function resolveContentActionIcon(action: ContentRow['action']): IconRole {
  switch (action?.kind) {
    case 'chooseMusicFolder':
    case 'requestLocalBrowseAdmission':
      return 'folder.plain'
    case 'loadChildren':
    case 'loadContentsPage':
    case 'loadLocalBrowseChildren':
    case 'loadLocalBrowseMore':
    case 'loadSearchPage':
    default:
      return 'action.more'
  }
}

function statusBadgeClass(view: StatusView): string {
  switch (view.tone) {
    case 'ready':
      return 'border-(--color-border) text-(--color-text)'
    case 'active':
      return 'border-(--color-accent) text-(--color-accent)'
    case 'warning':
      return 'border-(--color-warning) text-(--color-warning)'
    case 'danger':
      return 'border-(--color-danger) text-(--color-danger)'
    case 'muted':
      return 'border-(--color-border) text-(--color-text-muted)'
  }
}

function scopeHealthClass(tone: ContentProjection['header']['health']['tone']): string {
  switch (tone) {
    case 'ready':
      return 'border-(--color-border) text-(--color-text)'
    case 'active':
      return 'border-(--color-accent) text-(--color-accent)'
    case 'warning':
      return 'border-(--color-warning) text-(--color-warning)'
    case 'danger':
      return 'border-(--color-danger) text-(--color-danger)'
    case 'muted':
      return 'border-(--color-border) text-(--color-text-muted)'
  }
}

function resolveStatusActionIcon(action: StatusAction): IconRole {
  switch (action.kind) {
    case 'addLocalPath':
    case 'showSource':
      return 'folder.plain'
    case 'scanSource':
    case 'runMaintenance':
      return 'action.scan'
    case 'removeSource':
      return 'action.remove'
    case 'refreshStatus':
      return 'state.unknown'
  }
}

function resolveHandoffActionIcon(action: SourceAdmissionHandoffAction): IconRole {
  switch (action.kind) {
    case 'viewSource':
      return 'folder.plain'
    case 'scanSource':
      return 'action.scan'
    case 'addAnotherSource':
    case 'keepBrowsing':
      return 'action.more'
  }
}
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-sm border border-(--color-border) bg-(--color-background)"
    role="region"
    aria-labelledby="library-contents-region-label library-contents-title"
    aria-live="polite"
  >
    <header class="shrink-0 border-b border-(--color-border) px-4 py-3">
      <span id="library-contents-region-label" class="sr-only">Library contents</span>
      <div class="flex min-w-0 items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="text-xs font-bold uppercase tracking-normal text-(--color-text-muted)">
            {{ projection.header.surfaceLabel }}
            <template v-if="projection.header.profileLabel !== undefined">
              / {{ projection.header.profileLabel }}
            </template>
          </p>
          <h3
            id="library-contents-title"
            class="mt-1 truncate text-base font-bold leading-6 text-(--color-text)"
            :title="projection.header.scopeLabel"
          >
            {{ projection.header.scopeLabel }}
          </h3>
          <p
            v-if="projection.detail !== undefined || projection.header.searchLabel !== undefined"
            class="mt-1 min-w-0 text-xs leading-5 text-(--color-text-muted)"
          >
            <span
              v-if="projection.header.searchLabel !== undefined"
              class="font-semibold text-(--color-text)"
            >
              {{ projection.header.searchLabel }}
            </span>
            <span v-if="projection.header.searchScopeLabel !== undefined">
              {{ projection.header.searchLabel !== undefined ? ' - ' : '' }}
              {{ projection.header.searchScopeLabel }}
            </span>
            <span
              v-if="
                projection.detail !== undefined &&
                (projection.header.searchLabel !== undefined ||
                  projection.header.searchScopeLabel !== undefined)
              "
            >
              -
            </span>
            <span v-if="projection.detail !== undefined">
              {{ projection.detail }}
            </span>
          </p>
        </div>
        <span
          class="inline-flex min-h-7 shrink-0 items-center rounded-sm border px-2 py-1 text-xs font-bold"
          :class="scopeHealthClass(projection.header.health.tone)"
        >
          {{ projection.header.health.label }}
        </span>
      </div>
      <div
        v-if="sourceAdmissionHandoff !== undefined"
        class="mt-3 border border-(--color-border) bg-(--color-surface) px-3 py-2"
        aria-label="Source added"
      >
        <div class="flex min-w-0 flex-wrap items-center gap-2">
          <span class="text-sm font-bold text-(--color-text)">
            {{ sourceAdmissionHandoff.title }}
          </span>
          <span class="min-w-0 flex-1 truncate text-xs leading-5 text-(--color-text-muted)">
            {{ sourceAdmissionHandoff.detail }}
            {{ sourceAdmissionHandoff.readinessDetail }}
          </span>
          <button
            v-for="action in sourceAdmissionHandoff.actions"
            :key="action.kind"
            type="button"
            class="inline-flex min-h-7 shrink-0 items-center justify-center gap-1.5 rounded-sm border border-(--color-border) bg-(--color-background) px-2 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!action.enabled"
            :title="action.reason"
            @click="activateSourceAdmissionHandoffAction?.(action)"
          >
            <Icon :role="resolveHandoffActionIcon(action)" size="xs" />
            <span>{{ action.label }}</span>
          </button>
        </div>
      </div>
      <div
        v-if="statusView !== undefined && statusView.badge !== undefined"
        class="mt-2 flex min-w-0 flex-wrap items-center gap-2"
        role="group"
        aria-label="Source status"
      >
        <span
          class="inline-flex min-h-7 items-center rounded-sm border px-2 py-1 text-xs font-bold"
          :class="statusBadgeClass(statusView)"
        >
          {{ statusView.badge }}
        </span>
        <span
          v-if="statusView.detail !== undefined"
          class="min-w-0 flex-1 truncate text-xs leading-5 text-(--color-text-muted)"
          :title="statusView.detail"
        >
          {{ statusView.detail }}
        </span>
        <button
          v-for="action in statusView.actions"
          :key="`${action.kind}:${'sourceId' in action ? action.sourceId : action.resolvedPath}`"
          type="button"
          class="inline-flex min-h-7 shrink-0 items-center justify-center gap-1.5 rounded-sm border border-(--color-border) bg-(--color-surface) px-2 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background) disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="!action.enabled"
          :title="action.reason"
          @click="activateStatusAction?.(action)"
        >
          <Icon :role="resolveStatusActionIcon(action)" size="xs" />
          <span>{{ action.label }}</span>
        </button>
      </div>
    </header>

    <div
      class="min-h-0 flex-1 overflow-auto scrollbar-gutter-stable scrollbar-track-transparent scrollbar-thumb-gray-200"
    >
      <table class="min-w-full w-full table-fixed border-collapse text-left text-sm">
        <thead
          class="sticky top-0 z-10 border-b border-(--color-border) bg-(--color-background) text-xs uppercase text-(--color-text-muted)"
        >
          <tr>
            <th
              v-for="col in list.columns"
              :key="col.key"
              class="px-3 py-2 font-bold"
              :class="colClass(col.key)"
            >
              {{ col.label }}
            </th>
          </tr>
        </thead>
        <tbody class="divide-y divide-(--color-border)">
          <tr
            v-for="row in list.rows"
            :key="row.id"
            :class="[
              row.base.state === 'attention' ||
              row.base.state === 'failed' ||
              row.base.state === 'unsupported'
                ? 'text-(--color-warning)'
                : '',
              selectedRowId === row.id && canSelectRow(row)
                ? 'bg-(--color-surface) shadow-[inset_3px_0_0_var(--color-accent)]'
                : canSelectRow(row)
                  ? 'cursor-default hover:bg-(--color-surface)'
                  : ''
            ]"
            :data-content-row-kind="row.base.kind"
            :data-content-list-family="row.family"
            :data-content-row-selected="
              selectedRowId === row.id && canSelectRow(row) ? 'true' : undefined
            "
            :tabindex="canSelectRow(row) ? 0 : undefined"
            @click="selectRow(row)"
            @keydown="selectRowByKey($event, row)"
          >
            <td class="px-3 py-2 align-middle text-right text-xs font-semibold">
              <span
                class="block truncate"
                :class="cellClass(row.cells.index)"
                :title="row.cells.index.text"
              >
                {{ row.cells.index.text }}
              </span>
            </td>
            <td class="px-3 py-2 align-middle">
              <div class="flex min-w-0 items-center gap-2">
                <span class="grid h-7 w-7 shrink-0 place-items-center" aria-hidden="true">
                  <Icon
                    v-if="row.cells.title.icon !== undefined"
                    :role="resolveContentRowIcon(row.cells.title.icon) ?? 'state.unknown'"
                    size="sm"
                    :tone="iconToneForRow(row.base)"
                  />
                </span>
                <span
                  class="min-w-0 flex-1 truncate font-semibold"
                  :class="labelClassForRow(row.base)"
                  :title="row.cells.title.text"
                >
                  {{ row.cells.title.text }}
                </span>
              </div>
              <span
                v-if="row.cells.title.detail !== undefined"
                class="mt-0.5 block truncate pl-9 text-xs leading-5 text-(--color-text-muted)"
                :title="row.cells.title.detail"
              >
                {{ row.cells.title.detail }}
              </span>
            </td>
            <td
              v-for="key in musicalKeys"
              :key="key"
              class="px-3 py-2 align-middle text-xs font-semibold"
              :class="colClass(key)"
            >
              <span class="block truncate" :class="cellClass(row.cells[key])">
                {{ row.cells[key].text }}
              </span>
            </td>
            <td class="px-3 py-2 align-middle" :class="colClass('ready')">
              <div class="flex min-w-0 items-center justify-between gap-2">
                <span
                  class="min-w-0 truncate text-xs leading-5 text-(--color-text-muted)"
                  :class="cellClass(row.cells.ready)"
                  :title="row.cells.ready.text"
                >
                  {{ row.cells.ready.text }}
                </span>
                <button
                  v-if="row.base.action !== undefined"
                  type="button"
                  class="inline-flex min-h-8 shrink-0 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-surface) px-2.5 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
                  @click.stop="activateRowAction(row.base)"
                >
                  <Icon :role="resolveContentActionIcon(row.base.action)" size="xs" />
                  <span>{{ row.base.action.label }}</span>
                </button>
              </div>
            </td>
            <td class="px-3 py-2 align-middle text-xs font-semibold" :class="colClass('source')">
              <span
                class="block truncate"
                :class="cellClass(row.cells.source)"
                :title="row.cells.source.text"
              >
                {{ row.cells.source.text }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

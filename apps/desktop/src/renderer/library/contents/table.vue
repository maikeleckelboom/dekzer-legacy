<script setup lang="ts">
import { Icon, type IconRole, type IconTone } from '../../icons'
import { type ContentProjection, type ContentRow, type ContentRowIcon } from './projection'
import { type StatusAction, type StatusView } from '../sourceStatus/projection'
import {
  type SourceAdmissionHandoffAction,
  type SourceAdmissionHandoffProjection
} from '../runtime/sourceAdmissionHandoff'

defineOptions({
  name: 'ContentsTable'
})

defineProps<{
  projection: ContentProjection
  statusView?: StatusView
  sourceAdmissionHandoff: SourceAdmissionHandoffProjection | undefined
  activateRowAction: (row: ContentRow) => void
  activateStatusAction?: (action: StatusAction) => void
  activateSourceAdmissionHandoffAction?: (action: SourceAdmissionHandoffAction) => void
}>()

function formatContentDetail(row: ContentRow): string {
  if (row.state === 'empty') return row.detail ?? 'Empty'
  if (row.state === 'notLoaded') return 'Not loaded'
  if (row.state === 'loading') return 'Loading'
  if (row.state === 'attention') return row.detail ?? 'Needs attention'
  if (row.state === 'failed') return 'Unavailable'
  if (row.state === 'unsupported') return 'Unsupported'
  if (row.state === 'file' && row.kind === 'state') return row.detail ?? ''

  if (row.kind === 'state') return row.detail ?? ''
  if (row.kind === 'more') return ''

  if (row.kind === 'directory') {
    if (row.presence === 'missing') return 'Missing'
    if (row.presence === 'removed') return 'Removed'
    return 'Folder'
  }

  if (row.kind === 'file') {
    if (row.presence === 'missing') return 'Missing'
    if (row.presence === 'removed') return 'Removed'
    if (row.detail !== undefined) return row.detail
    if (row.fileClass === 'audio') return 'Audio'
    if (row.fileClass === 'video') return 'Video'
    if (row.fileClass === 'image') return 'Image'
    const icon = row.icon
    switch (icon) {
      case 'music':
        return 'Audio'
      case 'video':
        return 'Video'
      case 'cueSheet':
        return 'Cue sheet'
      case 'metadata':
        return 'Metadata'
      default:
        return 'File'
    }
  }

  return ''
}

function formatContentKind(row: ContentRow): string {
  if (row.kind === 'directory') return 'Folder'
  if (row.kind === 'more') return 'More'

  if (row.kind === 'state') {
    switch (row.state) {
      case 'empty':
        return 'Empty'
      case 'notLoaded':
        return 'Pending'
      case 'loading':
        return 'Loading'
      case 'attention':
        return 'Attention'
      case 'failed':
        return 'Unavailable'
      case 'unsupported':
        return 'Unavailable'
      case 'file':
        return 'File'
      default:
        return 'State'
    }
  }

  if (row.fileClass === 'audio') return 'Audio'
  if (row.fileClass === 'video') return 'Video'
  if (row.fileClass === 'image') return 'Image'

  switch (row.icon) {
    case 'cueSheet':
      return 'Cue sheet'
    case 'metadata':
      return 'Metadata'
    default:
      return 'File'
  }
}

function formatContentState(row: ContentRow): string {
  if (row.presence === 'missing') return 'Missing'
  if (row.presence === 'removed') return 'Removed'
  if (row.kind === 'more') return row.action === undefined ? 'Loading' : 'More available'

  if (row.kind === 'state') {
    if (row.state === 'empty') return 'Empty'
    if (row.state === 'notLoaded') return 'Not loaded'
    if (row.state === 'loading') return 'Loading'
    if (row.state === 'attention') return 'Needs attention'
    if (row.state === 'failed') return 'Unavailable'
    if (row.state === 'unsupported') return 'Unavailable'
    if (row.state === 'file') return 'Selected'
  }

  if (row.detail === 'Indexing') return 'Indexing'
  if (row.detail === 'Source missing' || row.detail === 'File missing') return 'Missing'
  if (row.detail === 'Source blocked') return 'Blocked'
  if (row.detail === 'Source file removed' || row.detail === 'File removed') return 'Removed'
  return 'Available'
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
            <th class="w-[58%] px-4 py-2 font-bold">Name</th>
            <th class="w-[16%] px-4 py-2 font-bold">Kind</th>
            <th class="w-[26%] px-4 py-2 font-bold">State</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-(--color-border)">
          <tr
            v-for="row in projection.rows"
            :key="row.id"
            :class="
              row.state === 'attention' || row.state === 'failed' || row.state === 'unsupported'
                ? 'text-(--color-warning)'
                : ''
            "
            :data-content-row-kind="row.kind"
          >
            <td class="px-4 py-2 align-middle">
              <div class="flex min-w-0 items-center gap-2">
                <span class="grid h-7 w-7 shrink-0 place-items-center" aria-hidden="true">
                  <Icon
                    v-if="row.icon !== undefined"
                    :role="resolveContentRowIcon(row.icon) ?? 'state.unknown'"
                    size="sm"
                    :tone="iconToneForRow(row)"
                  />
                </span>
                <span class="min-w-0 flex-1 truncate font-semibold" :class="labelClassForRow(row)">
                  {{ row.label }}
                </span>
              </div>
              <span
                v-if="formatContentDetail(row).length > 0"
                class="mt-0.5 block truncate pl-9 text-xs leading-5 text-(--color-text-muted)"
                :title="formatContentDetail(row)"
              >
                {{ formatContentDetail(row) }}
              </span>
            </td>
            <td class="px-4 py-2 align-middle text-xs font-semibold text-(--color-text-muted)">
              <span class="block truncate" :title="formatContentKind(row)">
                {{ formatContentKind(row) }}
              </span>
            </td>
            <td class="px-4 py-2 align-middle">
              <div class="flex min-w-0 items-center justify-between gap-2">
                <span
                  class="min-w-0 truncate text-xs leading-5 text-(--color-text-muted)"
                  :title="formatContentState(row)"
                >
                  {{ formatContentState(row) }}
                </span>
                <button
                  v-if="row.action !== undefined"
                  type="button"
                  class="inline-flex min-h-8 shrink-0 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-surface) px-2.5 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
                  @click="activateRowAction(row)"
                >
                  <Icon :role="resolveContentActionIcon(row.action)" size="xs" />
                  <span>{{ row.action.label }}</span>
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

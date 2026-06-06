<script setup lang="ts">
import {
  FileTextIcon,
  FolderIcon,
  FolderOpenIcon,
  Icon,
  ImageIcon,
  ListMusicIcon,
  LoadingIcon,
  MoreIcon,
  MusicIcon,
  StateIcon,
  VideoIcon,
  WarningIcon
} from '../../icons'
import type { IconComponent, IconTone } from '../../icons'
import { type ContentProjection, type ContentRow, type ContentRowIcon } from './projection'

defineOptions({
  name: 'ContentsTable'
})

defineProps<{
  projection: ContentProjection
  activateRowAction: (row: ContentRow) => void
}>()

function formatContentDetail(row: ContentRow): string {
  if (row.state === 'empty') return 'Empty folder'
  if (row.state === 'notLoaded') return 'Not loaded'
  if (row.state === 'loading') return 'Loading'
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
      case 'playlist':
        return 'Playlist'
      case 'metadata':
        return 'Metadata'
      default:
        return 'File'
    }
  }

  return ''
}

function resolveContentRowIcon(icon: ContentRowIcon | undefined): IconComponent | undefined {
  switch (icon) {
    case 'folder':
      return FolderIcon
    case 'music':
      return MusicIcon
    case 'video':
      return VideoIcon
    case 'image':
      return ImageIcon
    case 'cueSheet':
      return FileTextIcon
    case 'playlist':
      return ListMusicIcon
    case 'metadata':
      return FileTextIcon
    case 'more':
      return MoreIcon
    case 'loading':
      return LoadingIcon
    case 'warning':
      return WarningIcon
    case 'state':
      return StateIcon
    default:
      return undefined
  }
}

function iconToneForRow(row: ContentRow): IconTone {
  if (row.kind === 'state') {
    if (row.state === 'empty' || row.state === 'notLoaded') return 'muted'
    if (row.state === 'failed' || row.state === 'unsupported') return 'warning'
    return 'muted'
  }

  if (row.presence === 'missing') return 'warning'
  if (row.presence === 'removed') return 'danger'
  if (row.availabilityState === 'unavailable') return 'warning'
  if (row.availabilityState === 'degraded') return 'warning'

  const icon = row.icon
  switch (icon) {
    case 'music':
    case 'video':
    case 'image':
      return 'primary'
    case 'cueSheet':
    case 'playlist':
    case 'metadata':
      return 'muted'
    case 'warning':
      return 'warning'
    default:
      return 'inherit'
  }
}

function labelClassForRow(row: ContentRow): string {
  if (row.kind === 'state') return 'text-(--color-text-muted)'
  if (row.presence === 'missing') return 'text-(--color-warning)'
  if (row.presence === 'removed') return 'text-(--color-danger)'
  if (row.availabilityState === 'unavailable') return 'text-(--color-warning)'
  if (row.availabilityState === 'degraded') return 'text-(--color-warning)'

  const icon = row.icon
  switch (icon) {
    case 'music':
    case 'video':
    case 'image':
      return 'text-(--color-text)'
    case 'cueSheet':
    case 'playlist':
    case 'metadata':
      return 'text-(--color-text-muted)'
    default:
      return 'text-(--color-text)'
  }
}

function resolveContentActionIcon(row: ContentRow): IconComponent {
  return row.action?.kind === 'loadChildren' ? FolderOpenIcon : MoreIcon
}
</script>

<template>
  <section
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-sm border border-(--color-border) bg-(--color-background)"
    aria-labelledby="library-contents-title"
    aria-live="polite"
  >
    <header class="shrink-0 border-b border-(--color-border) px-4 py-3">
      <p class="text-xs font-bold uppercase tracking-normal text-(--color-text-muted)">Contents</p>
      <h3
        id="library-contents-title"
        class="mt-1 text-base font-bold leading-6 text-(--color-text)"
      >
        {{ projection.title }}
      </h3>
      <p
        v-if="projection.detail !== undefined"
        class="mt-1 text-xs leading-5 text-(--color-text-muted)"
      >
        {{ projection.detail }}
      </p>
    </header>

    <div
      class="min-h-0 flex-1 overflow-auto scrollbar-gutter-stable scrollbar-track-transparent scrollbar-thumb-gray-200"
    >
      <table class="min-w-full w-full table-fixed border-collapse text-left text-sm">
        <thead
          class="sticky top-0 z-10 border-b border-(--color-border) bg-(--color-background) text-xs uppercase text-(--color-text-muted)"
        >
          <tr>
            <th class="w-[70%] px-4 py-2 font-bold">Name</th>
            <th class="w-[30%] px-4 py-2 font-bold">Details</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-(--color-border)">
          <tr
            v-for="row in projection.rows"
            :key="row.id"
            :class="
              row.state === 'failed' || row.state === 'unsupported' ? 'text-(--color-warning)' : ''
            "
            :data-content-row-kind="row.kind"
          >
            <td class="px-4 py-2 align-middle">
              <div class="flex min-w-0 items-center gap-2">
                <span class="grid h-7 w-7 shrink-0 place-items-center" aria-hidden="true">
                  <Icon
                    v-if="row.icon !== undefined"
                    :icon="resolveContentRowIcon(row.icon) ?? StateIcon"
                    size="sm"
                    :tone="iconToneForRow(row)"
                  />
                </span>
                <span class="min-w-0 flex-1 truncate font-semibold" :class="labelClassForRow(row)">
                  {{ row.label }}
                </span>
                <button
                  v-if="row.action !== undefined"
                  type="button"
                  class="inline-flex min-h-8 shrink-0 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-surface) px-2.5 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
                  @click="activateRowAction(row)"
                >
                  <Icon :icon="resolveContentActionIcon(row)" size="xs" />
                  <span>{{ row.action.label }}</span>
                </button>
              </div>
            </td>
            <td class="px-4 py-2 align-middle text-xs leading-5 text-(--color-text-muted)">
              <span class="block truncate" :title="formatContentDetail(row)">
                {{ formatContentDetail(row) }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

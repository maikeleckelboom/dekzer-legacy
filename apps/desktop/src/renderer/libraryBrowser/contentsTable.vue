<script setup lang="ts">
import {
  FolderIcon,
  FolderOpenIcon,
  Icon,
  LoadingIcon,
  MoreIcon,
  MusicIcon,
  StateIcon,
  WarningIcon
} from '../icons'
import type { IconComponent } from '../icons'
import { type ContentProjection, type ContentRow, type ContentRowIcon } from './contentsProjection'

defineOptions({
  name: 'ContentsTable'
})

defineProps<{
  projection: ContentProjection
  activateRowAction: (row: ContentRow) => void
}>()

function formatContentRowKind(kind: ContentRow['kind']): string {
  switch (kind) {
    case 'directory':
      return 'Directory'
    case 'file':
      return 'File'
    case 'state':
      return 'State'
    case 'more':
      return 'More'
  }
}

function formatContentPresence(presence: ContentRow['presence']): string {
  switch (presence) {
    case 'present':
      return 'Present'
    case 'missing':
      return 'Missing'
    case 'removed':
      return 'Removed'
    default:
      return '-'
  }
}

function formatContentUpdated(updatedAtMs: ContentRow['updatedAtMs']): string {
  return updatedAtMs === undefined ? '-' : String(updatedAtMs)
}

function resolveContentRowIcon(icon: ContentRowIcon | undefined): IconComponent | undefined {
  switch (icon) {
    case 'folder':
      return FolderIcon
    case 'music':
      return MusicIcon
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

function resolveContentActionIcon(row: ContentRow): IconComponent {
  return row.action?.kind === 'loadMore' ? MoreIcon : FolderOpenIcon
}
</script>

<template>
  <section
    class="min-w-0 rounded-sm border border-(--color-border) bg-(--color-background)"
    aria-labelledby="library-contents-title"
    aria-live="polite"
  >
    <header class="border-b border-(--color-border) px-4 py-3">
      <p class="text-xs font-bold uppercase tracking-normal text-(--color-text-muted)">
        Selected contents
      </p>
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

    <div class="overflow-x-auto">
      <table class="min-w-[44rem] w-full table-fixed border-collapse text-left text-sm">
        <thead class="border-b border-(--color-border) text-xs uppercase text-(--color-text-muted)">
          <tr>
            <th class="w-[42%] px-4 py-2 font-bold">Name</th>
            <th class="w-[13%] px-3 py-2 font-bold">Kind</th>
            <th class="w-[13%] px-3 py-2 font-bold">Presence</th>
            <th class="w-[12%] px-3 py-2 font-bold">Updated</th>
            <th class="w-[20%] px-3 py-2 font-bold">Status</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-(--color-border)">
          <tr
            v-for="row in projection.rows"
            :key="row.id"
            class="text-(--color-text-muted)"
            :data-content-row-kind="row.kind"
          >
            <td class="px-4 py-2 align-middle">
              <div class="flex min-w-0 items-center gap-2">
                <span class="grid h-7 w-7 shrink-0 place-items-center" aria-hidden="true">
                  <Icon
                    v-if="row.icon !== undefined"
                    :icon="resolveContentRowIcon(row.icon) ?? StateIcon"
                    size="sm"
                    :decorative="true"
                  />
                </span>
                <span class="min-w-0 flex-1 truncate font-semibold text-(--color-text)">
                  {{ row.label }}
                </span>
                <button
                  v-if="row.action !== undefined"
                  type="button"
                  class="inline-flex min-h-8 shrink-0 items-center justify-center gap-2 rounded-sm border border-(--color-border) bg-(--color-surface) px-2.5 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-background)"
                  @click="activateRowAction(row)"
                >
                  <Icon :icon="resolveContentActionIcon(row)" size="xs" :decorative="true" />
                  <span>{{ row.action.label }}</span>
                </button>
              </div>
            </td>
            <td class="px-3 py-2 align-middle">
              {{ formatContentRowKind(row.kind) }}
            </td>
            <td class="px-3 py-2 align-middle">
              {{ formatContentPresence(row.presence) }}
            </td>
            <td class="px-3 py-2 align-middle font-mono text-xs">
              {{ formatContentUpdated(row.updatedAtMs) }}
            </td>
            <td class="px-3 py-2 align-middle text-xs leading-5">
              {{ row.detail ?? '-' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

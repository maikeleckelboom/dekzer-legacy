<script setup lang="ts">
import { computed } from 'vue'

import { Icon, type IconRole } from '../../icons'
import type { WorkstationContextLabel, WorkstationInspectorProps } from './types'
import type { SourceAdmissionHandoffAction } from '../runtime/sourceAdmissionHandoff'
import type { StatusAction } from '../sourceStatus/projection'
import type { TrackAnalysisAction } from '../trackAnalysis/controller'

defineOptions({
  name: 'LibraryWorkstationInspector'
})

const props = defineProps<WorkstationInspectorProps>()

const label = computed<WorkstationContextLabel>(() => {
  if (props.selection.kind === 'row') {
    return props.selection.subject.kind === 'playableMedia' ? 'TRACK' : 'NO SELECTION'
  }

  if (props.selection.kind === 'source') {
    switch (props.selection.context.kind) {
      case 'registeredSource':
      case 'registeredDirectory':
      case 'localBrowse':
        return 'SOURCE'
    }
  }

  return 'NO SELECTION'
})

const title = computed(() => {
  if (props.selection.kind === 'row') {
    return props.selection.subject.kind === 'sourceFile'
      ? 'Source file selected'
      : props.selection.subject.label
  }

  if (props.selection.kind === 'source') {
    if (props.status?.title !== undefined) {
      return props.status.title
    }

    return props.selection.context.title
  }

  return 'Nothing selected'
})

const detail = computed(() => {
  if (props.selection.kind === 'row') {
    const subject = props.selection.subject

    if (subject.kind === 'sourceFile') {
      return `${subject.label} is a source file. Track inspection is deferred until playable media identity is available.`
    }

    const observations = [subject.mimeType, subject.codec]
      .filter((value): value is string => value !== undefined && value.trim().length > 0)
      .join(' - ')
    return observations.length > 0
      ? observations
      : (subject.detail ?? 'Playable media identity is available.')
  }

  if (props.selection.kind === 'source') {
    if ('detail' in props.selection.context) {
      return props.selection.context.detail
    }

    return undefined
  }

  return undefined
})

const showNoContextCopy = computed(
  () => props.selection.kind === 'none' && detail.value === undefined
)

const showSourceActions = computed(
  () =>
    props.selection.kind === 'source' &&
    (props.status !== undefined || props.sourceAdmissionHandoff !== undefined)
)

const showTrackAnalysis = computed(() => props.trackAnalysis?.visible === true)

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

function resolveTrackAnalysisActionIcon(action: TrackAnalysisAction): IconRole {
  switch (action.kind) {
    case 'analyze':
      return 'action.scan'
  }
}

function warningClass(severity: 'info' | 'warning' | 'error'): string {
  switch (severity) {
    case 'error':
      return 'text-(--color-danger)'
    case 'warning':
      return 'text-(--color-warning)'
    case 'info':
      return 'text-(--color-text-muted)'
  }
}
</script>

<template>
  <aside
    class="min-h-0 min-w-0 overflow-y-auto border-l border-(--color-border) bg-(--color-surface) p-3"
    aria-label="Inspector"
  >
    <p class="text-xs font-bold tracking-normal text-(--color-text-muted)">
      {{ label }}
    </p>
    <h3 class="mt-2 truncate text-sm font-bold leading-5 text-(--color-text)" :title="title">
      {{ title }}
    </h3>

    <p v-if="detail !== undefined" class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      {{ detail }}
    </p>
    <p v-else-if="showNoContextCopy" class="mt-3 text-xs leading-5 text-(--color-text-muted)">
      No inspectable context selected.
    </p>

    <section
      v-if="showTrackAnalysis && trackAnalysis !== undefined"
      class="mt-4 border-t border-(--color-border) pt-3"
      aria-label="Track Analysis"
    >
      <div class="flex min-w-0 items-start justify-between gap-2">
        <div class="min-w-0">
          <h4 class="truncate text-xs font-bold text-(--color-text)">Track Analysis</h4>
          <p class="mt-1 text-xs leading-5 text-(--color-text-muted)">
            {{ trackAnalysis.detail }}
          </p>
        </div>
        <span
          class="inline-flex min-h-6 shrink-0 items-center rounded-sm border border-(--color-border) px-2 py-0.5 text-xs font-bold text-(--color-text)"
        >
          {{ trackAnalysis.statusLabel }}
        </span>
      </div>

      <div class="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          class="inline-flex min-h-7 items-center justify-center gap-1.5 rounded-sm border border-(--color-border) bg-(--color-background) px-2 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-surface) disabled:cursor-not-allowed disabled:opacity-60"
          :disabled="!trackAnalysis.action.enabled"
          :title="trackAnalysis.action.reason"
          @click="activateTrackAnalysisAction?.(trackAnalysis.action)"
        >
          <Icon :role="resolveTrackAnalysisActionIcon(trackAnalysis.action)" size="xs" />
          <span>{{ trackAnalysis.action.label }}</span>
        </button>
      </div>

      <dl v-if="trackAnalysis.facts.length > 0" class="mt-3 grid grid-cols-1 gap-2">
        <div
          v-for="fact in trackAnalysis.facts"
          :key="fact.label"
          class="min-w-0 border-l border-(--color-border) pl-2"
        >
          <dt class="text-[11px] font-bold uppercase tracking-normal text-(--color-text-muted)">
            {{ fact.label }}
          </dt>
          <dd class="mt-0.5 text-sm font-bold leading-5 text-(--color-text)">
            {{ fact.value }}
          </dd>
          <dd
            v-if="fact.detail !== undefined && fact.detail.length > 0"
            class="text-xs leading-5 text-(--color-text-muted)"
          >
            {{ fact.detail }}
          </dd>
        </div>
      </dl>

      <p
        v-if="trackAnalysis.warningSummary !== undefined"
        class="mt-3 text-xs leading-5 text-(--color-text-muted)"
      >
        {{ trackAnalysis.warningSummary }}
      </p>

      <ul v-if="trackAnalysis.warnings.length > 0" class="mt-2 space-y-1">
        <li
          v-for="warning in trackAnalysis.warnings"
          :key="`${warning.severity}:${warning.code}:${warning.message}`"
          class="text-xs leading-5"
          :class="warningClass(warning.severity)"
        >
          {{ warning.message }}
        </li>
      </ul>

      <p
        v-if="trackAnalysis.basisLine !== undefined"
        class="mt-3 break-words text-[11px] leading-4 text-(--color-text-muted)"
      >
        {{ trackAnalysis.basisLine }}
      </p>
    </section>

    <section
      v-if="showSourceActions"
      class="mt-4 border-t border-(--color-border) pt-3"
      aria-label="Source actions"
    >
      <div v-if="sourceAdmissionHandoff !== undefined" class="space-y-2">
        <div class="min-w-0">
          <h4 class="truncate text-xs font-bold text-(--color-text)">
            {{ sourceAdmissionHandoff.title }}
          </h4>
          <p
            class="mt-1 truncate text-xs leading-5 text-(--color-text-muted)"
            :title="`${sourceAdmissionHandoff.detail} ${sourceAdmissionHandoff.readinessDetail}`"
          >
            {{ sourceAdmissionHandoff.detail }}
            {{ sourceAdmissionHandoff.readinessDetail }}
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="action in sourceAdmissionHandoff.actions"
            :key="action.kind"
            type="button"
            class="inline-flex min-h-7 items-center justify-center gap-1.5 rounded-sm border border-(--color-border) bg-(--color-background) px-2 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-surface) disabled:cursor-not-allowed disabled:opacity-60"
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
        v-if="status !== undefined"
        class="mt-3 space-y-2 first:mt-0"
        role="group"
        aria-label="Source status"
      >
        <span
          v-if="status.badge !== undefined"
          class="inline-flex min-h-6 items-center rounded-sm border border-(--color-border) px-2 py-0.5 text-xs font-bold text-(--color-text)"
        >
          {{ status.badge }}
        </span>
        <p v-if="status.detail !== undefined" class="text-xs leading-5 text-(--color-text-muted)">
          {{ status.detail }}
        </p>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="action in status.actions"
            :key="`${action.kind}:${'sourceId' in action ? action.sourceId : action.resolvedPath}`"
            type="button"
            class="inline-flex min-h-7 items-center justify-center gap-1.5 rounded-sm border border-(--color-border) bg-(--color-background) px-2 py-1 text-xs font-bold text-(--color-text) transition hover:border-(--color-accent) hover:text-(--color-accent) focus-visible:ring-2 focus-visible:ring-(--color-accent) focus-visible:ring-offset-2 focus-visible:ring-offset-(--color-surface) disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="!action.enabled"
            :title="action.reason"
            @click="activateStatusAction?.(action)"
          >
            <Icon :role="resolveStatusActionIcon(action)" size="xs" />
            <span>{{ action.label }}</span>
          </button>
        </div>
      </div>
    </section>
  </aside>
</template>

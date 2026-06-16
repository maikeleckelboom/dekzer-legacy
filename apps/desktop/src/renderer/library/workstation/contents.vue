<script setup lang="ts">
import { computed } from 'vue'

import ContentsTable from '../contents/table.vue'
import type { WorkstationContentsProps } from './types'

defineOptions({
  name: 'LibraryWorkstationContents'
})

const props = defineProps<WorkstationContentsProps>()

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
</script>

<template>
  <main class="min-h-0 min-w-0 overflow-hidden p-2" aria-label="Contents">
    <ContentsTable v-bind="tableProps" @select-row="props.selectRow" />
  </main>
</template>

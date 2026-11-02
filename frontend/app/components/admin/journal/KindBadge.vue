<script setup lang="ts">
import type { JournalKind } from '~/types/admin-journal'

defineProps<{ kind: JournalKind; size?: 'sm' | 'md' }>()

const { t } = useI18n()
</script>

<template>
  <UiBadge
    :intent="JOURNAL_KIND_INTENT[kind]"
    :size="size ?? 'sm'"
    solid
    :label="t(`admin.journal.kind.${kind}`)"
  />
</template>

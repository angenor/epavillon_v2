<script setup lang="ts">
import type { JournalChange } from '~/types/admin-journal'

/**
 * Le détail d'une modification : avant et après, champ par champ. Un champ
 * masqué dit son nom et « valeur masquée », jamais sa valeur : l'API ne la
 * rend pas, et l'écran n'a rien à deviner.
 */

defineProps<{ change: JournalChange }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const labels = useJournalLabels()
</script>

<template>
  <aside
    class="rounded-lg border border-border bg-surface-raised p-4 lg:sticky lg:top-4"
    :aria-label="t('admin.journal.detail.label')"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <UiBadge :intent="JOURNAL_ACTION_INTENT[change.action]" size="sm" solid :label="t(`admin.journal.action.${change.action}`)" />
        <h2 class="mt-2 text-lg font-semibold">{{ labels.changeEntityLabel(change) }}</h2>
      </div>
      <UiButton variant="ghost" icon="close" icon-only :label="t('admin.journal.detail.close')" @click="emit('close')" />
    </div>

    <dl class="mt-3 grid grid-cols-[7rem_minmax(0,1fr)] gap-x-3 gap-y-1.5 text-sm">
      <dt class="text-text-muted">{{ t('admin.journal.detail.when') }}</dt>
      <dd>{{ labels.fullTime(change.occurred_at) }}</dd>

      <dt class="text-text-muted">{{ t('admin.journal.changes.columns.actor') }}</dt>
      <dd>
        <template v-if="change.actor">
          <NuxtLink :to="localePath(`/admin/utilisateurs/${change.actor.id}`)">{{ change.actor.display_name }}</NuxtLink>
          <span v-if="change.actor.organization_name">, {{ change.actor.organization_name }}</span>
        </template>
        <span v-else class="text-text-muted">{{ t('admin.journal.changes.system') }}</span>
      </dd>

      <template v-if="change.entity_id">
        <dt class="text-text-muted">{{ t('admin.journal.changes.detail.entityId') }}</dt>
        <dd class="font-mono text-xs [overflow-wrap:anywhere]">{{ change.entity_id }}</dd>
      </template>

      <dt class="text-text-muted">{{ t('admin.journal.changes.detail.table') }}</dt>
      <dd class="font-mono text-xs">{{ change.entity_schema }}.{{ change.entity_table }}</dd>

      <template v-if="change.request_id">
        <dt class="text-text-muted">{{ t('admin.journal.detail.requestId') }}</dt>
        <dd>
          <span class="block font-mono text-xs [overflow-wrap:anywhere]">{{ change.request_id }}</span>
          <NuxtLink class="text-sm" :to="{ query: { onglet: 'erreurs', demande: change.request_id } }">
            {{ t('admin.journal.changes.detail.seeRequest') }}
          </NuxtLink>
        </dd>
      </template>
    </dl>

    <h3 class="mt-4 text-sm font-semibold">{{ t('admin.journal.changes.detail.fields') }}</h3>

    <p v-if="change.fields.length === 0" class="mt-2 text-sm text-text-muted">
      {{ t(`admin.journal.changes.detail.noFields.${change.action}`) }}
    </p>

    <ul v-else class="mt-2 flex flex-col divide-y divide-border-subtle text-sm">
      <li v-for="field in change.fields" :key="field.name" class="py-2">
        <span class="block font-semibold">
          {{ labels.fieldLabel(field.name) }}
          <span class="font-mono text-xs font-normal text-text-muted">{{ field.name }}</span>
        </span>
        <p v-if="field.masked" class="mt-1 flex items-center gap-1.5 text-text-muted">
          <UiIcon name="lock" size="0.9rem" />{{ t('admin.journal.changes.masked') }}
        </p>
        <dl v-else class="mt-1 grid grid-cols-[4rem_minmax(0,1fr)] gap-x-2 gap-y-1">
          <dt class="text-text-muted">{{ t('admin.journal.changes.detail.before') }}</dt>
          <dd class="[overflow-wrap:anywhere]">
            <span v-if="journalValueText(field.before) !== null">{{ journalValueText(field.before) }}</span>
            <span v-else class="text-text-muted italic">{{ t('admin.journal.changes.detail.emptyValue') }}</span>
          </dd>
          <dt class="text-text-muted">{{ t('admin.journal.changes.detail.after') }}</dt>
          <dd class="[overflow-wrap:anywhere]">
            <span v-if="journalValueText(field.after) !== null">{{ journalValueText(field.after) }}</span>
            <span v-else class="text-text-muted italic">{{ t('admin.journal.changes.detail.emptyValue') }}</span>
          </dd>
        </dl>
      </li>
    </ul>
  </aside>
</template>

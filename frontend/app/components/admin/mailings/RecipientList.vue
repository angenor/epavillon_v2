<script setup lang="ts">
import type { MailingRecipientRow, MailingSource } from '~/types/admin-mailings'
import type { TableColumn } from '~/types/ui'

/**
 * LA LISTE DES DESTINATAIRES, avant l'envoi (telle qu'elle partirait) comme
 * après (avec l'état de chacun). Recherche locale, et cent lignes à la fois :
 * une COP compte des milliers d'inscrits.
 */

interface Props {
  rows: MailingRecipientRow[]
  caption: string
  timezone: string
  /** Ville du fuseau, accentuée (« heure de Belém »). */
  zone: string
  loading?: boolean
}

const props = defineProps<Props>()
const { t } = useI18n()
const { dateTime } = useDateTime()

const zoneText = computed(() => t(zoneFormulaKey('zoneOf', props.zone), { zone: props.zone }))

const PAGE = 100
const query = ref('')
const shown = ref(PAGE)

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return props.rows
  return props.rows.filter((r) =>
    [r.email, r.name ?? '', r.organization ?? ''].some((v) => v.toLowerCase().includes(q)),
  )
})
const visible = computed(() => filtered.value.slice(0, shown.value))
watch(query, () => (shown.value = PAGE))

const columns = computed<TableColumn[]>(() => [
  { key: 'email', label: t('admin.mailing.common.list.person') },
  { key: 'organization', label: t('admin.mailing.common.list.organization'), hideOnMobile: true },
  { key: 'sources', label: t('admin.mailing.common.list.sources'), hideBelow: 'lg' },
  { key: 'status', label: t('admin.mailing.common.list.state') },
])
</script>

<template>
  <div>
    <UiSearchInput
      v-if="props.rows.length > 8"
      v-model="query"
      class="mb-3 max-w-md"
      :label="t('admin.mailing.common.list.search')"
      hide-label
      :placeholder="t('admin.mailing.common.list.search')"
      :debounce="0"
    />

    <UiTable
      :columns="columns"
      :rows="visible"
      row-key="email"
      :caption="props.caption"
      visually-hidden-caption
      :loading="props.loading"
      dense
    >
      <template #cell-email="{ row }">
        <span class="block font-medium text-text">{{ row.name || row.email }}</span>
        <span v-if="row.name" class="block text-xs break-all text-text-subtle">{{ row.email }}</span>
      </template>
      <template #cell-organization="{ row }">
        <span class="text-sm text-text-secondary">{{ row.organization ?? '' }}</span>
      </template>
      <template #cell-sources="{ row }">
        <span class="text-xs text-text-subtle">
          {{ row.sources.map((s: MailingSource) => t(`admin.mailing.common.source.${s}`)).join(', ') }}
        </span>
      </template>
      <template #cell-status="{ row }">
        <AdminMailingsStatusBadge v-if="row.status" of="recipient" :status="row.status" />
        <UiBadge
          v-else-if="row.suppressed"
          intent="danger"
          size="sm"
          :label="t('admin.mailing.common.recipientStatus.suppressed')"
        />
        <span v-else class="text-xs text-text-subtle">{{ t('admin.mailing.common.list.willReceive') }}</span>
        <span v-if="row.sentAt" class="mt-0.5 block text-xs text-text-subtle">
          {{ t('admin.mailing.common.atZone', { date: dateTime(row.sentAt, props.timezone), zone: zoneText }) }}
        </span>
        <span v-if="row.error" class="mt-0.5 block text-xs break-words text-danger">{{ row.error }}</span>
      </template>
      <template #empty>
        <UiEmptyState compact icon="users" :title="t('admin.mailing.common.list.empty')" :description="t('admin.mailing.common.list.emptyHint')" />
      </template>
    </UiTable>

    <div v-if="filtered.length > shown" class="mt-3 flex items-center gap-3">
      <UiButton variant="secondary" @click="shown += PAGE">
        {{ t('admin.mailing.common.list.more') }}
      </UiButton>
      <span class="text-sm text-text-subtle">
        {{ t('admin.mailing.common.list.shown', { shown: visible.length, total: filtered.length }) }}
      </span>
    </div>
  </div>
</template>

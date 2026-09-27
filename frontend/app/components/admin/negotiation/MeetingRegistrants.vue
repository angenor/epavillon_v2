<script setup lang="ts">
import type { AdminMeetingRegistrant, AdminMeetingRegistrations } from '~/types/negotiation-meetings'
import type { TimeZoneName, Uuid } from '~/types/shared'
import type { TableColumn } from '~/types/ui'

const props = defineProps<{
  meetingId: Uuid
  timezone: TimeZoneName
  zoneLabel: string
  /** Relu après une annulation ou une publication. */
  revision?: number
}>()

const { t, locale } = useI18n()
const api = useApi()
const { date, time } = useDateTime()

const { data: liste, status, error, refresh } = await useAsyncData<AdminMeetingRegistrations | null>(
  `admin-negotiation-meeting-registrations-${props.meetingId}`,
  () => api.adminNegotiations.inscritesALaReunion(props.meetingId),
  { default: () => null, lazy: true, watch: [() => props.revision] },
)

const moment = (r: AdminMeetingRegistrant): string =>
  t('admin.negociations.reunions.registrants.at', {
    date: date(r.registered_at, props.timezone),
    time: time(r.registered_at, props.timezone),
    zone: props.zoneLabel,
  })

const colonnes = computed<TableColumn[]>(() => [
  { key: 'name', label: t('admin.negociations.reunions.registrants.columns.name') },
  { key: 'country', label: t('admin.negociations.reunions.registrants.columns.country'), hideOnMobile: true, width: '12rem' },
  { key: 'registered_at', label: t('admin.negociations.reunions.registrants.columns.registeredAt'), width: '16rem' },
])
const colonnesDAttente = computed<TableColumn[]>(() => [
  { key: 'waitlist_position', label: t('admin.negociations.reunions.registrants.columns.position'), width: '6rem', numeric: true, align: 'end' },
  ...colonnes.value,
])
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
    <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.registrants.title') }}</h2>

    <UiErrorState v-if="error" class="mt-4" compact :retry-label="t('common.actions.retry')" @retry="refresh()" />
    <UiLoadingState v-else-if="status === 'pending' || !liste" class="mt-4" variant="table" :lines="3" />
    <UiEmptyState
      v-else-if="!liste.registered.length && !liste.waitlisted.length"
      class="mt-4"
      icon="users"
      :title="t('admin.negociations.reunions.registrants.empty.title')"
      :description="t('admin.negociations.reunions.registrants.empty.description')"
    />

    <template v-else>
      <UiTable
        class="mt-4"
        :columns="colonnes"
        :rows="liste.registered"
        row-key="person_id"
        row-label-key="name"
        :caption="t('admin.negociations.reunions.registrants.registered', liste.registered.length)"
        :hoverable="false"
        dense
      >
        <template #cell-country="{ row }">{{ row.country ? resolveI18nText(row.country, locale) : '—' }}</template>
        <template #cell-registered_at="{ row }">{{ moment(row) }}</template>
        <template #empty>
          <p class="py-4 text-sm text-text-muted">{{ t('admin.negociations.reunions.registrants.noneRegistered') }}</p>
        </template>
      </UiTable>

      <UiTable
        v-if="liste.waitlisted.length"
        class="mt-6"
        :columns="colonnesDAttente"
        :rows="liste.waitlisted"
        row-key="person_id"
        row-label-key="name"
        :caption="t('admin.negociations.reunions.registrants.waitlisted', liste.waitlisted.length)"
        :hoverable="false"
        dense
      >
        <template #cell-country="{ row }">{{ row.country ? resolveI18nText(row.country, locale) : '—' }}</template>
        <template #cell-registered_at="{ row }">{{ moment(row) }}</template>
      </UiTable>
    </template>
  </section>
</template>

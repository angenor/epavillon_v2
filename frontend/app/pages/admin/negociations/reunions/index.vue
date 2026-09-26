<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { AdminFrancophoneMeeting, AdminFrancophoneMeetings } from '~/types/negotiation-meetings'
import type { PublicEditionRow } from '~/types/views'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

// Les réunions de la Francophonie d'une COP, brouillons compris. Portée globale.

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationMeetings' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const { date, timeRange, zoneOf } = useDateTime()

useHead(() => ({ title: t('admin.negociations.reunions.list.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canManage = computed(() => hasPermission(granted.value, 'negotiation.meeting.manage'))

const { data: editions, status: editionsStatus } = await useAsyncData<PublicEditionRow[]>(
  'admin-negotiation-cop-editions',
  () => api.events.publicList(),
  { default: () => [], lazy: true },
)
const choixEditions = computed(() => editionsDeCop(editions.value, locale.value))
const slug = computed(() =>
  typeof route.query.edition === 'string' && route.query.edition
    ? route.query.edition
    : (choixEditions.value.parDefaut ?? ''),
)

const { data: ecran, status, error, refresh } = await useAsyncData<AdminFrancophoneMeetings | null>(
  'admin-negotiation-meetings',
  () => (slug.value ? api.adminNegotiations.reunions(slug.value) : Promise.resolve(null)),
  { lazy: true, watch: [slug] },
)

const isForbidden = computed(
  () => (!canManage.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)

const fuseau = computed(() => (ecran.value?.edition.timezone ?? 'UTC') as TimeZoneName)
const ville = computed(() => ecran.value?.edition.city?.trim() || timeZoneCityLabel(fuseau.value))
const reunions = computed(() => ecran.value?.meetings ?? [])

function choisirLEdition(valeur: string): void {
  router.replace({ query: { ...route.query, edition: valeur } })
}

const nouvelle = computed(() => localePath({ path: '/admin/negociations/reunions/nouvelle', query: { edition: slug.value } }))

const colonnes = computed<TableColumn[]>(() => [
  { key: 'title', label: t('admin.negociations.reunions.list.columns.title') },
  { key: 'when', label: t('admin.negociations.reunions.list.columns.when'), width: '15rem' },
  { key: 'format', label: t('admin.negociations.reunions.list.columns.format'), hideBelow: 'lg', width: '8rem' },
  { key: 'seats', label: t('admin.negociations.reunions.list.columns.seats'), hideBelow: 'lg', width: '11rem' },
  { key: 'status', label: t('admin.negociations.reunions.list.columns.status'), width: '8rem' },
])

const lignes = computed(() =>
  reunions.value.map((r) => ({ ...r, titre: resolveI18nText(r.title, locale.value) })),
)

const places = (r: AdminFrancophoneMeeting): string => {
  if (!r.requires_registration) return t('admin.negociations.reunions.list.noRegistration')
  const inscrites =
    r.capacity === null
      ? t('admin.negociations.reunions.list.seatsOpen', r.registered_count)
      : t('admin.negociations.reunions.list.seats', { count: r.registered_count, capacity: r.capacity })
  return r.waitlisted_count > 0
    ? t('admin.negociations.reunions.list.withWaitlist', {
        seats: inscrites,
        waitlist: t('admin.negociations.reunions.list.waitlist', r.waitlisted_count),
      })
    : inscrites
}

function ouvrir(r: AdminFrancophoneMeeting): void {
  void navigateTo(localePath(`/admin/negociations/reunions/${r.id}`))
}
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.reunions.forbidden.scope')"
      :description="t('admin.negociations.reunions.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-4">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.reunions.list.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.reunions.list.subtitle') }}</p>
        </div>
        <div class="flex w-full flex-wrap items-end gap-3 sm:w-auto">
          <UiSelect
            v-if="choixEditions.options.length > 1"
            class="w-full sm:w-56"
            :label="t('admin.negociations.reunions.edition')"
            :options="choixEditions.options"
            :model-value="slug"
            hide-optional
            @update:model-value="choisirLEdition"
          />
          <UiButton v-if="slug" icon="plus" :to="nouvelle">
            {{ t('admin.negociations.reunions.list.new') }}
          </UiButton>
        </div>
      </header>

      <UiEmptyState
        v-if="editionsStatus === 'success' && !choixEditions.options.length"
        class="mt-8"
        icon="calendar"
        :title="t('admin.negociations.reunions.noEdition.title')"
        :description="t('admin.negociations.reunions.noEdition.description')"
      />

      <UiErrorState v-else-if="error" class="mt-8" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <UiLoadingState v-else-if="status === 'pending' || !ecran" class="mt-8" variant="table" />

      <UiEmptyState
        v-else-if="!reunions.length"
        class="mt-8"
        icon="calendar"
        :title="t('admin.negociations.reunions.list.empty.title')"
        :description="t('admin.negociations.reunions.list.empty.description')"
        :action-label="t('admin.negociations.reunions.list.new')"
        :action-to="nouvelle"
      />

      <UiTable
        v-else
        class="mt-8"
        :columns="colonnes"
        :rows="lignes"
        row-key="id"
        row-label-key="titre"
        :caption="t('admin.negociations.reunions.list.caption', { zone: zoneOf(ville) })"
        @row-click="ouvrir"
      >
        <template #cell-title="{ row }">
          <NuxtLink
            :to="localePath(`/admin/negociations/reunions/${row.id}`)"
            class="font-semibold text-text hover:text-accent hover:underline"
          >
            {{ row.titre }}
          </NuxtLink>
          <span class="mt-0.5 block text-sm text-text-muted">
            {{ row.type ? resolveI18nText(row.type.label, locale) : t('admin.negociations.reunions.list.noType') }}
          </span>
        </template>
        <template #cell-when="{ row }">
          <span class="block">{{ date(row.start_at, fuseau) }}</span>
          <span class="block text-sm text-text-muted">{{ timeRange(row.start_at, row.end_at, fuseau, ville) }}</span>
        </template>
        <template #cell-format="{ row }">{{ t(`admin.negociations.reunions.format.${row.format}`) }}</template>
        <template #cell-seats="{ row }">{{ places(row) }}</template>
        <template #cell-status="{ row }">
          <AdminNegotiationMeetingStatusBadge :status="row.status" />
        </template>
      </UiTable>
    </template>
  </div>
</template>

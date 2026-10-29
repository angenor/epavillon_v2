<script setup lang="ts">
import type { MailingListScreen, MailingStatus, MailingSummary } from '~/types/admin-mailings'
import type { EffectivePermission } from '~/types/identity'
import type { TabItem, TableColumn } from '~/types/ui'

/**
 * LES COURRIELS DE L'ÉDITION, `/admin/courriels` : l'historique des envois.
 *
 * La boîte d'envoi de la plateforme ne se consulte pas : cette liste la
 * remplace. Chaque envoi y reste, avec qui l'a écrit, quand, et ce qu'il est
 * advenu de ses messages. « Écrire un courriel » crée tout de suite un
 * brouillon enregistré : on peut le quitter et y revenir.
 *
 * Chemin français : `/admin/emails` est une adresse de la v1, redirigée vers
 * `/admin` par le serveur.
 *
 * Gardé par `engagement.mailing.send` SUR L'ÉDITION choisie (règle n° 8).
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.mailings' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()
const { dateTime } = useDateTime()

useHead(() => ({ title: t('admin.mailing.list.title') }))

await adminScope.ensureLoaded()

const { data: granted, status: grantStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-mailings-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canSend = computed(() =>
  hasPermission(granted.value, 'engagement.mailing.send', adminScope.currentEventId),
)

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<MailingListScreen | null>(
  'admin-mailings',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId || !canSend.value) return null
    return api.adminMailings.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId, canSend], lazy: true },
)

const timezone = computed(() => adminScope.currentEvent?.timezone ?? 'UTC')
const zone = computed(() => adminScope.currentEvent?.city || timeZoneCityLabel(timezone.value))

type Filter = 'all' | MailingStatus
const filter = ref<Filter>('all')
const query = ref('')

const allRows = computed<MailingSummary[]>(() => screen.value?.rows ?? [])
const tabs = computed<TabItem[]>(() =>
  (['all', 'draft', 'queued', 'sent', 'cancelled'] as const).map((value) => ({
    value,
    label: t(`admin.mailing.list.filter.${value}`),
    count: value === 'all' ? allRows.value.length : allRows.value.filter((r) => r.status === value).length,
  })),
)
const rows = computed(() => {
  const q = query.value.trim().toLowerCase()
  return allRows.value.filter(
    (r) =>
      (filter.value === 'all' || r.status === filter.value) &&
      (!q || r.subject.toLowerCase().includes(q) || (r.queued_by_name ?? r.created_by_name ?? '').toLowerCase().includes(q)),
  )
})

const columns = computed<TableColumn[]>(() => [
  { key: 'subject', label: t('admin.mailing.list.columns.subject') },
  { key: 'status', label: t('admin.mailing.list.columns.status') },
  { key: 'counts', label: t('admin.mailing.list.columns.recipients'), hideOnMobile: true },
  { key: 'date', label: t('admin.mailing.list.columns.date', { zone: t(zoneFormulaKey('zoneOf', zone.value), { zone: zone.value }) }), hideBelow: 'lg' },
  { key: 'actions', label: t('admin.mailing.list.columns.actions'), align: 'end', width: '4.5rem' },
])

// Un envoi en cours ne se supprime pas : on l'arrête d'abord, depuis sa fiche.
const toDelete = ref<MailingSummary | null>(null)

function onDeleted(id: string): void {
  toDelete.value = null
  if (screen.value) screen.value = { ...screen.value, rows: screen.value.rows.filter((r) => r.id !== id) }
  void refresh()
}

// « Nouveau courriel » et les brouillons ouvrent la fenêtre de rédaction ; la
// liste se relit quand elle se ferme.
const composer = useMailComposerStore()
const createError = ref<string | null>(null)
watch(() => composer.version, () => void refresh())

function create(): void {
  composer.open()
}

function open(row: MailingSummary): void {
  if (row.status === 'draft') composer.open(row.id)
  else navigateTo(localePath(`/admin/courriels/${row.id}`))
}

function dateOf(row: MailingSummary): string {
  return dateTime(row.queued_at ?? row.updated_at, timezone.value)
}

const isLoadingAccess = computed(() => adminScope.isLoading || grantStatus.value === 'pending')
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.mailing.list.forbidden')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.mailing.list.title') }}</h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.mailing.list.subtitle') }}</p>
        </div>
        <div v-if="canSend" class="flex flex-wrap gap-3">
          <UiButton variant="secondary" icon="grid" :to="localePath('/admin/courriels/gabarits')">
            {{ t('admin.mailing.list.templates') }}
          </UiButton>
          <UiButton icon="plus" @click="create">
            {{ t('admin.mailing.list.new') }}
          </UiButton>
        </div>
      </header>

      <UiAlert v-if="createError" class="mt-6" intent="danger" live :message="createError" />

      <UiLoadingState v-if="isLoadingAccess" class="mt-8" variant="text" :lines="4" />

      <UiForbiddenState
        v-else-if="!canSend"
        class="mt-8"
        :required-scope="t('admin.mailing.list.forbidden')"
        action-to="/admin"
        :action-label="t('admin.mailing.common.backToDashboard')"
      />

      <UiErrorState
        v-else-if="error"
        class="mt-8"
        :description="apiErrorMessage(error, t)"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="status !== 'pending' && allRows.length === 0"
        class="mt-8"
        icon="mail"
        :title="t('admin.mailing.list.empty.title')"
        :description="t('admin.mailing.list.empty.description')"
        :action-label="t('admin.mailing.list.new')"
        @action="create"
      />

      <template v-else>
        <div class="mt-8 flex flex-wrap items-end justify-between gap-4">
          <UiTabs :model-value="filter" :items="tabs" :label="t('admin.mailing.list.filter.label')" @update:model-value="filter = $event as Filter" />
          <UiSearchInput
            v-model="query"
            class="w-full sm:w-72"
            :label="t('admin.mailing.list.search')"
            hide-label
            :placeholder="t('admin.mailing.list.search')"
            :debounce="0"
          />
        </div>

        <UiTable
          class="mt-4"
          :columns="columns"
          :rows="rows"
          row-key="id"
          row-label-key="subject"
          :caption="t('admin.mailing.list.caption')"
          visually-hidden-caption
          :loading="status === 'pending'"
          @row-click="open"
        >
          <template #cell-subject="{ row }">
            <button
              v-if="row.status === 'draft'"
              type="button"
              class="cursor-pointer text-left font-semibold text-text hover:underline"
              @click.stop="open(row)"
            >
              {{ row.subject || t('admin.mailing.common.noSubject') }}
            </button>
            <NuxtLink
              v-else
              :to="localePath(`/admin/courriels/${row.id}`)"
              class="font-semibold text-text no-underline hover:underline"
            >
              {{ row.subject || t('admin.mailing.common.noSubject') }}
            </NuxtLink>
            <span class="block text-xs text-text-subtle">
              {{ row.queued_by_name ?? row.created_by_name ?? '' }}
              <template v-if="row.attachment_count">
                · {{ t('admin.mailing.list.attachments', { count: row.attachment_count }, row.attachment_count) }}
              </template>
            </span>
          </template>
          <template #cell-status="{ row }">
            <AdminMailingsStatusBadge of="mailing" :status="row.status" />
          </template>
          <template #cell-counts="{ row }">
            <span v-if="row.status === 'draft'" class="text-sm text-text-subtle">{{ t('admin.mailing.list.notSent') }}</span>
            <span v-else class="text-sm tabular-nums">
              {{ t('admin.mailing.list.progress', { sent: row.counts.sent, total: row.counts.total }) }}
              <span v-if="row.counts.failed" class="text-danger">
                · {{ t('admin.mailing.list.failed', { count: row.counts.failed }, row.counts.failed) }}
              </span>
            </span>
          </template>
          <template #cell-date="{ row }">
            <span class="text-sm text-text-secondary">{{ dateOf(row) }}</span>
          </template>
          <template #cell-actions="{ row }">
            <UiButton
              v-if="row.status !== 'queued'"
              variant="ghost"
              icon="trash"
              icon-only
              :label="t('admin.mailing.list.delete.actionFor', { subject: row.subject || t('admin.mailing.common.noSubject') })"
              @click.stop="toDelete = row"
            />
          </template>
          <template #empty>
            <UiEmptyState compact icon="search" filtered :title="t('admin.mailing.list.noResults')" />
          </template>
        </UiTable>
      </template>
    </template>

    <AdminMailingsDeleteDialog :target="toDelete" @close="toDelete = null" @deleted="onDeleted" />
  </div>
</template>

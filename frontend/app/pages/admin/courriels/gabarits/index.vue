<script setup lang="ts">
import type { MailTemplate, MailTemplatePayload } from '~/types/admin-mailings'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'

/**
 * LES GABARITS DE COURRIEL, `/admin/courriels/gabarits` (29/10).
 *
 * Ceux de l'édition choisie dans la barre du back-office, puis les communs à
 * toutes. Un gabarit habille une mise en page par défaut de ses propres textes
 * et images ; la fenêtre de rédaction les propose après les modèles par défaut.
 * « Nouveau gabarit » le crée tout de suite et ouvre sa page : ses images se
 * rattachent à un gabarit qui existe.
 *
 * Gardé par `engagement.mailing.send` SUR L'ÉDITION choisie (règle n° 8).
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.templates.list.crumb' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()
const { dateTime } = useDateTime()

useHead(() => ({ title: t('admin.mailing.templates.list.title') }))

await adminScope.ensureLoaded()

const { data: granted, status: grantStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-mail-templates-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canSend = computed(() =>
  hasPermission(granted.value, 'engagement.mailing.send', adminScope.currentEventId),
)

const {
  data: templates,
  status,
  error,
  refresh,
} = await useAsyncData<MailTemplate[] | null>(
  'admin-mail-templates',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId || !canSend.value) return null
    return api.adminMailings.templates.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId, canSend], lazy: true },
)

const rows = computed<MailTemplate[]>(() => templates.value ?? [])
const timezone = computed(() => adminScope.currentEvent?.timezone ?? 'UTC')
const zone = computed(() => adminScope.currentEvent?.city || timeZoneCityLabel(timezone.value))

const columns = computed<TableColumn[]>(() => [
  { key: 'name', label: t('admin.mailing.templates.list.columns.name') },
  { key: 'layout', label: t('admin.mailing.templates.list.columns.layout'), hideOnMobile: true },
  { key: 'scope', label: t('admin.mailing.templates.list.columns.scope') },
  {
    key: 'date',
    label: t('admin.mailing.templates.list.columns.date', { zone: t(zoneFormulaKey('zoneOf', zone.value), { zone: zone.value }) }),
    hideBelow: 'lg',
  },
])

const creating = ref(false)
const createError = ref<string | null>(null)

async function create(): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId) return
  creating.value = true
  createError.value = null
  const payload: MailTemplatePayload = {
    name: t('admin.mailing.templates.list.defaultName'),
    layout: 'banniere',
    title: null,
    subtitle: null,
    gradient: true,
    event_id: eventId,
    preselect_for_submitters: false,
  }
  try {
    const created = await api.adminMailings.templates.create(payload)
    await navigateTo(localePath(`/admin/courriels/gabarits/${created.id}`))
  } catch (thrown) {
    createError.value = apiErrorMessage(thrown, t)
  } finally {
    creating.value = false
  }
}

function open(row: MailTemplate): void {
  navigateTo(localePath(`/admin/courriels/gabarits/${row.id}`))
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
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.mailing.templates.list.title') }}</h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.mailing.templates.list.subtitle') }}</p>
        </div>
        <UiButton v-if="canSend" icon="plus" :loading="creating" @click="create">
          {{ t('admin.mailing.templates.list.new') }}
        </UiButton>
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
        v-else-if="status !== 'pending' && rows.length === 0"
        class="mt-8"
        icon="mail"
        :title="t('admin.mailing.templates.list.empty.title')"
        :description="t('admin.mailing.templates.list.empty.description')"
        :action-label="t('admin.mailing.templates.list.new')"
        @action="create"
      />

      <UiTable
        v-else
        class="mt-8"
        :columns="columns"
        :rows="rows"
        row-key="id"
        row-label-key="name"
        :caption="t('admin.mailing.templates.list.caption')"
        visually-hidden-caption
        :loading="status === 'pending'"
        @row-click="open"
      >
        <template #cell-name="{ row }">
          <NuxtLink
            :to="localePath(`/admin/courriels/gabarits/${row.id}`)"
            class="font-semibold text-text no-underline hover:underline"
          >
            {{ row.name }}
          </NuxtLink>
          <span class="block text-xs text-text-subtle">{{ row.created_by_name ?? '' }}</span>
        </template>
        <template #cell-layout="{ row }">
          <span class="text-sm">{{ t(`admin.mailing.composer.templates.${row.layout}.name`) }}</span>
        </template>
        <template #cell-scope="{ row }">
          <span class="text-sm">
            {{ row.event_id ? t('admin.mailing.templates.list.scope.edition') : t('admin.mailing.templates.list.scope.all') }}
          </span>
          <UiBadge v-if="row.preselect_for_submitters" class="mt-1" intent="info">
            {{ t('admin.mailing.templates.list.preselected') }}
          </UiBadge>
          <span v-if="!row.can_edit" class="block text-xs text-text-subtle">{{ t('admin.mailing.templates.list.readOnly') }}</span>
        </template>
        <template #cell-date="{ row }">
          <span class="text-sm text-text-secondary">{{ dateTime(row.updated_at, timezone) }}</span>
        </template>
      </UiTable>
    </template>
  </div>
</template>

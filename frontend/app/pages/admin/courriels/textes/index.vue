<script setup lang="ts">
import type { MailText, MailTextPayload } from '~/types/admin-mailings'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'

/**
 * LES TEXTES TYPES, `/admin/courriels/textes` (01/11), sur le modèle des
 * gabarits.
 *
 * Un objet et un message prêts à l'emploi, proposés dans la fenêtre de
 * rédaction sous les gabarits : en choisir un remplace l'objet et le message.
 * Ceux de l'édition choisie, puis les communs à toutes. « Nouveau texte » le
 * crée tout de suite et ouvre sa page, enregistrée à chaque pause de frappe.
 *
 * Gardé par `engagement.mailing.send` SUR L'ÉDITION choisie (règle n° 8).
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.texts.list.crumb' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()
const { dateTime } = useDateTime()

useHead(() => ({ title: t('admin.mailing.texts.list.title') }))

await adminScope.ensureLoaded()

const { data: granted, status: grantStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-mail-texts-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canSend = computed(() =>
  hasPermission(granted.value, 'engagement.mailing.send', adminScope.currentEventId),
)

const {
  data: texts,
  status,
  error,
  refresh,
} = await useAsyncData<MailText[] | null>(
  'admin-mail-texts',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId || !canSend.value) return null
    return api.adminMailings.texts.list(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId, canSend], lazy: true },
)

const rows = computed<MailText[]>(() => texts.value ?? [])
const timezone = computed(() => adminScope.currentEvent?.timezone ?? 'UTC')
const zone = computed(() => adminScope.currentEvent?.city || timeZoneCityLabel(timezone.value))

const columns = computed<TableColumn[]>(() => [
  { key: 'name', label: t('admin.mailing.texts.list.columns.name') },
  { key: 'category', label: t('admin.mailing.texts.list.columns.category'), hideOnMobile: true },
  { key: 'scope', label: t('admin.mailing.texts.list.columns.scope') },
  {
    key: 'date',
    label: t('admin.mailing.texts.list.columns.date', { zone: t(zoneFormulaKey('zoneOf', zone.value), { zone: zone.value }) }),
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
  const payload: MailTextPayload = {
    name: t('admin.mailing.texts.list.defaultName'),
    category: null,
    subject: '',
    body_html: '',
    sort_order: 0,
    event_id: eventId,
  }
  try {
    const created = await api.adminMailings.texts.create(payload)
    await navigateTo(localePath(`/admin/courriels/textes/${created.id}`))
  } catch (thrown) {
    createError.value = apiErrorMessage(thrown, t)
  } finally {
    creating.value = false
  }
}

function open(row: MailText): void {
  navigateTo(localePath(`/admin/courriels/textes/${row.id}`))
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
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.mailing.texts.list.title') }}</h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.mailing.texts.list.subtitle') }}</p>
        </div>
        <UiButton v-if="canSend" icon="plus" :loading="creating" @click="create">
          {{ t('admin.mailing.texts.list.new') }}
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
        :title="t('admin.mailing.texts.list.empty.title')"
        :description="t('admin.mailing.texts.list.empty.description')"
        :action-label="t('admin.mailing.texts.list.new')"
        @action="create"
      />

      <UiTable
        v-else
        class="mt-8"
        :columns="columns"
        :rows="rows"
        row-key="id"
        row-label-key="name"
        :caption="t('admin.mailing.texts.list.caption')"
        visually-hidden-caption
        :loading="status === 'pending'"
        @row-click="open"
      >
        <template #cell-name="{ row }">
          <NuxtLink
            :to="localePath(`/admin/courriels/textes/${row.id}`)"
            class="font-semibold text-text no-underline hover:underline"
          >
            {{ row.name }}
          </NuxtLink>
          <span class="block text-xs wrap-break-word text-text-subtle">{{ row.subject }}</span>
        </template>
        <template #cell-category="{ row }">
          <span class="text-sm">{{ row.category ?? t('admin.mailing.texts.list.noCategory') }}</span>
        </template>
        <template #cell-scope="{ row }">
          <span class="text-sm">
            {{ row.event_id ? t('admin.mailing.texts.list.scope.edition') : t('admin.mailing.texts.list.scope.all') }}
          </span>
          <span v-if="!row.can_edit" class="block text-xs text-text-subtle">{{ t('admin.mailing.texts.list.readOnly') }}</span>
        </template>
        <template #cell-date="{ row }">
          <span class="text-sm text-text-secondary">{{ dateTime(row.updated_at, timezone) }}</span>
        </template>
      </UiTable>
    </template>
  </div>
</template>

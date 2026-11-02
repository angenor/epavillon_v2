<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { JournalSummary } from '~/types/admin-journal'
import type { TabItem } from '~/types/ui'

/**
 * JOURNAL DE LA PLATEFORME : `/admin/journal`, maquette
 * `docs/maquettes/journal/index.html`.
 *
 * Réservé à l'équipe centrale : `identity.platform.journal` en portée globale.
 * Le geste principal est la recherche par code de demande, celui que la
 * personne qui signale une erreur lit à l'écran.
 *
 * Trois onglets pilotés par `?onglet=` ; chacun porte ses filtres dans l'URL.
 * Changer d'onglet repart d'une adresse nue : les filtres d'un onglet ne
 * veulent rien dire dans l'autre.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.journal' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.journal.title') }))

const { data: granted, status: grantedStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-journal-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [] },
)

const canRead = computed(() => hasPermission(granted.value, 'identity.platform.journal'))
const canRetry = computed(() => hasPermission(granted.value, 'identity.platform.maintenance'))

const {
  data: summary,
  status: summaryStatus,
  error: summaryError,
  refresh: refreshSummary,
} = useAsyncData<JournalSummary | null>(
  'admin-journal-summary',
  async () => (canRead.value ? api.adminJournal.summary() : null),
  { watch: [canRead], lazy: true, default: () => null },
)

/** La permission tient, mais l'API refuse quand même : son refus fait foi. */
const isForbidden = computed(
  () => (grantedStatus.value !== 'pending' && !canRead.value) || isForbiddenError(summaryError.value),
)

const tab = computed(() => journalTabOf(route.query.onglet))

const tabs = computed<TabItem[]>(() => [
  { value: 'erreurs', label: t('admin.journal.tabs.erreurs'), count: summary.value?.http_error || undefined },
  {
    value: 'taches',
    label: t('admin.journal.tabs.taches'),
    count: summary.value ? summary.value.failing_jobs + summary.value.failing_events || undefined : undefined,
  },
  { value: 'modifications', label: t('admin.journal.tabs.modifications') },
])

const tabPath = (item: TabItem): string => `${route.path}?onglet=${item.value}`

const lookup = ref(typeof route.query.demande === 'string' ? route.query.demande : '')
watch(() => route.query.demande, (value) => (lookup.value = typeof value === 'string' ? value : ''))

function findRequest(): void {
  const code = lookup.value.trim()
  if (!code) return
  router.push({ query: { onglet: 'erreurs', demande: code } })
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.journal.forbidden.scope')"
      :action-to="localePath('/admin')"
      :action-label="t('admin.journal.forbidden.action')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-4">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.journal.title') }}</h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.journal.subtitle', { count: summary?.retention_months ?? 3 }) }}
          </p>
        </div>

        <form class="flex w-full flex-wrap items-end gap-2 sm:w-auto" role="search" @submit.prevent="findRequest">
          <UiInput
            v-model="lookup"
            class="w-full font-mono sm:w-80"
            :label="t('admin.journal.lookup.label')"
            hide-label
            hide-optional
            autocomplete="off"
            :placeholder="t('admin.journal.lookup.placeholder')"
          />
          <UiButton type="submit" variant="primary" icon="search">{{ t('admin.journal.lookup.submit') }}</UiButton>
        </form>
      </header>

      <UiAlert
        v-if="summary && summary.dropped > 0"
        class="mt-6"
        intent="warning"
        compact
        :message="t('admin.journal.dropped', summary.dropped)"
      />

      <UiErrorState
        v-if="summaryError"
        class="mt-6"
        compact
        :description="apiErrorMessage(summaryError, (key) => t(key))"
        :retry-label="t('common.actions.retry')"
        @retry="refreshSummary()"
      />
      <AdminJournalFigures v-else class="mt-6" :summary="summary" :loading="summaryStatus === 'pending'" />

      <UiTabs
        class="mt-8"
        :items="tabs"
        :model-value="tab"
        :label="t('admin.journal.tabs.label')"
        :to="tabPath"
      >
        <AdminJournalEntriesTab v-if="tab === 'erreurs'" />
        <AdminJournalFailuresTab v-else-if="tab === 'taches'" :can-retry="canRetry" @changed="refreshSummary()" />
        <AdminJournalChangesTab v-else />
      </UiTabs>
    </template>
  </div>
</template>

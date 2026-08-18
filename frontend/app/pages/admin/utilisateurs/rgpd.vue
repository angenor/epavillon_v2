<script setup lang="ts">
import type { HandlePrivacyRequestPayload, PrivacyQueueScreen, PrivacyRequestView } from '~/types/admin-users'
import type { EffectivePermission } from '~/types/identity'

/**
 * DEMANDES RGPD — `/admin/utilisateurs/rgpd`.
 *
 * LE RGPD ÉTAIT ABSENT DE LA V1. Une plateforme portée par un organe de l'OIF,
 * qui traite des données de ressortissants européens et africains, doit pouvoir
 * prouver un consentement et honorer une demande EN TRENTE JOURS —
 * `privacy_requests.due_at` porte cette obligation dans son `DEFAULT`. Cet écran
 * est la file, et son sujet est l'échéance.
 *
 * PORTÉE GLOBALE EXIGÉE, ET CE N'EST PAS UN EXCÈS DE PRUDENCE. Une demande
 * d'effacement porte sur la plateforme entière, jamais sur une édition : la
 * découper par COP n'a aucun sens, et en montrer une part à un administrateur
 * détaché lui donnerait une file dont il ne peut honorer aucune ligne. L'écran
 * s'ouvre donc entièrement, ou pas du tout.
 *
 * L'ANONYMISATION EST L'ACTE, PAS LE STATUT. `identity.anonymize_person()` purge
 * l'identité et supprime les comptes en conservant les agrégats de
 * participation — pour que les compteurs d'une COP passée ne s'effondrent pas
 * parce qu'une personne exerce son droit. Irréversible, donc confirmée par le
 * nom, comme la fusion de deux organisations.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.users', to: '/admin/utilisateurs' },
    { labelKey: 'admin.user.privacy.breadcrumb' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.user.privacy.title') }))

await adminScope.ensureLoaded()

const timezone = computed(() => auth.person?.timezone ?? 'UTC')

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-privacy-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Portée GLOBALE, et rien d'autre : une demande RGPD ne se découpe pas. */
const canRead = computed(() => hasPermission(granted.value, 'identity.person.manage'))

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<PrivacyQueueScreen | null>(
  'admin-privacy-queue',
  async () => (canRead.value ? api.adminUsers.privacyQueue() : null),
  { watch: [canRead], lazy: true },
)

const handling = ref<PrivacyRequestView | null>(null)
const dialogOpen = ref(false)
const submitting = ref(false)
const dialogError = ref<string | null>(null)

function openRequest(request: PrivacyRequestView): void {
  handling.value = request
  dialogError.value = null
  dialogOpen.value = true
}

async function handle(payload: Omit<HandlePrivacyRequestPayload, 'request_id'>): Promise<void> {
  if (!handling.value) return
  submitting.value = true
  dialogError.value = null

  try {
    const result = await api.adminUsers.handlePrivacyRequest(
      { ...payload, request_id: handling.value.id },
      auth.person?.id ?? null,
    )

    if (result.status !== 'saved' && result.status !== 'anonymized') {
      dialogError.value = t(`admin.user.privacy.error.${result.status}`)
      return
    }

    dialogOpen.value = false
    await refresh()
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[80rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !canRead"
      :required-scope="t('admin.user.privacy.forbidden.scope')"
      :description="t('admin.user.privacy.forbidden.description')"
      action-to="/admin/utilisateurs"
      :action-label="t('admin.user.detail.backToList')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.user.privacy.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.user.privacy.subtitle', { days: screen?.deadline_days ?? 30 }) }}
        </p>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else>
        <!-- LE RETARD EN TÊTE : c'est la seule chose de cet écran qui engage la
             responsabilité de l'IFDD. -->
        <UiAlert
          v-if="screen && screen.overdue_count > 0"
          class="mt-6"
          intent="danger"
          :title="t('admin.user.privacy.overdueAlert.title', { count: screen.overdue_count })"
          :message="t('admin.user.privacy.overdueAlert.message')"
        />

        <UiEmptyState
          v-else-if="screen && screen.requests.length === 0 && status !== 'pending'"
          class="mt-8"
          icon="shield-check"
          :title="t('admin.user.privacy.empty.title')"
          :description="t('admin.user.privacy.empty.description')"
        />

        <AdminUsersPrivacyTable
          v-if="!screen || screen.requests.length > 0"
          class="mt-6"
          :requests="screen?.requests ?? []"
          :caption="t('admin.user.privacy.caption')"
          :timezone="timezone"
          :loading="status === 'pending'"
          @handle="openRequest"
        />
      </template>

      <AdminUsersPrivacyDialog
        v-model:open="dialogOpen"
        :request="handling"
        :timezone="timezone"
        :submitting="submitting"
        :error="dialogError"
        @submit="handle"
      />
    </template>
  </div>
</template>

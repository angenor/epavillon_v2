<script setup lang="ts">
import type { AdminGlossaryInput } from '~/types/admin-negotiation-savoir'
import type { AdminProposal } from '~/types/admin-negotiation-queue'
import type { EffectivePermission } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

/**
 * Un terme proposé : l'expert le définit avec le formulaire du lexique, ce qui crée
 * l'entrée en brouillon ; ses auteurs reçoivent un courriel à sa publication.
 */
definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'admin.negociations.file.nav', to: '/admin/negociations/file?kind=proposals' },
    { labelKey: 'admin.negociations.file.proposal.page.crumb' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()

const id = computed<Uuid>(() => String(route.params.id ?? ''))
const FILE = '/admin/negociations/file?kind=proposals'

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canReview = computed(() => hasPermission(granted.value, 'negotiation.knowledge.review'))

const { data: proposal, status, error, refresh } = await useAsyncData<AdminProposal | null>(
  () => `admin-negotiation-proposal-${id.value}`,
  async () => {
    try {
      return await api.adminNegotiationSavoir.propositionDeLaFile(id.value)
    } catch (erreur) {
      if (erreur instanceof ApiRequestError && erreur.status === 404) return null
      throw erreur
    }
  },
  { default: () => null, lazy: true },
)

useHead(() => ({ title: proposal.value?.term ?? t('admin.negociations.file.proposal.page.crumb') }))

const refusee = computed(
  () => (!canReview.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)

const acceptation = ref(false)
const echec = ref<DocumentFormFailure | null>(null)

async function accepter(entree: AdminGlossaryInput): Promise<void> {
  if (acceptation.value || !proposal.value) return
  acceptation.value = true
  echec.value = null
  try {
    const cree = await api.adminNegotiationSavoir.accepterUneProposition(proposal.value.id, entree)
    await navigateTo(localePath(`/admin/negociations/lexique/${cree.id}`))
  } catch (erreur) {
    echec.value = {
      message: apiErrorMessage(erreur, t),
      field: erreur instanceof ApiRequestError ? erreur.field : null,
    }
  } finally {
    acceptation.value = false
  }
}

const rejet = ref(false)
const echecRejet = ref<string | null>(null)

async function rejeter(pid: Uuid, reason: string): Promise<void> {
  if (rejet.value) return
  rejet.value = true
  echecRejet.value = null
  try {
    proposal.value = await api.adminNegotiationSavoir.rejeterUneProposition(pid, { reason })
  } catch (erreur) {
    echecRejet.value = apiErrorMessage(erreur, t)
  } finally {
    rejet.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="refusee"
      :required-scope="t('admin.negociations.file.forbidden.scope')"
      :description="t('admin.negociations.file.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <UiLoadingState v-else-if="permissionStatus === 'pending' || (status === 'pending' && !proposal)" variant="card" :lines="3" />

    <UiErrorState
      v-else-if="error"
      :description="apiErrorMessage(error, t)"
      :request-id="incidentReference(error) ?? undefined"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="!proposal"
      icon="search"
      :title="t('admin.negociations.file.proposal.page.notFound')"
      :action-label="t('admin.negociations.file.proposal.page.back')"
      :action-to="localePath(FILE)"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance break-words">
          {{ t('admin.negociations.file.proposal.page.title', { term: proposal.term }) }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.file.proposal.page.intro') }}</p>
      </header>

      <AdminNegotiationQueueProposal
        class="mt-6"
        :proposal="proposal"
        :accept-link="false"
        :rejecting="rejet"
        :error="echecRejet"
        @reject="rejeter"
      />

      <AdminNegotiationGlossaryForm
        v-if="proposal.status === 'pending'"
        class="mt-8"
        :entry="null"
        :initial-term="proposal.term"
        :submitting="acceptation"
        :failure="echec"
        @submit="accepter"
      >
        <template #actions>
          <UiButton variant="ghost" :to="localePath(FILE)">
            {{ t('admin.negociations.file.proposal.page.back') }}
          </UiButton>
        </template>
      </AdminNegotiationGlossaryForm>
    </template>
  </div>
</template>

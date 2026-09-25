<script setup lang="ts">
import type { AdminFaqEntry, AdminFaqInput } from '~/types/admin-negotiation-savoir'
import type { EffectivePermission } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

// `nouveau` ouvre la création sur la même adresse que la fiche.

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationFaq', to: '/admin/negociations/faq' },
    { labelKey: 'admin.negociations.faq.detail.crumb' },
  ],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()
const { date, dateTime, zoneLabel } = useDateTime()

const entryId = computed<Uuid>(() => String(route.params.id ?? ''))
const creation = computed(() => entryId.value === 'nouveau')

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const mayPublish = computed(() => hasPermission(granted.value, 'negotiation.knowledge.publish'))
const canEnter = computed(
  () => mayPublish.value || hasPermission(granted.value, 'negotiation.knowledge.review'),
)

const { data: entry, status, error, refresh } = await useAsyncData<AdminFaqEntry | null>(
  () => `admin-negotiation-faq-${entryId.value}`,
  async () => {
    if (creation.value) return null
    try {
      return await api.adminNegotiationSavoir.entreeFaq(entryId.value)
    } catch (erreur) {
      if (erreur instanceof ApiRequestError && erreur.status === 404) return null
      throw erreur
    }
  },
  { default: () => null, lazy: true },
)

const question = computed(() => resolveI18nText(entry.value?.question, locale.value))

useHead(() => ({
  title: creation.value ? t('admin.negociations.faq.detail.newTitle') : question.value || t('admin.negociations.faq.detail.crumb'),
}))

const canPublish = computed(() => entry.value?.can_publish ?? false)
const canReview = computed(() => entry.value?.can_review ?? false)

const enregistrement = ref(false)
const geste = ref<string | null>(null)
const echecDuFormulaire = ref<DocumentFormFailure | null>(null)
const echecDuGeste = ref<string | null>(null)
const resultat = ref<string | null>(null)
const revision = ref(0)
const refusee = ref(false)
const verifieLe = ref('')
const confirmerDepublication = ref(false)
const confirmerSuppression = ref(false)

watch(entryId, () => {
  echecDuFormulaire.value = null
  echecDuGeste.value = null
  resultat.value = null
  verifieLe.value = ''
})

const zone = (instant: string): string =>
  t('admin.negociations.faq.detail.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

const echecDe = (erreur: unknown): DocumentFormFailure => ({
  message: apiErrorMessage(erreur, t),
  field: erreur instanceof ApiRequestError ? erreur.field : null,
})

async function enregistrer(entree: AdminFaqInput): Promise<void> {
  if (enregistrement.value) return
  enregistrement.value = true
  echecDuFormulaire.value = null
  resultat.value = null
  try {
    if (creation.value) {
      const creee = await api.adminNegotiationSavoir.creerFaq(entree)
      await navigateTo(localePath(`/admin/negociations/faq/${creee.id}`))
      return
    }
    entry.value = await api.adminNegotiationSavoir.modifierFaq(entryId.value, entree)
    revision.value += 1
    resultat.value = t('admin.negociations.faq.detail.result.saved')
  } catch (erreur) {
    if (isForbiddenError(erreur)) refusee.value = true
    echecDuFormulaire.value = echecDe(erreur)
  } finally {
    enregistrement.value = false
  }
}

async function agir(nom: string, action: () => Promise<string | null>): Promise<void> {
  if (geste.value) return
  geste.value = nom
  echecDuGeste.value = null
  resultat.value = null
  try {
    const message = await action()
    if (message !== null) resultat.value = message
  } catch (erreur) {
    echecDuGeste.value = apiErrorMessage(erreur, t)
  } finally {
    geste.value = null
    confirmerDepublication.value = false
    confirmerSuppression.value = false
  }
}

const faire = (nom: string, appel: (id: Uuid) => Promise<AdminFaqEntry>, cle: string) =>
  agir(nom, async () => {
    entry.value = await appel(entryId.value)
    revision.value += 1
    return t(cle)
  })

const verifier = () =>
  faire(
    'verify',
    (id) => api.adminNegotiationSavoir.verifierFaq(id, verifieLe.value ? { verified_on: verifieLe.value } : {}),
    'admin.negociations.faq.detail.result.verified',
  )
const publier = () => faire('publish', api.adminNegotiationSavoir.publierFaq, 'admin.negociations.faq.detail.result.published')
const aRevoir = () => faire('toReview', api.adminNegotiationSavoir.aRevoirFaq, 'admin.negociations.faq.detail.result.toReview')
const depublier = () =>
  faire('unpublish', api.adminNegotiationSavoir.depublierFaq, 'admin.negociations.faq.detail.result.unpublished')
const supprimer = () =>
  agir('delete', async () => {
    await api.adminNegotiationSavoir.supprimerFaq(entryId.value)
    await navigateTo(localePath('/admin/negociations/faq'))
    return null
  })
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="
        refusee ||
        isForbiddenError(error) ||
        (permissionStatus !== 'pending' && (creation ? !mayPublish : !canEnter))
      "
      :required-scope="t(creation ? 'admin.negociations.faq.detail.forbidden.scopePublish' : 'admin.negociations.faq.list.forbidden.scope')"
      :description="t('admin.negociations.faq.list.forbidden.description')"
      action-to="/admin/negociations/faq"
      :action-label="t('admin.negociations.faq.detail.backToList')"
    />

    <UiLoadingState v-else-if="permissionStatus === 'pending' || (status === 'pending' && !entry && !creation)" />

    <template v-else-if="creation">
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.negociations.faq.detail.newTitle') }}</h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.faq.detail.newSubtitle') }}</p>
      </header>
      <AdminNegotiationFaqForm
        class="mt-8"
        :entry="null"
        :submitting="enregistrement"
        :failure="echecDuFormulaire"
        @submit="enregistrer"
      >
        <template #actions>
          <UiButton variant="ghost" :to="localePath('/admin/negociations/faq')">
            {{ t('admin.negociations.faq.detail.cancel') }}
          </UiButton>
        </template>
      </AdminNegotiationFaqForm>
    </template>

    <UiErrorState
      v-else-if="error"
      :description="apiErrorMessage(error, t)"
      :request-id="incidentReference(error) ?? undefined"
      :retry-label="t('common.actions.retry')"
      @retry="refresh()"
    />

    <UiEmptyState
      v-else-if="!entry"
      icon="search"
      :title="t('admin.negociations.faq.detail.notFound.title')"
      :description="t('admin.negociations.faq.detail.notFound.description')"
      :action-label="t('admin.negociations.faq.detail.backToList')"
      :action-to="localePath('/admin/negociations/faq')"
    />

    <template v-else>
      <header class="min-w-0">
        <div class="flex flex-wrap items-center gap-3">
          <h1 class="text-3xl leading-tight font-semibold text-balance break-words">{{ question }}</h1>
          <UiBadge
            :intent="KNOWLEDGE_STATUS_INTENT[entry.status]"
            :label="t(`admin.negociations.faq.state.${entry.status}`)"
          />
        </div>
        <p class="mt-1 text-sm text-text-muted">
          <template v-if="entry.verified_on">
            {{
              t(
                entry.verified_by_name ? 'admin.negociations.faq.detail.verifiedBy' : 'admin.negociations.faq.detail.verified',
                { date: date(entry.verified_on, 'UTC'), name: entry.verified_by_name ?? '' },
              )
            }}
          </template>
          <template v-else>{{ t('admin.negociations.faq.detail.notVerified') }}</template>
          · {{ t('admin.negociations.faq.detail.updated', { date: zone(entry.updated_at) }) }}
        </p>
        <p v-if="entry.first_published_at" class="mt-1 text-sm text-text-muted">
          {{ t('admin.negociations.faq.detail.firstPublished', { date: zone(entry.first_published_at) }) }}
        </p>
      </header>

      <div v-if="canPublish" class="mt-6 flex flex-wrap gap-3">
        <UiButton v-if="entry.status !== 'published'" icon="check" :loading="geste === 'publish'" @click="publier">
          {{ t('admin.negociations.faq.detail.actions.publish') }}
        </UiButton>
        <UiButton
          v-if="entry.status === 'published'"
          variant="secondary"
          icon="warning"
          :loading="geste === 'toReview'"
          @click="aRevoir"
        >
          {{ t('admin.negociations.faq.detail.actions.toReview') }}
        </UiButton>
        <UiButton v-if="entry.status !== 'draft'" variant="secondary" icon="eye-off" @click="confirmerDepublication = true">
          {{ t('admin.negociations.faq.detail.actions.unpublish') }}
        </UiButton>
        <UiButton
          v-if="entry.status === 'draft' && !entry.first_published_at"
          variant="danger"
          icon="trash"
          @click="confirmerSuppression = true"
        >
          {{ t('admin.negociations.faq.detail.actions.delete') }}
        </UiButton>
      </div>

      <UiAlert v-if="echecDuGeste" class="mt-6" intent="danger" live :message="echecDuGeste" />
      <UiAlert v-else-if="resultat" class="mt-6" intent="success" live compact dismissible :message="resultat" />

      <div class="mt-8 grid gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]">
        <AdminNegotiationFaqForm
          class="min-w-0"
          :entry="entry"
          :readonly="!canPublish"
          :submitting="enregistrement"
          :failure="echecDuFormulaire"
          :revision="revision"
          @submit="enregistrer"
        />

        <aside class="min-w-0 space-y-6 lg:sticky lg:top-6 lg:self-start">
          <section v-if="canReview" class="rounded-lg border border-border bg-surface p-4">
            <h2 class="text-lg font-semibold">{{ t('admin.negociations.faq.detail.verify.title') }}</h2>
            <p class="mt-1 text-sm text-text-muted">{{ t('admin.negociations.faq.detail.verify.description') }}</p>
            <UiDatePicker
              v-model="verifieLe"
              class="mt-3"
              :label="t('admin.negociations.faq.detail.verify.date')"
              :hint="t('admin.negociations.faq.detail.verify.hint')"
            />
            <UiButton class="mt-3" block icon="check-circle" :loading="geste === 'verify'" @click="verifier">
              {{ t('admin.negociations.faq.detail.actions.verify') }}
            </UiButton>
          </section>

          <AdminNegotiationFaqFeedback :feedback="entry.feedback" :reports="entry.reports" />
        </aside>
      </div>
    </template>

    <UiModal
      v-model:open="confirmerDepublication"
      :title="t('admin.negociations.faq.detail.confirm.unpublish.title')"
      :description="t('admin.negociations.faq.detail.confirm.unpublish.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerDepublication = false">
          {{ t('admin.negociations.faq.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'unpublish'" @click="depublier">
          {{ t('admin.negociations.faq.detail.confirm.unpublish.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <UiModal
      v-model:open="confirmerSuppression"
      :title="t('admin.negociations.faq.detail.confirm.delete.title')"
      :description="t('admin.negociations.faq.detail.confirm.delete.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerSuppression = false">
          {{ t('admin.negociations.faq.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'delete'" @click="supprimer">
          {{ t('admin.negociations.faq.detail.confirm.delete.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>

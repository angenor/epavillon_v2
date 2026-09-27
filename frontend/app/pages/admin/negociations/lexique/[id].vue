<script setup lang="ts">
import type { AdminGlossaryEntry, AdminGlossaryInput } from '~/types/admin-negotiation-savoir'
import type { EffectivePermission } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationGlossary', to: '/admin/negociations/lexique' },
    { labelKey: 'admin.negociations.lexique.detail.crumb' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()
const { dateTime, zoneLabel } = useDateTime()

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

const { data: entry, status, error, refresh } = await useAsyncData<AdminGlossaryEntry | null>(
  () => `admin-negotiation-glossary-${entryId.value}`,
  async () => {
    if (creation.value) return null
    try {
      return await api.adminNegotiationSavoir.terme(entryId.value)
    } catch (erreur) {
      if (erreur instanceof ApiRequestError && erreur.status === 404) return null
      throw erreur
    }
  },
  { default: () => null, lazy: true },
)

useHead(() => ({
  title: creation.value
    ? t('admin.negociations.lexique.detail.newTitle')
    : entry.value?.term || t('admin.negociations.lexique.detail.crumb'),
}))

const canPublish = computed(() => entry.value?.can_publish ?? false)

const enregistrement = ref(false)
const geste = ref<string | null>(null)
const echecDuFormulaire = ref<DocumentFormFailure | null>(null)
const echecDuGeste = ref<string | null>(null)
const resultat = ref<string | null>(null)
const revision = ref(0)
const refusee = ref(false)
const confirmerDepublication = ref(false)
const confirmerSuppression = ref(false)

watch(entryId, () => {
  echecDuFormulaire.value = null
  echecDuGeste.value = null
  resultat.value = null
})

const zone = (instant: string): string =>
  t('admin.negociations.lexique.detail.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

async function enregistrer(entree: AdminGlossaryInput): Promise<void> {
  if (enregistrement.value) return
  enregistrement.value = true
  echecDuFormulaire.value = null
  resultat.value = null
  try {
    if (creation.value) {
      const cree = await api.adminNegotiationSavoir.creerTerme(entree)
      await navigateTo(localePath(`/admin/negociations/lexique/${cree.id}`))
      return
    }
    entry.value = await api.adminNegotiationSavoir.modifierTerme(entryId.value, entree)
    revision.value += 1
    resultat.value = t('admin.negociations.lexique.detail.result.saved')
  } catch (erreur) {
    if (isForbiddenError(erreur)) refusee.value = true
    echecDuFormulaire.value = {
      message: apiErrorMessage(erreur, t),
      field: erreur instanceof ApiRequestError ? erreur.field : null,
    }
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

const faire = (nom: string, appel: (id: Uuid) => Promise<AdminGlossaryEntry>, cle: string) =>
  agir(nom, async () => {
    entry.value = await appel(entryId.value)
    revision.value += 1
    return t(cle)
  })

const publier = () =>
  faire('publish', api.adminNegotiationSavoir.publierTerme, 'admin.negociations.lexique.detail.result.published')
const aRevoir = () =>
  faire('toReview', api.adminNegotiationSavoir.aRevoirTerme, 'admin.negociations.lexique.detail.result.toReview')
const depublier = () =>
  faire('unpublish', api.adminNegotiationSavoir.depublierTerme, 'admin.negociations.lexique.detail.result.unpublished')
const supprimer = () =>
  agir('delete', async () => {
    await api.adminNegotiationSavoir.supprimerTerme(entryId.value)
    await navigateTo(localePath('/admin/negociations/lexique'))
    return null
  })
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="
        refusee ||
        isForbiddenError(error) ||
        (permissionStatus !== 'pending' && (creation ? !mayPublish : !canEnter))
      "
      :required-scope="
        t(
          creation
            ? 'admin.negociations.lexique.detail.forbidden.scopePublish'
            : 'admin.negociations.lexique.list.forbidden.scope',
        )
      "
      :description="t('admin.negociations.lexique.list.forbidden.description')"
      action-to="/admin/negociations/lexique"
      :action-label="t('admin.negociations.lexique.detail.backToList')"
    />

    <UiLoadingState v-else-if="permissionStatus === 'pending' || (status === 'pending' && !entry && !creation)" />

    <template v-else-if="creation">
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.lexique.detail.newTitle') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.lexique.detail.newSubtitle') }}</p>
      </header>
      <AdminNegotiationGlossaryForm
        class="mt-8"
        :entry="null"
        :submitting="enregistrement"
        :failure="echecDuFormulaire"
        @submit="enregistrer"
      >
        <template #actions>
          <UiButton variant="ghost" :to="localePath('/admin/negociations/lexique')">
            {{ t('admin.negociations.lexique.detail.cancel') }}
          </UiButton>
        </template>
      </AdminNegotiationGlossaryForm>
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
      :title="t('admin.negociations.lexique.detail.notFound.title')"
      :description="t('admin.negociations.lexique.detail.notFound.description')"
      :action-label="t('admin.negociations.lexique.detail.backToList')"
      :action-to="localePath('/admin/negociations/lexique')"
    />

    <template v-else>
      <header class="min-w-0">
        <div class="flex flex-wrap items-center gap-3">
          <h1 class="text-3xl leading-tight font-semibold text-balance break-words">{{ entry.term }}</h1>
          <UiBadge
            :intent="KNOWLEDGE_STATUS_INTENT[entry.status]"
            :label="t(`admin.negociations.lexique.state.${entry.status}`)"
          />
        </div>
        <p class="mt-1 text-sm text-text-muted">
          {{ t('admin.negociations.lexique.detail.slug') }}
          <code class="font-mono text-text">{{ entry.slug }}</code>
          · {{ t('admin.negociations.lexique.detail.updated', { date: zone(entry.updated_at) }) }}
        </p>
        <p v-if="entry.first_published_at" class="mt-1 text-sm text-text-muted">
          {{ t('admin.negociations.lexique.detail.firstPublished', { date: zone(entry.first_published_at) }) }}
        </p>
      </header>

      <div v-if="canPublish" class="mt-6 flex flex-wrap gap-3">
        <UiButton v-if="entry.status !== 'published'" icon="check" :loading="geste === 'publish'" @click="publier">
          {{ t('admin.negociations.lexique.detail.actions.publish') }}
        </UiButton>
        <UiButton
          v-if="entry.status === 'published'"
          variant="secondary"
          icon="warning"
          :loading="geste === 'toReview'"
          @click="aRevoir"
        >
          {{ t('admin.negociations.lexique.detail.actions.toReview') }}
        </UiButton>
        <UiButton
          v-if="entry.status !== 'draft'"
          variant="secondary"
          icon="eye-off"
          @click="confirmerDepublication = true"
        >
          {{ t('admin.negociations.lexique.detail.actions.unpublish') }}
        </UiButton>
        <UiButton
          v-if="entry.status === 'draft' && !entry.first_published_at"
          variant="danger"
          icon="trash"
          @click="confirmerSuppression = true"
        >
          {{ t('admin.negociations.lexique.detail.actions.delete') }}
        </UiButton>
      </div>

      <UiAlert v-if="echecDuGeste" class="mt-6" intent="danger" live :message="echecDuGeste" />
      <UiAlert v-else-if="resultat" class="mt-6" intent="success" live compact dismissible :message="resultat" />

      <AdminNegotiationGlossaryForm
        class="mt-8"
        :entry="entry"
        :readonly="!canPublish"
        :submitting="enregistrement"
        :failure="echecDuFormulaire"
        :revision="revision"
        @submit="enregistrer"
      />
    </template>

    <UiModal
      v-model:open="confirmerDepublication"
      :title="t('admin.negociations.lexique.detail.confirm.unpublish.title')"
      :description="t('admin.negociations.lexique.detail.confirm.unpublish.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerDepublication = false">
          {{ t('admin.negociations.lexique.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'unpublish'" @click="depublier">
          {{ t('admin.negociations.lexique.detail.confirm.unpublish.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <UiModal
      v-model:open="confirmerSuppression"
      :title="t('admin.negociations.lexique.detail.confirm.delete.title')"
      :description="t('admin.negociations.lexique.detail.confirm.delete.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerSuppression = false">
          {{ t('admin.negociations.lexique.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'delete'" @click="supprimer">
          {{ t('admin.negociations.lexique.detail.confirm.delete.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>

<script setup lang="ts">
import type { AdminDocument, AdminDocumentInput } from '~/types/admin-negotiation-documents'
import type { EffectivePermission } from '~/types/identity'
import type { TimeZoneName, Uuid } from '~/types/shared'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

/**
 * La fiche d'un document. L'extraction est relue tant qu'elle tourne : le
 * worker la mène seul, et rien ne prévient l'écran quand elle se termine.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationDocuments', to: '/admin/negociations/documents' },
    { labelKey: 'admin.negociations.documents.form.detail.title' },
  ],
})

const RELECTURE_MS = 4000
const ECHECS_TOLERES = 3

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const apiUrl = useApiUrl()
const { dateTime, zoneLabel } = useDateTime()

const documentId = computed<Uuid>(() => String(route.params.id ?? ''))

const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canEnter = computed(
  () =>
    hasPermission(granted.value, 'negotiation.document.publish') ||
    hasPermission(granted.value, 'negotiation.correction.post'),
)

const { data: document, status, error, refresh } = await useAsyncData<AdminDocument | null>(
  () => `admin-negotiation-document-${documentId.value}`,
  async () => {
    try {
      return await api.adminNegotiationDocuments.document(documentId.value)
    } catch (erreur) {
      // Un document supprimé et une adresse forgée se confondent, et c'est voulu.
      if (erreur instanceof ApiRequestError && erreur.status === 404) return null
      throw erreur
    }
  },
  { default: () => null, lazy: true },
)

const titre = computed(() => resolveI18nText(document.value?.title, locale.value))

useHead(() => ({ title: titre.value || t('admin.negociations.documents.form.detail.title') }))

const canPublish = computed(() => document.value?.can_publish ?? false)
const pdfUrl = computed(() => (document.value?.asset_id ? apiUrl(api.adminNegotiationDocuments.pdf(documentId.value)) : null))

const enregistrement = ref(false)
const depot = ref(false)
const geste = ref<string | null>(null)
const echecDuFormulaire = ref<DocumentFormFailure | null>(null)
const echecDuGeste = ref<string | null>(null)
const resultat = ref<string | null>(null)
const revision = ref(0)
const confirmerDepublication = ref(false)
const confirmerSuppression = ref(false)
/** Le message posé annonce une extraction : il se remplace quand elle se conclut. */
const annonceUneExtraction = ref(false)

function surArrivee(): void {
  echecDuFormulaire.value = null
  echecDuGeste.value = null
  resultat.value = null
  annonceUneExtraction.value = false
  // « Nouvelle version » mène ici ; son message appartient à la fiche d'arrivée.
  if (route.query.nouvelle === '1') {
    resultat.value = t('admin.negociations.documents.form.detail.result.newVersion')
    if (import.meta.client) {
      const { nouvelle: _, ...reste } = route.query
      void nextTick(() => router.replace({ query: reste }))
    }
  }
}

watch(documentId, surArrivee, { immediate: true })

const zone = (instant: string): string =>
  t('admin.negociations.documents.form.detail.zoned', {
    date: dateTime(instant, timezone.value),
    zone: zoneLabel(timezone.value),
  })

const echecDe = (erreur: unknown): DocumentFormFailure => ({
  message: apiErrorMessage(erreur, t),
  field: erreur instanceof ApiRequestError ? erreur.field : null,
})

// --- L'extraction, relue tant qu'elle tourne --------------------------------

const extractionEnCours = computed(() => {
  const etat = document.value?.extraction?.status
  return etat === 'pending' || etat === 'extracting'
})

const relectureArretee = ref(false)
let minuterie: ReturnType<typeof setTimeout> | null = null
let relectureEnVol = false
let echecsDeRelecture = 0
/** Compte les écritures : une relecture partie avant l'une d'elles n'a plus rien à dire. */
let ecritures = 0

function arreterLaRelecture(): void {
  if (minuterie) clearTimeout(minuterie)
  minuterie = null
}

function planifier(): void {
  arreterLaRelecture()
  if (!import.meta.client || relectureEnVol || relectureArretee.value || !extractionEnCours.value) return
  minuterie = setTimeout(relire, RELECTURE_MS)
}

async function relire(): Promise<void> {
  minuterie = null
  if (relectureEnVol) return
  relectureEnVol = true
  const depart = ecritures
  const id = documentId.value
  try {
    const frais = await api.adminNegotiationDocuments.document(id)
    echecsDeRelecture = 0
    if (depart === ecritures && id === documentId.value) document.value = frais
  } catch {
    echecsDeRelecture += 1
    if (echecsDeRelecture >= ECHECS_TOLERES) relectureArretee.value = true
  } finally {
    relectureEnVol = false
  }
  planifier()
}

function reprendreLaRelecture(): void {
  relectureArretee.value = false
  echecsDeRelecture = 0
  void relire()
}

watch(extractionEnCours, (enCours, avant) => {
  if (avant && !enCours && annonceUneExtraction.value) {
    annonceUneExtraction.value = false
    resultat.value =
      document.value?.extraction?.status === 'ready'
        ? t('admin.negociations.documents.form.detail.result.extractionReady')
        : null
  }
  planifier()
})

watch(documentId, () => {
  relectureArretee.value = false
  echecsDeRelecture = 0
  planifier()
})

onMounted(planifier)
onBeforeUnmount(arreterLaRelecture)

// --- Les gestes ----------------------------------------------------------------

async function ecrire<T>(action: () => Promise<T>): Promise<T> {
  ecritures += 1
  try {
    return await action()
  } finally {
    ecritures += 1
  }
}

async function enregistrer(entree: AdminDocumentInput): Promise<void> {
  if (enregistrement.value) return
  enregistrement.value = true
  echecDuFormulaire.value = null
  resultat.value = null
  try {
    document.value = await ecrire(() => api.adminNegotiationDocuments.modifier(documentId.value, entree))
    revision.value += 1
    resultat.value = t('admin.negociations.documents.form.detail.result.saved')
  } catch (erreur) {
    echecDuFormulaire.value = echecDe(erreur)
  } finally {
    enregistrement.value = false
  }
}

async function attacher(assetId: Uuid): Promise<void> {
  if (depot.value) return
  depot.value = true
  echecDuFormulaire.value = null
  resultat.value = null
  try {
    document.value = await ecrire(() => api.adminNegotiationDocuments.attacherLeFichier(documentId.value, assetId))
    resultat.value = t('admin.negociations.documents.form.detail.result.attached')
    annonceUneExtraction.value = extractionEnCours.value
  } catch (erreur) {
    echecDuFormulaire.value = echecDe(erreur)
  } finally {
    depot.value = false
  }
}

/** Un message nul laisse en place celui que la fiche d'arrivée a posé. */
async function agir(nom: string, action: () => Promise<string | null>): Promise<void> {
  if (geste.value) return
  geste.value = nom
  echecDuGeste.value = null
  resultat.value = null
  annonceUneExtraction.value = false
  try {
    const message = await ecrire(action)
    if (message !== null) resultat.value = message
  } catch (erreur) {
    echecDuGeste.value = apiErrorMessage(erreur, t)
  } finally {
    geste.value = null
    confirmerDepublication.value = false
    confirmerSuppression.value = false
  }
}

const relancer = () =>
  agir('retry', async () => {
    await api.adminNegotiationDocuments.relancerLExtraction(documentId.value)
    document.value = await api.adminNegotiationDocuments.document(documentId.value)
    relectureArretee.value = false
    echecsDeRelecture = 0
    return t('admin.negociations.documents.form.detail.result.retried')
  }).then(() => {
    annonceUneExtraction.value = resultat.value !== null && extractionEnCours.value
  })

const texteAgrandi = (choix: boolean | null) =>
  agir('largeText', async () => {
    document.value = await api.adminNegotiationDocuments.choisirLeTexteAgrandi(documentId.value, choix)
    if (choix === null) return t('admin.negociations.documents.form.detail.result.largeTextVerdict')
    return choix
      ? t('admin.negociations.documents.form.detail.result.largeTextOn')
      : t('admin.negociations.documents.form.detail.result.largeTextOff')
  })

const publier = () =>
  agir('publish', async () => {
    document.value = await api.adminNegotiationDocuments.publier(documentId.value)
    return t('admin.negociations.documents.form.detail.result.published')
  })

const depublier = () =>
  agir('unpublish', async () => {
    document.value = await api.adminNegotiationDocuments.depublier(documentId.value)
    return t('admin.negociations.documents.form.detail.result.unpublished')
  })

const nouvelleVersion = () =>
  agir('newVersion', async () => {
    const brouillon = await api.adminNegotiationDocuments.nouvelleVersion(documentId.value)
    await navigateTo(`${localePath(`/admin/negociations/documents/${brouillon.id}`)}?nouvelle=1`)
    return null
  })

const supprimer = () =>
  agir('delete', async () => {
    await api.adminNegotiationDocuments.supprimer(documentId.value)
    await navigateTo(localePath('/admin/negociations/documents'))
    return null
  })
</script>

<template>
  <div class="mx-auto w-full max-w-6xl">
    <UiForbiddenState
      v-if="isForbiddenError(error) || (!canEnter && permissionStatus !== 'pending')"
      :required-scope="t('admin.negociations.documents.form.forbidden.scope')"
      :description="t('admin.negociations.documents.form.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <UiLoadingState v-if="status === 'pending' && !document" />

      <UiErrorState
        v-else-if="error"
        :description="apiErrorMessage(error, t)"
        :request-id="incidentReference(error) ?? undefined"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiEmptyState
        v-else-if="!document"
        icon="search"
        :title="t('admin.negociations.documents.form.detail.notFound.title')"
        :description="t('admin.negociations.documents.form.detail.notFound.description')"
        :action-label="t('admin.negociations.documents.form.backToList')"
        :action-to="localePath('/admin/negociations/documents')"
      />

      <template v-else>
        <header class="flex flex-wrap items-start justify-between gap-x-6 gap-y-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-3">
              <h1 class="text-3xl leading-tight font-semibold text-balance break-words">{{ titre }}</h1>
              <UiBadge
                :intent="DOCUMENT_STATE_INTENT[document.state]"
                :label="t(`admin.negociations.documents.list.state.${document.state}`)"
              />
              <UiBadge
                v-if="document.restricted"
                intent="neutral"
                icon="lock"
                :label="t('admin.negociations.documents.list.restricted')"
              />
            </div>
            <p class="mt-1 text-sm text-text-muted">
              {{ t('admin.negociations.documents.form.detail.version', { version: document.version }) }}
              · {{ t('admin.negociations.documents.form.detail.updated', { date: zone(document.updated_at) }) }}
            </p>
            <p v-if="document.published_at" class="mt-1 text-sm text-text-muted">
              {{ t('admin.negociations.documents.form.detail.published', { date: zone(document.published_at) }) }}
            </p>
            <p v-if="document.unpublished_at" class="mt-1 text-sm text-text-muted">
              {{ t('admin.negociations.documents.form.detail.unpublished', { date: zone(document.unpublished_at) }) }}
            </p>
            <p v-if="document.supersedes" class="mt-1 text-sm">
              {{ t('admin.negociations.documents.form.detail.supersedes') }}
              <NuxtLink :to="localePath(`/admin/negociations/documents/${document.supersedes.id}`)">
                {{
                  t('admin.negociations.documents.form.detail.link', {
                    title: document.supersedes.title,
                    version: document.supersedes.version,
                  })
                }}
              </NuxtLink>
            </p>
            <p v-if="document.superseded_by" class="mt-1 text-sm">
              {{ t('admin.negociations.documents.form.detail.supersededBy') }}
              <NuxtLink :to="localePath(`/admin/negociations/documents/${document.superseded_by.id}`)">
                {{
                  t('admin.negociations.documents.form.detail.link', {
                    title: document.superseded_by.title,
                    version: document.superseded_by.version,
                  })
                }}
              </NuxtLink>
            </p>
          </div>

          <div class="flex flex-wrap gap-3">
            <UiButton v-if="document.source !== 'link'" variant="secondary" icon="eye" :to="localePath(`/admin/negociations/documents/${document.id}/apercu`)">
              {{ t('admin.negociations.documents.form.detail.actions.preview') }}
            </UiButton>
            <UiButton v-if="pdfUrl" variant="secondary" icon="download" :href="pdfUrl">
              {{ t('admin.negociations.documents.form.detail.actions.pdf') }}
            </UiButton>
          </div>
        </header>

        <div v-if="canPublish" class="mt-6 flex flex-wrap gap-3">
          <UiButton
            v-if="document.state !== 'published'"
            icon="check"
            :loading="geste === 'publish'"
            @click="publier"
          >
            {{ t('admin.negociations.documents.form.detail.actions.publish') }}
          </UiButton>
          <UiButton
            v-if="document.state === 'published'"
            variant="secondary"
            icon="eye-off"
            @click="confirmerDepublication = true"
          >
            {{ t('admin.negociations.documents.form.detail.actions.unpublish') }}
          </UiButton>
          <UiButton
            v-if="document.file_locked && !document.superseded_by"
            variant="secondary"
            icon="copy"
            :loading="geste === 'newVersion'"
            @click="nouvelleVersion"
          >
            {{ t('admin.negociations.documents.form.detail.actions.newVersion') }}
          </UiButton>
          <UiButton
            v-if="document.state === 'draft' && !document.file_locked"
            variant="danger"
            icon="trash"
            @click="confirmerSuppression = true"
          >
            {{ t('admin.negociations.documents.form.detail.actions.delete') }}
          </UiButton>
        </div>

        <UiAlert v-if="echecDuGeste" class="mt-6" intent="danger" live :message="echecDuGeste" />
        <UiAlert
          v-else-if="resultat"
          class="mt-6"
          intent="success"
          live
          compact
          dismissible
          :message="resultat"
        />

        <div class="mt-8 grid gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]">
          <AdminNegotiationDocumentForm
            class="min-w-0"
            :document="document"
            :readonly="!canPublish"
            :submitting="enregistrement"
            :attaching="depot"
            :failure="echecDuFormulaire"
            :revision="revision"
            @submit="enregistrer"
            @attach="attacher"
          />

          <aside class="min-w-0 lg:sticky lg:top-6 lg:self-start">
            <AdminNegotiationDocumentExtraction
              :extraction="document.extraction"
              :can-publish="canPublish"
              :file-locked="document.file_locked"
              :retrying="geste === 'retry'"
              :saving-large-text="geste === 'largeText'"
              :polling-stopped="relectureArretee"
              :timezone="timezone"
              @retry="relancer"
              @update:large-text-choice="texteAgrandi"
              @reload="reprendreLaRelecture"
            />
          </aside>
        </div>
      </template>
    </template>

    <UiModal
      v-model:open="confirmerDepublication"
      :title="t('admin.negociations.documents.form.detail.confirm.unpublish.title')"
      :description="t('admin.negociations.documents.form.detail.confirm.unpublish.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerDepublication = false">
          {{ t('admin.negociations.documents.form.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'unpublish'" @click="depublier">
          {{ t('admin.negociations.documents.form.detail.confirm.unpublish.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <UiModal
      v-model:open="confirmerSuppression"
      :title="t('admin.negociations.documents.form.detail.confirm.delete.title')"
      :description="t('admin.negociations.documents.form.detail.confirm.delete.question')"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="confirmerSuppression = false">
          {{ t('admin.negociations.documents.form.detail.confirm.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="geste === 'delete'" @click="supprimer">
          {{ t('admin.negociations.documents.form.detail.confirm.delete.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>

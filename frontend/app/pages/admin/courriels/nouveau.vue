<script setup lang="ts">
/**
 * ÉCRIRE AU DÉPOSANT D'UN DOSSIER, `/admin/courriels/nouveau?dossier=<id>` (01/11).
 *
 * Le bouton de la fiche d'un dossier mène ici. La page crée le brouillon du
 * dossier (au déposant, la personne à contacter en copie, objet = titre de
 * l'activité) puis REMPLACE son adresse par celle du brouillon, qui s'ouvre dans
 * la fenêtre de rédaction : revenir en arrière ramène au dossier, pas ici, et
 * un rechargement ne crée pas un second brouillon.
 *
 * Création au montage seulement : une écriture n'a pas sa place dans le rendu
 * serveur, qui la referait à l'hydratation.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.new.crumb' }],
})

const { t } = useI18n()
const api = useApi()
const route = useRoute()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.mailing.new.title') }))

const proposalId = computed(() => {
  const raw = route.query.dossier
  return typeof raw === 'string' && /^[0-9a-f-]{36}$/i.test(raw) ? raw : null
})

type State = 'creating' | 'missing' | 'forbidden' | 'error'
const state = ref<State>(proposalId.value ? 'creating' : 'missing')
const errorMessage = ref<string | null>(null)
let started = false

async function create(): Promise<void> {
  const id = proposalId.value
  if (!id) {
    state.value = 'missing'
    return
  }
  state.value = 'creating'
  errorMessage.value = null
  try {
    const draft = await api.adminMailings.createForProposal(id)
    await navigateTo(localePath(`/admin/courriels/${draft.id}`), { replace: true })
  } catch (thrown) {
    // 404 : dossier inexistant ou hors périmètre ; 403 : sans la permission.
    if (isForbiddenError(thrown) || (thrown instanceof ApiRequestError && thrown.status === 404)) {
      state.value = 'forbidden'
      return
    }
    errorMessage.value = apiErrorMessage(thrown, t)
    state.value = 'error'
  }
}

onMounted(() => {
  if (started) return
  started = true
  void create()
})
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <div v-if="state === 'creating'" class="space-y-4" role="status">
      <p class="text-sm text-text-muted">{{ t('admin.mailing.new.creating') }}</p>
      <UiLoadingState variant="text" :lines="4" />
    </div>

    <UiEmptyState
      v-else-if="state === 'missing'"
      icon="mail"
      :title="t('admin.mailing.new.missing.title')"
      :description="t('admin.mailing.new.missing.description')"
      :action-label="t('admin.mailing.compose.backToList')"
      @action="navigateTo(localePath('/admin/courriels'))"
    />

    <UiForbiddenState
      v-else-if="state === 'forbidden'"
      :title="t('admin.mailing.new.forbidden.title')"
      :description="t('admin.mailing.new.forbidden.description')"
      action-to="/admin/courriels"
      :action-label="t('admin.mailing.compose.backToList')"
    />

    <UiErrorState v-else :description="errorMessage ?? undefined" @retry="create()" />
  </div>
</template>

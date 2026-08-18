<script setup lang="ts">
import type {
  EditionFormError,
  EditionFormOptions,
  EditionFormPayload,
} from '~/types/admin-events'
import type { EffectivePermission } from '~/types/identity'

/**
 * CRÉATION D'UNE ÉDITION — `/admin/evenements/nouveau`.
 *
 * LA SEULE ACTION DU BACK-OFFICE DONT LA PORTÉE NE SOIT PAS CELLE D'UNE ÉDITION.
 * Une édition qui n'existe pas encore n'a aucun périmètre où vérifier un droit : la
 * création demande donc `event.event.manage` sur la portée GLOBALE. Une
 * administratrice détachée sur la seule COP31 gère son édition sans pouvoir en créer
 * d'autres — c'est la règle métier n° 8 prise par l'autre bout.
 *
 * LE CALENDRIER EST GÉNÉRÉ À LA CRÉATION. Une édition sans aucun jour n'est
 * utilisable par aucun autre écran : le planificateur n'aurait pas de colonnes, la
 * programmation publique pas de sections. La réponse dit combien de jours ont été
 * créés, et l'écran l'annonce en arrivant sur les onglets.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.events', to: '/admin/evenements' },
    { labelKey: 'admin.event.form.createTitle' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.event.form.createTitle') }))

await adminScope.ensureLoaded()

/**
 * Les listes de référence du formulaire.
 *
 * L'erreur est RENDUE, pas avalée : un chargement qui échoue laissait sinon la
 * page sur son squelette indéfiniment, sans rien dire à personne. C'est arrivé
 * pour un simple alias de fuseau horaire refusé par l'exécution.
 */
const {
  data: options,
  status,
  error: optionsError,
  refresh: refreshOptions,
} = await useAsyncData<EditionFormOptions | null>(
  'admin-edition-form-options',
  () => api.adminEvents.formOptions(),
  { lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-edition-create-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Portée GLOBALE, sans identifiant d'édition : voir l'en-tête. */
const canCreate = computed(() => hasPermission(granted.value, 'event.event.manage'))

const errors = ref<EditionFormError[]>([])
const busy = ref(false)

async function submit(payload: EditionFormPayload): Promise<void> {
  busy.value = true
  errors.value = []
  try {
    const result = await api.adminEvents.save(payload, auth.person?.id ?? null, adminScope.scope)
    if (!result.ok || !result.edition) {
      errors.value = result.errors
      return
    }
    // Le périmètre porte une édition de plus : sans ce rechargement, le sélecteur
    // de la tête de page ignorerait celle qu'on vient de créer.
    await adminScope.reload()
    await navigateTo({
      path: localePath(`/admin/evenements/${result.edition.id}`),
      query: result.days_created > 0 ? { jours: String(result.days_created) } : {},
    })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="status !== 'pending' && !canCreate"
      :required-scope="t('admin.event.list.forbidden.scope')"
      action-to="/admin/evenements"
      :action-label="t('admin.event.form.actions.backToList')"
    />

    <template v-else>
      <header>
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.event.form.createTitle') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.event.form.createSubtitle') }}
        </p>
      </header>

      <UiErrorState
        v-if="optionsError"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refreshOptions()"
      />

      <UiLoadingState v-else-if="!options" class="mt-8" :label="t('common.states.loading.label')" />

      <AdminEventsEditionForm
        v-else
        class="mt-8"
        :options="options"
        :errors="errors"
        :busy="busy"
        is-creation
        @submit="submit"
        @cancel="navigateTo(localePath('/admin/evenements'))"
      />
    </template>
  </div>
</template>

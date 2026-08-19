<script setup lang="ts">
import type {
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseSessionOption,
  ShowcaseValidationError,
} from '~/types/admin-showcase'
import type { HighlightPlacement } from '~/types/content'
import type { EffectivePermission } from '~/types/identity'
import type { EventId } from '~/types/shared'
import { ForbiddenError } from '~/composables/useApi'

/**
 * CRÉER UNE DIAPOSITIVE — `/admin/vitrine/nouveau`.
 *
 * L'ÉCRAN NE CONTIENT PAS DE FORMULAIRE : il en charge un. Le corps de saisie est
 * `AdminShowcaseForm`, partagé avec la modification — deux jeux de six sections
 * pour la même table, c'est la garantie qu'ils divergeront au premier ajout de
 * colonne.
 *
 * L'EMPLACEMENT ARRIVE PAR L'URL (`?emplacement=bandeau|panneau`). C'est la
 * liste qui l'y met, pour qu'on revienne dans l'onglet d'où l'on est parti. Le
 * paramètre est en français, comme partout dans ce projet : ces adresses
 * s'échangent entre collègues.
 *
 * LA CRÉATION SE PLACE EN FIN D'EMPLACEMENT, et c'est l'API qui le décide : la
 * placer en tête déplacerait silencieusement tout le reste du bandeau. Le rang
 * se règle ensuite, dans la liste, avec les deux boutons qui sont la raison
 * d'être de cet écran.
 *
 * UN COMPTE DÉTACHÉ NE CRÉE PAS DE CONTENU DE PLATEFORME. `form(null, scope)`
 * ouvre alors le formulaire sur SON édition, et l'option « toute la plateforme »
 * n'est pas offerte — la refuser après l'avoir proposée serait une impasse.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.showcase', to: '/admin/vitrine' },
    { labelKey: 'admin.showcase.form.titleNew' },
  ],
})

defineI18nRoute({ paths: { fr: '/admin/vitrine/nouveau', en: '/admin/showcase/new' } })

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.showcase.form.titleNew') }))

await adminScope.ensureLoaded()

/** L'onglet d'arrivée, en français dans l'URL — voir l'en-tête. */
const placement = computed<HighlightPlacement>(() =>
  route.query.emplacement === 'panneau' ? 'home_aside' : 'home_hero',
)

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<ShowcaseFormScreen | null>(
  'admin-showcase-new',
  () => api.adminShowcase.form(null, adminScope.scope, { placement: placement.value }),
  { watch: [() => adminScope.scope, placement], lazy: true },
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-showcase-new-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canCreate = computed(() => hasPermissionOnAnyScope(granted.value, 'content.highlight.manage'))
const isSettling = computed(
  () => adminScope.isLoading || status.value === 'pending' || permissionStatus.value === 'pending',
)

// ---------------------------------------------------------------------------
// La cascade « édition → séance »
// ---------------------------------------------------------------------------

/**
 * Les séances vivent dans la PAGE, pas dans `screen` : changer d'édition doit
 * rafraîchir la liste sans recharger l'écran, faute de quoi la saisie en cours
 * serait perdue à chaque essai.
 */
const sessions = ref<ShowcaseSessionOption[]>([])
const sessionsLoading = ref(false)
watch(screen, (next) => (sessions.value = next ? [...next.sessions] : []), { immediate: true })

async function loadSessions(eventId: EventId | null): Promise<void> {
  if (eventId === null) {
    sessions.value = []
    return
  }
  sessionsLoading.value = true
  try {
    sessions.value = await api.adminShowcase.sessionsFor(eventId, adminScope.scope)
  } catch {
    // Hors périmètre : aucune séance à offrir. Le refus se dira à l'envoi, sur
    // le champ « édition », plutôt que par une liste vide inexpliquée.
    sessions.value = []
  } finally {
    sessionsLoading.value = false
  }
}

// ---------------------------------------------------------------------------
// L'envoi
// ---------------------------------------------------------------------------

const submitting = ref(false)
const serverErrors = ref<ShowcaseValidationError[]>([])
const formError = ref<string | null>(null)

async function submit(values: ShowcaseFormValues): Promise<void> {
  submitting.value = true
  serverErrors.value = []
  formError.value = null

  try {
    const result = await api.adminShowcase.save(values, adminScope.scope)
    if (!result.ok) {
      // Un refus de validation N'EST PAS une erreur de réseau : il se pose sur
      // les champs, et le formulaire reste rempli.
      serverErrors.value = result.errors
      return
    }
    await navigateTo(localePath('/admin/vitrine'))
  } catch (thrown) {
    formError.value =
      thrown instanceof ForbiddenError
        ? t('admin.showcase.form.error.forbidden')
        : t('admin.showcase.form.error.network')
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!isSettling && (!adminScope.canAdminister || !canCreate || screen === null)"
      :required-scope="t('admin.showcase.form.forbidden.scope')"
      :action-to="localePath('/admin/vitrine')"
      :action-label="t('admin.showcase.form.forbidden.action')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.showcase.form.titleNew') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.showcase.form.subtitleNew') }}
        </p>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiLoadingState v-else-if="isSettling" class="mt-8" variant="form" />

      <AdminShowcaseForm
        v-else-if="screen"
        class="mt-6"
        :screen="screen"
        :sessions="sessions"
        :sessions-loading="sessionsLoading"
        :submit-label="t('admin.showcase.form.submitNew')"
        :submitting="submitting"
        :server-errors="serverErrors"
        :form-error="formError"
        @submit="submit"
        @cancel="navigateTo(localePath('/admin/vitrine'))"
        @event-change="loadSessions"
      />
    </template>
  </div>
</template>

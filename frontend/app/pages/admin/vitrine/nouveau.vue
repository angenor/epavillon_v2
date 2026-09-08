<script setup lang="ts">
import type {
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseMediaPayload,
  ShowcaseSessionOption,
  ShowcaseValidationError,
} from '~/types/admin-showcase'
import type { HighlightId, HighlightPlacement } from '~/types/content'
import type { EffectivePermission } from '~/types/identity'
import type { AttachableRoleRule } from '~/types/media'
import type { EventId } from '~/types/shared'

/**
 * CRÉER UNE DIAPOSITIVE — `/admin/vitrine/nouveau`.
 *
 * L'ÉCRAN NE CONTIENT PAS DE FORMULAIRE : il en charge un. Le corps de saisie est
 * `AdminShowcaseForm`, partagé avec la modification — deux jeux de six sections
 * pour la même table, c'est la garantie qu'ils divergeront au premier ajout de
 * colonne.
 *
 * L'EMPLACEMENT NE SE CHOISIT PLUS. Il arrivait par l'URL, la liste ayant deux
 * onglets ; depuis le 24/08 il n'y a plus qu'un emplacement — le bandeau
 * d'ouverture. Le panneau latéral de l'accueil s'alimente seul, et `home_aside`
 * a quitté le modèle.
 *
 * LA CRÉATION SE PLACE EN FIN D'EMPLACEMENT, et c'est l'API qui le décide : la
 * placer en tête déplacerait silencieusement tout le reste du bandeau. Le rang
 * se règle ensuite, dans la liste, avec les deux boutons qui sont la raison
 * d'être de cet écran.
 *
 * LES MÉDIAS VIENNENT APRÈS, ET L'ORDRE EST FORCÉ. Le rattachement vise la
 * diapositive, qui n'existe pas avant d'être enregistrée. Un rattachement refusé
 * ne doit pas faire recréer la diapositive au second envoi : elle est retenue le
 * temps de l'écran, faute de quoi la liste montrerait deux fois la même.
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
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.showcase.form.titleNew') }))

await adminScope.ensureLoaded()

/** Un seul emplacement depuis le 24/08 : le bandeau d'ouverture. */
const placement = computed<HighlightPlacement>(() => 'home_hero')

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

/** Ce que chaque emplacement de média exige — `media.attachable_roles`. */
const { data: mediaRules } = await useAsyncData<AttachableRoleRule[]>(
  'media-roles-content-highlights',
  () => api.media.roles('content', 'highlights'),
  { default: () => [], lazy: true },
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

/** La diapositive déjà créée, quand le rattachement de ses médias a échoué. */
const created = ref<HighlightId | null>(null)

async function submit(values: ShowcaseFormValues, media: ShowcaseMediaPayload): Promise<void> {
  submitting.value = true
  serverErrors.value = []
  formError.value = null

  try {
    if (created.value === null) {
      const result = await api.adminShowcase.save(values, adminScope.scope)
      if (!result.ok || !result.row) {
        // Un refus de validation N'EST PAS une erreur de réseau : il se pose sur
        // les champs, et le formulaire reste rempli.
        serverErrors.value = result.errors
        return
      }
      created.value = result.row.id
    }
    // LES MÉDIAS APRÈS, ici : le rattachement vise la diapositive. Voir l'en-tête.
    if (Object.keys(media).length > 0) {
      await api.adminShowcase.saveMedia(created.value, media, values.event_id, adminScope.scope)
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
        :media-rules="mediaRules"
        :highlight-id="created"
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

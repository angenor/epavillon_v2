<script setup lang="ts">
import type {
  ShowcaseFormScreen,
  ShowcaseFormValues,
  ShowcaseSessionOption,
  ShowcaseValidationError,
} from '~/types/admin-showcase'
import type { EffectivePermission } from '~/types/identity'
import type { EventId } from '~/types/shared'
import { ForbiddenError } from '~/composables/useApi'

/**
 * MODIFIER UNE DIAPOSITIVE — `/admin/vitrine/[id]`.
 *
 * ── C'EST ICI QU'UNE URL FORGÉE SE PRÉSENTE, ET C'EST ICI QU'ON REFUSE ─────
 *
 * Règle métier n° 8, ADR-14. Une administratrice détachée sur la COP31 qui tape
 * l'identifiant d'une diapositive de la COP30 doit lire « ceci ne vous regarde
 * pas », et non une page vide qui laisserait croire à une suppression. Le refus
 * est porté par l'API (`assertContentInScope`), qui LÈVE `ForbiddenError` ;
 * cette page se contente de le rendre. Deux issues, jamais confondues :
 *   · `null`            → la diapositive n'existe pas ;
 *   · `ForbiddenError`  → elle existe et n'est pas dans le périmètre.
 *
 * Un contenu SANS édition (`event_id` nul) parle au nom de la plateforme
 * entière : il exige la portée globale, et le même refus s'applique.
 *
 * ── LA PERMISSION SE TESTE SUR L'ÉDITION DE LA DIAPOSITIVE ────────────────
 *
 * `content.highlight.manage`, avec `screen.values.event_id` en portée — jamais
 * un nom de rôle, jamais une permission « quelque part ». Le droit d'ENTRER sur
 * l'écran et celui de MODIFIER cette ligne sont deux questions : la première
 * ouvre la page en lecture, la seconde ouvre le bouton d'enregistrement.
 *
 * ── APRÈS ENREGISTREMENT, ON REVIENT À LA LISTE ───────────────────────────
 *
 * Et non pas « on reste avec un message vert » : ce qu'on vient vérifier après
 * avoir modifié une diapositive, c'est sa place dans le rail, et cette place ne
 * se voit que dans la liste.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.showcase', to: '/admin/vitrine' },
    { labelKey: 'admin.showcase.form.titleEdit' },
  ],
})

defineI18nRoute({ paths: { fr: '/admin/vitrine/[id]', en: '/admin/showcase/[id]' } })

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

await adminScope.ensureLoaded()

const highlightId = computed(() => String(route.params.id))

/**
 * `null` et refus ne se confondent pas — voir l'en-tête. `useAsyncData` porte
 * l'un dans `data` et l'autre dans `error` ; le gabarit lit `error.name` plutôt
 * que de rattraper ici, pour que la reprise reste possible sur une vraie panne.
 */
const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<ShowcaseFormScreen | null>(
  `admin-showcase-${highlightId.value}`,
  () => api.adminShowcase.form(highlightId.value, adminScope.scope),
  { watch: [highlightId, () => adminScope.scope], lazy: true },
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-showcase-edit-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** La permission SUR L'ÉDITION de la diapositive — `null` exige le global. */
const canEdit = computed(() =>
  hasPermission(granted.value, 'content.highlight.manage', screen.value?.values.event_id ?? null),
)

const isForbidden = computed(() => error.value?.name === 'ForbiddenError')
const isSettling = computed(
  () => adminScope.isLoading || status.value === 'pending' || permissionStatus.value === 'pending',
)

const title = computed(() => {
  const label = tr(screen.value?.values.title).trim()
  return label || t('admin.showcase.form.titleEdit')
})

useHead(() => ({ title: title.value }))

// ---------------------------------------------------------------------------
// La cascade « édition → séance »
// ---------------------------------------------------------------------------

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
    <!-- HORS PÉRIMÈTRE, OU SANS DROIT — un refus, jamais une page vide. -->
    <UiForbiddenState
      v-if="isForbidden || (!isSettling && screen !== null && !canEdit)"
      :required-scope="t('admin.showcase.form.forbidden.scope')"
      :action-to="localePath('/admin/vitrine')"
      :action-label="t('admin.showcase.form.forbidden.action')"
    />

    <!-- INTROUVABLE — la diapositive n'existe pas ; ce n'est pas un refus. -->
    <UiEmptyState
      v-else-if="!isSettling && !error && screen === null"
      icon="grid"
      :title="t('admin.showcase.form.notFound.title')"
      :description="t('admin.showcase.form.notFound.description')"
      :action-label="t('admin.showcase.form.forbidden.action')"
      :action-to="localePath('/admin/vitrine')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">{{ title }}</h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.showcase.form.subtitleEdit') }}
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
        :submit-label="t('admin.showcase.form.submitEdit')"
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

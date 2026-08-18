<script setup lang="ts">
import type {
  PlannerScreen,
  PlannerSession,
  UnplacedFilters,
  UnplacedSortKey,
} from '~/types/admin-planner'
import type { PublishProgrammeResult } from '~/types/admin-planner'
import type { EffectivePermission } from '~/types/identity'
import type { ParticipationMode } from '~/types/event/edition'
import type { ScheduleConflict } from '~/types/programme/session'
import type { PlannerSessionText } from '~/utils/planner'
import type { PlannerCalendarView } from '~/components/admin/planner/Calendar.vue'
import type { IsoDate, IsoDateTime, RoomId, Uuid } from '~/types/shared'

/**
 * LE PLANIFICATEUR DE CRÉNEAUX — `/admin/programmation`.
 *
 * L'ÉCRAN OÙ LE PAVILLON SE COMPOSE. À gauche, les activités retenues qui
 * n'ont pas encore de place ; à droite, la grille salle × heure des douze jours
 * de l'édition. On traîne une carte dans une salle, on ajuste sa durée en
 * étirant le bloc, et le programme prend forme.
 *
 * ── LA RÈGLE QUI GOUVERNE TOUT : ON NE BLOQUE PAS, ON MONTRE ────────────────
 *
 * Aucun dépôt n'est refusé, même sur un créneau occupé. Les organisations ont
 * proposé leurs horaires sans se coordonner, l'équipe réorganise, et un état
 * transitoire incohérent — deux blocs superposés le temps de recaler le second —
 * fait partie du travail. Le modèle ne pose aucune contrainte d'exclusion sur
 * les créneaux (décision structurante n° 1 de `075_programme_sessions.sql`), et
 * cet écran ne la réintroduit nulle part.
 *
 * Le bandeau de conflits est donc PERMANENT et non refermable. Le seul garde-fou
 * dur se situe à la PUBLICATION, où un point matériellement impossible retient
 * tout — c'est le seul bouton de l'écran qui peut refuser d'agir.
 *
 * ── DEUX CHEMINS POUR PLACER, TOUJOURS LES DEUX ─────────────────────────────
 *
 * Le glisser-déposer ne fonctionne ni au clavier ni sur une tablette. Le panneau
 * de séance fait donc exactement la même chose en deux temps : on choisit une
 * activité, puis son jour, sa salle et son heure. Sur écran étroit, c'est le
 * seul chemin offert — traîner un bloc dans une grille de 40 rem sur 375 px n'a
 * pas de sens.
 *
 * ── L'ÉTAT VIT DANS L'URL ───────────────────────────────────────────────────
 *
 * Le jour affiché (`?jour=2027-11-12`), la vue (`?vue=semaine`, `mois`, `annee`)
 * et la séance ouverte (`?seance=<id>`) :
 * le bandeau de conflits renvoie vers un cas précis, et une grille se transmet
 * entre deux membres de l'équipe.
 *
 * ── RÈGLE MÉTIER N° 8 ───────────────────────────────────────────────────────
 *
 * Le périmètre d'administration filtre l'écran, y compris quand l'URL est forgée :
 * `useApi()` REFUSE une édition hors périmètre plutôt que de rendre une grille
 * vide. L'arbitrage lui-même se teste par PERMISSION
 * (`programme.session.schedule`, avec sa portée), jamais par nom de rôle.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.schedule' }],
})

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const { date: formatDay } = useDateTime()
const route = useRoute()
const router = useRouter()

useHead(() => ({ title: t('admin.planner.title') }))

await adminScope.ensureLoaded()

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------

const {
  data: screen,
  status,
  error,
  refresh,
} = await useAsyncData<PlannerScreen | null>(
  'admin-planner',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return null
    return api.planner.screen(eventId, adminScope.scope)
  },
  { watch: [() => adminScope.currentEventId], lazy: true },
)

const { data: granted } = await useAsyncData<EffectivePermission[]>(
  'admin-planner-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/**
 * UNE SEULE PERMISSION POUR DEUX GESTES, ET C'EST LE MODÈLE QUI LE DIT.
 *
 * `programme.session.schedule` ouvre l'arbitrage ET la publication : le rôle
 * `programmer` est décrit en base comme celui qui « planifie les créneaux et
 * publie la programmation » (`030_identity.sql`). On ne teste donc pas deux
 * codes, et surtout PAS un nom de rôle — la règle du projet est de tester une
 * permission, avec sa portée.
 *
 * L'écran distingue quand même les deux gestes dans son interface, parce qu'ils
 * n'engagent pas la même chose : composer la grille est réversible, publier
 * envoie le programme aux délégations. Si l'IFDD veut un jour confier la
 * composition sans la publication, c'est une permission à AJOUTER au modèle,
 * pas un test de rôle à écrire ici (écart consigné dans docs/PROGRESSION.md).
 */
const canManage = computed(() =>
  hasPermission(granted.value, 'programme.session.schedule', adminScope.currentEventId),
)
const canPublish = canManage

const timezone = computed(() => screen.value?.timezone ?? 'UTC')
/**
 * « heure de Belém », et non « heure de America/Belem ». La ville de l'édition
 * nomme le fuseau ; à défaut — un cycle de webinaires n'a pas de ville —, on
 * garde le dernier segment de l'identifiant IANA, jamais le chemin entier.
 */
const zoneLabel = computed(
  () => screen.value?.zone_label?.trim() || timeZoneCityLabel(timezone.value),
)

/** Les conflits vivent à part de la réponse : chaque écriture les remplace. */
const conflicts = ref<ScheduleConflict[]>([])
watch(screen, (value) => (conflicts.value = value?.conflicts ?? []), { immediate: true })

/** Les séances aussi : une écriture rend la séance modifiée, sans tout recharger. */
const placed = ref<PlannerSession[]>([])
const unplaced = ref<PlannerSession[]>([])
watch(
  screen,
  (value) => {
    placed.value = value?.placed ?? []
    unplaced.value = value?.unplaced ?? []
  },
  { immediate: true },
)

const allSessions = computed(() => [...placed.value, ...unplaced.value])

// ---------------------------------------------------------------------------
// État porté par l'URL
// ---------------------------------------------------------------------------

function queryText(value: unknown): string {
  return typeof value === 'string' ? value : ''
}

/** Le jour affiché : celui de l'URL, à défaut le premier jour de l'édition. */
const selectedDate = computed<IsoDate>(() => {
  // Toute date bien formée est acceptée, et pas seulement un jour de l'édition :
  // en vue semaine, la colonne du lundi tombe souvent avant l'ouverture du
  // pavillon, et refuser cette date renverrait l'écran au 9 novembre à chaque
  // changement de semaine.
  const fromQuery = queryText(route.query.jour)
  if (/^\d{4}-\d{2}-\d{2}$/.test(fromQuery)) return fromQuery
  return screen.value?.days[0]?.day_date ?? new Date().toISOString().slice(0, 10)
})

const openSessionId = computed(() => queryText(route.query.seance) || null)

/**
 * LA VUE — le jour pour poser un créneau à l'heure près, la semaine pour voir
 * l'équilibre du programme. Elle vit dans l'URL au même titre que le jour : on
 * s'envoie « regarde la semaine du 16 », pas « ouvre le planificateur et bascule ».
 */
const VIEW_PARAM: Record<string, PlannerCalendarView> = {
  jour: 'day',
  semaine: 'week',
  mois: 'month',
  annee: 'year',
}
const VIEW_TO_PARAM: Record<PlannerCalendarView, string> = {
  day: 'jour',
  week: 'semaine',
  month: 'mois',
  year: 'annee',
}

const calendarView = computed<PlannerCalendarView>(
  () => VIEW_PARAM[queryText(route.query.vue)] ?? 'day',
)

function updateQuery(patch: Record<string, string | null>): void {
  const query: Record<string, string> = { ...route.query } as Record<string, string>
  for (const [key, value] of Object.entries(patch)) {
    if (value === null || value === '') delete query[key]
    else query[key] = value
  }
  router.replace({ query })
}

const openSession = computed<PlannerSession | null>(
  () => allSessions.value.find((session) => session.id === openSessionId.value) ?? null,
)

/**
 * Ouvrir un cas depuis le bandeau : on va sur SON jour avant de le sélectionner,
 * faute de quoi le bloc mis en avant serait celui d'une autre journée — c'est
 * l'écueil du lien « voir » qui ne montre rien.
 */
function focusSession(sessionId: Uuid): void {
  const session = allSessions.value.find((entry) => entry.id === sessionId)
  if (!session) return
  const day = wallClockInZone(session.starts_at, timezone.value).slice(0, 10)
  updateQuery({ jour: day, seance: sessionId })
}

// ---------------------------------------------------------------------------
// Panneau latéral : filtres, tri
// ---------------------------------------------------------------------------

const filters = ref<UnplacedFilters>({ search: '', themes: [], formats: [], organizations: [] })
const sortKey = ref<UnplacedSortKey>('score')

watch(() => adminScope.currentEventId, () => {
  filters.value = { search: '', themes: [], formats: [], organizations: [] }
})

/**
 * Les libellés que les fonctions pures ne peuvent pas produire. Deux natures qui
 * ne se confondent pas : le FORMAT est un libellé d'interface (i18n), le titre,
 * l'organisation et la thématique sont des DONNÉES de la base, résolues par
 * l'utilitaire multilingue.
 */
const sessionText = computed<PlannerSessionText>(() => ({
  format: (session) => t(`admin.planner.format.${session.format}`),
  organization: (session) =>
    session.organization_acronym?.trim() || session.organization_name || '',
  title: (session) => tr(session.title),
  theme: (badge) => tr(badge.label),
}))

const facets = computed(() => unplacedFacets(unplaced.value, sessionText.value))

const visibleUnplaced = computed(() =>
  sortUnplaced(
    filterUnplaced(unplaced.value, filters.value, sessionText.value),
    sortKey.value,
    sessionText.value,
  ),
)

// ---------------------------------------------------------------------------
// Conflits
// ---------------------------------------------------------------------------

const marks = computed(() => conflictsBySession(conflicts.value))

const conflictsOfOpenSession = computed(() =>
  openSessionId.value
    ? conflicts.value.filter(
        (conflict) =>
          conflict.session_a === openSessionId.value || conflict.session_b === openSessionId.value,
      )
    : [],
)

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

const busy = ref(false)
const actionError = ref<string | null>(null)
const actionNotice = ref<string | null>(null)

/** Range la séance modifiée du bon côté : au calendrier, ou au panneau. */
function applyMutation(session: PlannerSession, nextConflicts: ScheduleConflict[]): void {
  placed.value = placed.value.filter((entry) => entry.id !== session.id)
  unplaced.value = unplaced.value.filter((entry) => entry.id !== session.id)

  if (session.room_id === null) {
    unplaced.value = [...unplaced.value, session]
  } else {
    placed.value = [...placed.value, session].sort((a, b) => a.starts_at.localeCompare(b.starts_at))
  }
  conflicts.value = nextConflicts
}

/**
 * PLACER, DÉPLACER, REDIMENSIONNER, RETIRER — une seule écriture, comme en base.
 *
 * AUCUN CONTRÔLE PRÉALABLE. On envoie, on applique, et le bandeau dit ce que
 * cela produit comme chevauchements.
 */
async function schedule(payload: {
  sessionId: Uuid
  roomId: RoomId | null
  startsAt: IsoDateTime
  endsAt: IsoDateTime
}): Promise<void> {
  if (!canManage.value) return
  // Placer, déplacer et redimensionner sont la même écriture, mais pas la même
  // phrase : « placée » ne se dit que la première fois, quand l'activité quitte
  // le panneau latéral.
  const wasUnplaced = unplaced.value.some((entry) => entry.id === payload.sessionId)
  busy.value = true
  actionError.value = null
  try {
    const result = await api.planner.schedule({
      session_id: payload.sessionId,
      room_id: payload.roomId,
      starts_at: payload.startsAt,
      ends_at: payload.endsAt,
    })
    if (!result) return
    applyMutation(result.session, result.conflicts)
    actionNotice.value =
      payload.roomId === null
        ? t('admin.planner.notice.unplaced', { title: tr(result.session.title) })
        : wasUnplaced
          ? t('admin.planner.notice.placed', { title: tr(result.session.title) })
          : t('admin.planner.notice.moved', { title: tr(result.session.title) })
  } catch {
    actionError.value = t('admin.planner.error.schedule')
    // La grille a bougé côté bibliothèque : on la remet sur les données réelles.
    await refresh()
  } finally {
    busy.value = false
  }
}

/** Les trois écritures du panneau de séance, dans l'ordre où elles comptent. */
async function submitSessionChanges(changes: {
  schedule?: { room_id: RoomId | null; starts_at: IsoDateTime; ends_at: IsoDateTime }
  track_ids?: string[]
  broadcast?: { is_streamed: boolean; broadcast_channel_id: string | null }
}): Promise<void> {
  const session = openSession.value
  if (!session || !canManage.value) return

  busy.value = true
  actionError.value = null
  try {
    if (changes.schedule) {
      const result = await api.planner.schedule({ session_id: session.id, ...changes.schedule })
      if (result) applyMutation(result.session, result.conflicts)
    }
    if (changes.track_ids) {
      const result = await api.planner.setTracks(auth.person?.id ?? null, {
        session_id: session.id,
        track_ids: changes.track_ids,
      })
      if (result) applyMutation(result.session, result.conflicts)
    }
    if (changes.broadcast) {
      const result = await api.planner.setBroadcast({
        session_id: session.id,
        is_streamed: changes.broadcast.is_streamed,
        broadcast_channel_id: changes.broadcast.broadcast_channel_id,
      })
      if (result) applyMutation(result.session, result.conflicts)
    }
    actionNotice.value = t('admin.planner.notice.saved', { title: tr(session.title) })
    updateQuery({ seance: null })
  } catch {
    actionError.value = t('admin.planner.error.save')
  } finally {
    busy.value = false
  }
}

/** Retirer du calendrier : la séance retourne au panneau, son créneau reste. */
async function unplace(session: PlannerSession): Promise<void> {
  await schedule({
    sessionId: session.id,
    roomId: null,
    startsAt: session.starts_at,
    endsAt: session.ends_at,
  })
  updateQuery({ seance: null })
}

// ---------------------------------------------------------------------------
// Publication
// ---------------------------------------------------------------------------

const publishOpen = ref(false)
const publishResult = ref<PublishProgrammeResult | null>(null)

const { data: readiness, refresh: refreshReadiness } = await useAsyncData(
  'admin-planner-readiness',
  async () => {
    const eventId = adminScope.currentEventId
    if (!eventId) return []
    return api.planner.readiness(eventId, adminScope.scope)
  },
  { default: () => [], watch: [() => adminScope.currentEventId], lazy: true },
)

/** Ce qui deviendrait public : les séances placées et pas encore publiées. */
const readyCount = computed(
  () => placed.value.filter((session) => session.published_at === null).length,
)

async function openPublish(): Promise<void> {
  publishResult.value = null
  await refreshReadiness()
  publishOpen.value = true
}

async function publish(): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId || !canPublish.value) return

  busy.value = true
  actionError.value = null
  try {
    const result = await api.planner.publish(eventId, adminScope.scope)
    publishResult.value = result
    if (!result.blocked) {
      actionNotice.value = t('admin.planner.notice.published', result.published_count)
      await refresh()
    }
    await refreshReadiness()
  } catch {
    actionError.value = t('admin.planner.error.publish')
  } finally {
    busy.value = false
  }
}

// ---------------------------------------------------------------------------
// Écran étroit : pas de glisser-déposer
// ---------------------------------------------------------------------------

/**
 * Le glisser-déposer est offert à partir de 1024 px, et nulle part ailleurs.
 * Ce n'est pas une question de largeur mais de GESTE : sur un écran tactile, le
 * glissement HTML5 n'existe pas, et sur 375 px la grille ne tient pas. Le
 * placement en deux temps, lui, fonctionne partout.
 */
const canDrag = ref(false)
onMounted(() => {
  const query = window.matchMedia('(min-width: 1024px) and (pointer: fine)')
  canDrag.value = query.matches
  query.addEventListener('change', (media) => (canDrag.value = media.matches))
})

/** Onglet actif sur écran étroit : la liste ou la grille, jamais les deux. */
const mobileTab = ref<'unplaced' | 'calendar'>('unplaced')
</script>

<template>
  <div class="mx-auto w-full max-w-[110rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.planner.forbidden.scope')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.planner.title') }}</h1>
          <p class="mt-1 text-text-muted">{{ t('admin.planner.subtitle') }}</p>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <p v-if="screen?.programme_published_at" class="text-xs text-text-subtle">
            {{ t('admin.planner.publishedSince', {
              date: formatDay(screen.programme_published_at, timezone),
            }) }}
          </p>
          <UiButton
            v-if="canPublish"
            variant="secondary"
            icon="globe"
            :disabled="status === 'pending' || !screen"
            @click="openPublish"
          >
            {{ t('admin.planner.publish.open') }}
          </UiButton>
        </div>
      </header>

      <UiErrorState v-if="error" class="mt-8" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <UiEmptyState
        v-else-if="!screen && status !== 'pending'"
        class="mt-8"
        icon="calendar"
        :title="t('admin.planner.empty.title')"
        :description="t('admin.planner.empty.description')"
      />

      <template v-else>
        <UiAlert
          v-if="actionNotice"
          class="mt-6"
          intent="success"
          live
          dismissible
          :title="actionNotice"
          @dismiss="actionNotice = null"
        />
        <UiAlert
          v-if="actionError"
          class="mt-6"
          intent="danger"
          live
          dismissible
          :title="actionError"
          @dismiss="actionError = null"
        />

        <!-- LE BANDEAU DE CONFLITS, EN PERMANENCE ET EN TÊTE. Il ne se referme
             pas : un bandeau qu'on chasse est un bandeau qu'on ne lit plus, et
             les chevauchements réapparaîtraient le jour de la publication. -->
        <AdminPlannerConflictBanner
          class="mt-6"
          :conflicts="conflicts"
          :sessions="allSessions"
          :timezone="timezone"
          :zone-label="zoneLabel"
          :loading="status === 'pending'"
          @focus="focusSession"
        />

        <!-- Écran étroit : une chose à la fois. La grille et la liste côte à
             côte sur 375 px ne donneraient ni l'une ni l'autre. -->
        <div class="mt-4 lg:hidden">
          <UiTabs
            :model-value="mobileTab"
            :label="t('admin.planner.tabs.label')"
            :items="[
              { value: 'unplaced', label: t('admin.planner.tabs.unplaced'), count: unplaced.length },
              { value: 'calendar', label: t('admin.planner.tabs.calendar'), count: placed.length },
            ]"
            @update:model-value="(value: string) => (mobileTab = value as 'unplaced' | 'calendar')"
          />
        </div>

        <div class="mt-4 grid grid-cols-1 gap-4 lg:grid-cols-[22rem_minmax(0,1fr)]">
          <AdminPlannerUnplacedPanel
            :class="mobileTab === 'unplaced' ? '' : 'hidden lg:flex'"
            class="max-h-[48rem]"
            :sessions="visibleUnplaced"
            :facets="facets"
            :filters="filters"
            :sort="sortKey"
            :total="unplaced.length"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :draggable="canDrag && canManage"
            :loading="status === 'pending'"
            :disabled="busy || !canManage"
            :selected-id="openSessionId"
            @update:filters="(value: UnplacedFilters) => (filters = value)"
            @update:sort="(value: UnplacedSortKey) => (sortKey = value)"
            @place="(session: PlannerSession) => updateQuery({ seance: session.id })"
          />

          <AdminPlannerCalendar
            :class="mobileTab === 'calendar' ? '' : 'hidden lg:block'"
            :sessions="placed"
            :rooms="screen?.rooms ?? []"
            :days="screen?.days ?? []"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :selected-date="selectedDate"
            :view="calendarView"
            :marks="marks"
            :selected-id="openSessionId"
            :editable="canManage && canDrag"
            :busy="busy"
            @update:selected-date="(day: IsoDate) => updateQuery({ jour: day })"
            @update:view="(next: PlannerCalendarView) => updateQuery({ vue: next === 'day' ? null : VIEW_TO_PARAM[next] })"
            @schedule="schedule"
            @open="(session: PlannerSession) => updateQuery({ seance: session.id })"
          />
        </div>
      </template>
    </template>

    <AdminPlannerSessionDialog
      :open="openSession !== null"
      :session="openSession"
      :rooms="screen?.rooms ?? []"
      :days="screen?.days ?? []"
      :tracks="screen?.tracks ?? []"
      :channels="screen?.channels ?? []"
      :timezone="timezone"
      :zone-label="zoneLabel"
      :conflicts="conflictsOfOpenSession"
      :busy="busy"
      :error="actionError"
      :editable="canManage"
      @update:open="(value: boolean) => !value && updateQuery({ seance: null })"
      @submit="submitSessionChanges"
      @unplace="unplace"
    />

    <AdminPlannerPublishDialog
      v-model:open="publishOpen"
      :issues="readiness"
      :timezone="timezone"
      :zone-label="zoneLabel"
      :ready-count="readyCount"
      :published-at="screen?.programme_published_at ?? null"
      :busy="busy"
      :error="actionError"
      :result="publishResult"
      @publish="publish"
      @focus="(sessionId: Uuid) => { publishOpen = false; focusSession(sessionId) }"
    />
  </div>
</template>

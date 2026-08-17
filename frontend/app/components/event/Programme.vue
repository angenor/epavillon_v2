<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type {
  ProgrammeData,
  ProgrammeDay,
  ProgrammeEditionOption,
  ProgrammeFilterState,
} from '~/types/event-programme'
import type { PublicScheduleRow } from '~/types/views'
import type { SelectOption } from '~/types/ui'
import type { IsoDate } from '~/types/shared'

/**
 * SECTION « PROGRAMMATION » — deux vues sur LES MÊMES DONNÉES, un sélecteur
 * d'année, et une entrée pour ce qui ne relève d'aucune COP.
 *
 * ── UN SEUL JEU DE DONNÉES, DEUX LECTURES ───────────────────────────────────
 *
 * La grille et le calendrier consomment la MÊME liste filtrée. Ils ne font pas
 * deux requêtes, ne recalculent pas deux fois l'état d'une séance, et surtout
 * partagent filtres ET sélection : basculer de l'une à l'autre ne fait rien
 * perdre. C'est l'exigence du prompt, et c'est aussi la seule façon d'éviter que
 * les deux vues finissent par ne plus dire la même chose.
 *
 * ── LE SÉLECTEUR D'ANNÉE ────────────────────────────────────────────────────
 *
 * Changer d'année recharge le programme correspondant SANS quitter la page :
 * l'en-tête, l'appel et les critères restent ceux de l'édition consultée, et un
 * bandeau dit clairement qu'on lit une archive. Les programmes déjà chargés sont
 * gardés en mémoire : revenir en arrière est instantané, et l'API n'est
 * interrogée qu'une fois par édition.
 *
 * ── « AUTRES » ──────────────────────────────────────────────────────────────
 *
 * Les webinaires et cycles organisés par l'IFDD n'appartiennent à aucune COP.
 * Ils se consultent ici, dans leur propre entrée, parce qu'ils sont de la
 * programmation publique au même titre que le reste. La distinction vient du
 * MODÈLE (`event.event_series.kind`), jamais d'une liste de slugs recopiée.
 *
 * ── QUAND LE PROGRAMME N'EST PAS PUBLIÉ ─────────────────────────────────────
 *
 * La section reste présente et l'annonce. La faire disparaître laisserait croire
 * que la page est incomplète, alors que l'information — « le programme sera
 * publié après la sélection » — est justement ce qu'on vient chercher.
 */

interface Props {
  /** Édition de la page. Le sélecteur peut en afficher une autre. */
  edition: EventEdition
  /** Programme de l'édition de la page, chargé par la page (rendu serveur). */
  initial: ProgrammeData
  /** Éditions publiques, toutes séries confondues. */
  editions: EventEdition[]
  series: EventSeries[]
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const api = useApi()
const route = useRoute()
const router = useRouter()

// ---------------------------------------------------------------------------
// Les éditions consultables
// ---------------------------------------------------------------------------

const seriesById = computed(() => new Map(props.series.map((entry) => [entry.id, entry])))

/** Une COP ou une conférence onusienne : c'est le genre de la SÉRIE qui tranche. */
const CONFERENCE_KINDS = new Set<EventSeries['kind']>([
  'cop_climate',
  'cop_biodiversity',
  'cop_desertification',
  'un_conference',
])

const options = computed<ProgrammeEditionOption[]>(() =>
  props.editions.map((edition) => {
    const series = edition.series_id ? seriesById.value.get(edition.series_id) : undefined
    return {
      id: edition.id,
      slug: edition.slug,
      label: edition.acronym ?? edition.edition_label ?? tr(edition.title),
      year: edition.edition_year,
      isConference: series ? CONFERENCE_KINDS.has(series.kind) : false,
      isPublished: edition.programme_published_at !== null,
    }
  }),
)

const conferences = computed(() =>
  options.value.filter((option) => option.isConference).sort((a, b) => b.year - a.year),
)
const others = computed(() =>
  options.value.filter((option) => !option.isConference).sort((a, b) => b.year - a.year),
)

const editionById = computed(() => new Map(props.editions.map((edition) => [edition.id, edition])))

// ---------------------------------------------------------------------------
// Sélection courante et chargement
// ---------------------------------------------------------------------------

/** L'édition affichée dans la section. Elle démarre sur celle de la page. */
const selectedId = ref(props.edition.id)
const selectedEdition = computed(() => editionById.value.get(selectedId.value) ?? props.edition)
const isArchive = computed(() => selectedId.value !== props.edition.id)

/** Programmes déjà chargés — une édition n'est demandée qu'une fois. */
const loaded = reactive(new Map<string, ProgrammeData>([[props.edition.id, props.initial]]))
const loading = ref(false)
const failed = ref(false)

const data = computed<ProgrammeData>(
  () => loaded.get(selectedId.value) ?? { schedule: [], days: [], rooms: [] },
)

async function load(eventId: string): Promise<void> {
  if (loaded.has(eventId)) return
  loading.value = true
  failed.value = false
  try {
    const [schedule, days, rooms] = await Promise.all([
      api.sessions.schedule(eventId),
      api.events.days(eventId),
      api.events.rooms(eventId),
    ])
    loaded.set(eventId, { schedule, days, rooms })
  } catch {
    // Le détail technique n'apprend rien au public : l'écran propose de
    // réessayer, et l'édition consultée ne change pas tant que rien n'est chargé.
    failed.value = true
  } finally {
    loading.value = false
  }
}

async function select(eventId: string): Promise<void> {
  if (eventId === selectedId.value) return
  await load(eventId)
  if (failed.value) return
  selectedId.value = eventId
  // Les filtres appartiennent à un programme : les garder d'une édition à
  // l'autre afficherait « aucun résultat » sur une salle qui n'existe pas ici.
  filters.value = { day: null, theme: null, format: null, room: null }
  selectedSessionId.value = null
}

// ---------------------------------------------------------------------------
// Vue active, filtres, sélection — l'état PARTAGÉ par les deux vues
// ---------------------------------------------------------------------------

type ViewMode = 'grid' | 'calendar'

const view = ref<ViewMode>('grid')
const filters = ref<ProgrammeFilterState>({ day: null, theme: null, format: null, room: null })
const selectedSessionId = ref<string | null>(null)

const selectedSession = computed(
  () => data.value.schedule.find((session) => session.id === selectedSessionId.value) ?? null,
)

/**
 * L'URL porte l'édition consultée et la vue active : un lien vers « le
 * calendrier du 12 novembre » doit pouvoir se coller dans un courriel. Les
 * filtres, eux, restent locaux — les inscrire tous produirait des adresses
 * illisibles pour un gain douteux.
 */
onMounted(() => {
  const programme = route.query.programme
  const wanted = typeof programme === 'string' ? options.value.find((o) => o.slug === programme) : undefined
  if (wanted && wanted.id !== selectedId.value) void select(wanted.id)
  if (route.query.vue === 'calendrier') view.value = 'calendar'
  const day = route.query.jour
  if (typeof day === 'string') filters.value.day = day
})

watch([selectedId, view, () => filters.value.day], () => {
  const query = { ...route.query }
  if (selectedId.value === props.edition.id) delete query.programme
  else query.programme = selectedEdition.value.slug
  if (view.value === 'calendar') query.vue = 'calendrier'
  else delete query.vue
  if (filters.value.day) query.jour = filters.value.day
  else delete query.jour
  void router.replace({ query })
})

// ---------------------------------------------------------------------------
// Filtrage et regroupement
// ---------------------------------------------------------------------------

const timezone = computed(() => selectedEdition.value.timezone)
const zoneLabel = computed(() => selectedEdition.value.city ?? undefined)

/** Jour civil d'une séance DANS LE FUSEAU DE L'ÉDITION — jamais celui du visiteur. */
function dayOf(session: PublicScheduleRow): IsoDate {
  return dayKeyInZone(session.starts_at, timezone.value)
}

const filtered = computed(() =>
  data.value.schedule.filter((session) => {
    if (filters.value.day && dayOf(session) !== filters.value.day) return false
    if (filters.value.theme && !session.theme_codes.includes(filters.value.theme)) return false
    if (filters.value.format && session.format !== filters.value.format) return false
    if (filters.value.room && session.room_id !== filters.value.room) return false
    return true
  }),
)

const groupedDays = computed<ProgrammeDay[]>(() => {
  const byDate = new Map<string, PublicScheduleRow[]>()
  for (const session of filtered.value) {
    const key = dayOf(session)
    const bucket = byDate.get(key)
    if (bucket) bucket.push(session)
    else byDate.set(key, [session])
  }
  const dayIdByDate = new Map(data.value.days.map((day) => [day.day_date, day.id]))
  return [...byDate.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([dayDate, sessions]) => ({
      date: dayDate,
      dayId: dayIdByDate.get(dayDate) ?? null,
      sessions: sessions.sort((a, b) => a.starts_at.localeCompare(b.starts_at)),
    }))
})

/** Titres de journée (`event_days.title`), quand le calendrier en porte. */
const dayTitles = computed<Record<string, string>>(() => {
  const titles: Record<string, string> = {}
  for (const day of data.value.days) {
    if (day.title) titles[day.day_date] = tr(day.title)
  }
  return titles
})

// ---------------------------------------------------------------------------
// Options de filtre — tirées des données affichées, jamais écrites en dur
// ---------------------------------------------------------------------------

const allLabel = (key: string): SelectOption => ({ value: '', label: t(key) })

/** Jours qui portent au moins une activité, dans l'ordre chronologique. */
const dayOptions = computed<SelectOption[]>(() => {
  const dates = [...new Set(data.value.schedule.map(dayOf))].sort()
  return [
    allLabel('event.public.programme.filters.allDays'),
    ...dates.map((value) => ({
      value,
      label: date(`${value}T12:00:00Z`, timezone.value),
      description: dayTitles.value[value],
    })),
  ]
})

/** Thématiques présentes, avec le libellé de la BASE (`term_badges`). */
const themeOptions = computed<SelectOption[]>(() => {
  const seen = new Map<string, string>()
  for (const session of data.value.schedule) {
    for (const theme of session.themes) {
      if (!seen.has(theme.code)) seen.set(theme.code, tr(theme.label))
    }
  }
  return [
    allLabel('event.public.programme.filters.allThemes'),
    ...[...seen.entries()]
      .map(([value, label]) => ({ value, label }))
      .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
  ]
})

const formatOptions = computed<SelectOption[]>(() => {
  const present = [...new Set(data.value.schedule.map((session) => session.format))]
  return [
    allLabel('event.public.programme.filters.allFormats'),
    ...present.map((value) => ({ value, label: t(`session-card.format.${value}`) })),
  ]
})

const roomOptions = computed<SelectOption[]>(() => {
  const seen = new Map<string, string>()
  for (const session of data.value.schedule) {
    if (session.room_id && session.room_name && !seen.has(session.room_id)) {
      seen.set(session.room_id, tr(session.room_name))
    }
  }
  return [
    allLabel('event.public.programme.filters.allRooms'),
    ...[...seen.entries()].map(([value, label]) => ({ value, label })),
  ]
})

/** Journées spéciales représentées dans ce programme — pour la légende. */
const legendTracks = computed(() => {
  const seen = new Map<string, PublicScheduleRow['tracks'][number]>()
  for (const session of data.value.schedule) {
    for (const track of session.tracks) {
      if (!seen.has(track.slug)) seen.set(track.slug, track)
    }
  }
  return [...seen.values()]
})

// ---------------------------------------------------------------------------
// Vue calendrier : le jour affiché suit le filtre, et réciproquement
// ---------------------------------------------------------------------------

const availableDates = computed(() => [...new Set(data.value.schedule.map(dayOf))].sort())

const calendarDate = computed<IsoDate>(
  () => filters.value.day ?? groupedDays.value[0]?.date ?? availableDates.value[0] ?? dayKeyInZone(Date.now(), timezone.value),
)

function onCalendarDateChange(value: IsoDate): void {
  // Le calendrier navigue jour par jour : ce déplacement DEVIENT le filtre de
  // jour, sans quoi revenir à la grille annulerait ce qu'on vient de faire.
  filters.value = { ...filters.value, day: availableDates.value.includes(value) ? value : null }
}

const viewTabs = computed(() => [
  { value: 'grid', label: t('event.public.programme.views.grid'), count: filtered.value.length },
  { value: 'calendar', label: t('event.public.programme.views.calendar') },
])

const isPublished = computed(() => selectedEdition.value.programme_published_at !== null)
</script>

<template>
  <section id="programmation" class="scroll-mt-24" aria-labelledby="programmation-titre">
    <div class="flex flex-wrap items-end justify-between gap-4">
      <div>
        <h2 id="programmation-titre" class="font-display text-xl">
          {{ t('event.public.programme.title') }}
        </h2>
        <p class="mt-1 text-sm text-text-muted">{{ t('event.public.programme.description') }}</p>
      </div>

      <UiTabs
        :model-value="view"
        :items="viewTabs"
        :label="t('event.public.programme.views.label')"
        :panel-id="() => 'programmation-panneau'"
        @update:model-value="view = $event === 'calendar' ? 'calendar' : 'grid'"
      />
    </div>

    <!-- SÉLECTEUR D'ANNÉE. Les éditions passées d'abord, puis ce qui ne relève
         d'aucune conférence : deux groupes nommés, jamais une liste mêlée. -->
    <nav class="mt-5 flex flex-wrap items-center gap-x-6 gap-y-3" :aria-label="t('event.public.programme.editions.label')">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.programme.editions.conferences') }}
        </span>
        <UiButton
          v-for="option in conferences"
          :key="option.id"
          size="sm"
          :variant="option.id === selectedId ? 'primary' : 'secondary'"
          :pressed="option.id === selectedId"
          :label="`${option.label} · ${option.year}`"
          :loading="loading && option.id !== selectedId"
          @click="select(option.id)"
        />
      </div>

      <div v-if="others.length" class="flex flex-wrap items-center gap-2">
        <span class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('event.public.programme.editions.others') }}
        </span>
        <UiButton
          v-for="option in others"
          :key="option.id"
          size="sm"
          :variant="option.id === selectedId ? 'primary' : 'secondary'"
          :pressed="option.id === selectedId"
          :label="option.label"
          @click="select(option.id)"
        />
      </div>
    </nav>

    <!-- On lit une AUTRE édition que celle de la page : le dire, et proposer le
         retour. Sans cela, l'en-tête et le programme se contrediraient en
         silence. -->
    <UiAlert
      v-if="isArchive"
      class="mt-4"
      intent="info"
      :title="t('event.public.programme.editions.viewingOther', { edition: tr(selectedEdition.title) })"
    >
      {{ t('event.public.programme.editions.viewingOtherHint') }}
      <template #actions>
        <UiButton
          variant="secondary"
          size="sm"
          :label="t('event.public.programme.editions.backToCurrent', { edition: props.edition.acronym ?? tr(props.edition.title) })"
          @click="select(props.edition.id)"
        />
      </template>
    </UiAlert>

    <UiErrorState
      v-if="failed"
      class="mt-4"
      compact
      :title="t('event.public.programme.error.title')"
      :description="t('event.public.programme.error.description')"
      @retry="load(selectedId)"
    />

    <UiLoadingState
      v-else-if="loading"
      class="mt-4"
      variant="card"
      :lines="3"
      :label="t('event.public.programme.loading')"
    />

    <template v-else>
      <!-- Programme non publié : la section reste, et annonce. -->
      <UiEmptyState
        v-if="!isPublished"
        class="mt-5"
        icon="calendar"
        :title="t('event.public.programme.unpublished.title')"
        :description="t('event.public.programme.unpublished.description')"
      />

      <template v-else>
        <div class="mt-5 flex flex-col gap-4">
          <EventProgrammeFilters
            v-model="filters"
            :days="dayOptions"
            :themes="themeOptions"
            :formats="formatOptions"
            :rooms="roomOptions"
            :result-count="filtered.length"
            :total-count="data.schedule.length"
          />

          <EventProgrammeLegend :tracks="legendTracks" />
        </div>

        <div id="programmation-panneau" class="mt-6" role="region" :aria-label="t('event.public.programme.title')">
          <UiEmptyState
            v-if="!filtered.length"
            filtered
            :title="t('event.public.programme.empty.title')"
            :description="t('event.public.programme.empty.description')"
            :action-label="t('common.actions.reset')"
            @action="filters = { day: null, theme: null, format: null, room: null }"
          />

          <EventProgrammeGrid
            v-else-if="view === 'grid'"
            :days="groupedDays"
            :timezone="timezone"
            :zone-label="zoneLabel"
            :day-titles="dayTitles"
            :selected-id="selectedSessionId"
            @select="selectedSessionId = $event.id"
          />

          <!-- Le calendrier ne se rend que côté client : il mesure la hauteur des
               cellules au montage, et un rendu serveur produirait une grille à
               remplacer aussitôt. La vue grille reste l'équivalent complet. -->
          <ClientOnly v-else>
            <EventProgrammeCalendar
              :sessions="filtered"
              :timezone="timezone"
              :zone-label="zoneLabel"
              :selected-date="calendarDate"
              :min-date="availableDates[0]"
              :max-date="availableDates[availableDates.length - 1]"
              :selected-id="selectedSessionId"
              @select="selectedSessionId = $event.id"
              @update:selected-date="onCalendarDateChange"
            />
            <template #fallback>
              <UiLoadingState variant="card" :lines="2" :label="t('event.public.programme.loading')" />
            </template>
          </ClientOnly>
        </div>
      </template>
    </template>

    <!-- La fiche d'une activité, ouverte depuis l'une ou l'autre des deux vues. -->
    <UiModal
      :open="selectedSession !== null"
      size="lg"
      :title="selectedSession ? tr(selectedSession.title) : ''"
      @update:open="selectedSessionId = $event ? selectedSessionId : null"
    >
      <EventSessionDetail v-if="selectedSession" :session="selectedSession" :zone-label="zoneLabel" />
    </UiModal>
  </section>
</template>

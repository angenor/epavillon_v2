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
import type { LocationQueryRaw } from 'vue-router'

/**
 * NAVIGATEUR DE PROGRAMMATION — le corps de la page `/programmations`.
 *
 * ── POURQUOI IL N'EST PLUS UNE SECTION DE LA PAGE D'ÉDITION ─────────────────
 *
 * Il l'a été, et c'était une faute de lecture : la page de la COP31 annonçait
 * « Programmation », et le sélecteur d'année y proposait aussi la COP30 et les
 * webinaires PACO. On lisait donc le programme du cycle PACO sous le titre de la
 * COP31, sans que rien ne l'en avertisse assez fort. Un sélecteur d'édition n'a
 * de sens que sur un écran DONT LE SUJET est l'édition choisie ; la page d'une
 * édition, elle, ne porte plus qu'un lien vers ici.
 *
 * D'où la règle que cet écran s'impose : **l'édition consultée est nommée en
 * titre**, au-dessus des filtres, avec ses dates et son lieu. On ne peut pas
 * lire une activité sans avoir lu à quelle édition elle appartient.
 *
 * ── UN SEUL JEU DE DONNÉES, DEUX LECTURES ───────────────────────────────────
 *
 * La grille et le calendrier consomment la MÊME liste filtrée. Ils ne font pas
 * deux requêtes, ne recalculent pas deux fois l'état d'une séance, et surtout
 * partagent filtres ET sélection : basculer de l'une à l'autre ne fait rien
 * perdre. C'est l'exigence du prompt, et c'est aussi la seule façon d'éviter que
 * les deux vues finissent par ne plus dire la même chose.
 *
 * ── LE SÉLECTEUR D'ÉDITION ──────────────────────────────────────────────────
 *
 * Changer d'édition recharge le programme correspondant SANS quitter la page.
 * Les programmes déjà chargés sont gardés en mémoire : revenir en arrière est
 * instantané, et l'API n'est interrogée qu'une fois par édition.
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
 * L'édition reste sélectionnable et l'écran l'annonce. La retirer de la liste
 * laisserait croire qu'elle n'existe pas, alors que l'information — « le
 * programme sera publié après la sélection » — est justement ce qu'on vient
 * chercher.
 */

interface Props {
  /** Édition sélectionnée à l'arrivée, résolue par la page depuis l'URL. */
  edition: EventEdition
  /** Programme de cette édition, chargé par la page (rendu serveur). */
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
const localePath = useLocalePath()

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

const selectedId = ref(props.edition.id)
const selectedEdition = computed(() => editionById.value.get(selectedId.value) ?? props.edition)

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

/**
 * L'URL porte l'édition consultée, la vue active et le jour : un lien vers « le
 * calendrier du 12 novembre » doit pouvoir se coller dans un courriel. Les
 * autres filtres restent locaux — les inscrire tous produirait des adresses
 * illisibles pour un gain douteux.
 *
 * Lu au `setup` et non au montage : `route.query` existe aussi côté serveur, et
 * l'attendre ferait basculer la vue après un premier rendu.
 */
const view = ref<ViewMode>(route.query.vue === 'calendrier' ? 'calendar' : 'grid')
const filters = ref<ProgrammeFilterState>({
  day: typeof route.query.jour === 'string' ? route.query.jour : null,
  theme: null,
  format: null,
  room: null,
})
const selectedSessionId = ref<string | null>(null)

const selectedSession = computed(
  () => data.value.schedule.find((session) => session.id === selectedSessionId.value) ?? null,
)

function syncQuery(): void {
  const query: LocationQueryRaw = { ...route.query, edition: selectedEdition.value.slug }
  if (view.value === 'calendar') query.vue = 'calendrier'
  else delete query.vue
  if (filters.value.day) query.jour = filters.value.day
  else delete query.jour
  void router.replace({ query })
}

// Au montage aussi : entrer par la barre de navigation ouvre une édition par
// défaut, et l'adresse doit la nommer pour être copiable. Jamais côté serveur —
// une redirection au rendu ferait perdre le premier passage.
onMounted(syncQuery)
watch([selectedId, view, () => filters.value.day], syncQuery)

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
    allLabel('programme.filters.allDays'),
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
    allLabel('programme.filters.allThemes'),
    ...[...seen.entries()]
      .map(([value, label]) => ({ value, label }))
      .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
  ]
})

const formatOptions = computed<SelectOption[]>(() => {
  const present = [...new Set(data.value.schedule.map((session) => session.format))]
  return [
    allLabel('programme.filters.allFormats'),
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
    allLabel('programme.filters.allRooms'),
    ...[...seen.entries()].map(([value, label]) => ({ value, label })),
  ]
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
  { value: 'grid', label: t('programme.views.grid'), count: filtered.value.length },
  { value: 'calendar', label: t('programme.views.calendar') },
])

const isPublished = computed(() => selectedEdition.value.programme_published_at !== null)

/** « Du 9 au 20 novembre 2027 », dans le fuseau de l'édition consultée. */
const period = computed(() =>
  t('programme.editions.period', {
    start: date(selectedEdition.value.starts_at, timezone.value),
    end: date(selectedEdition.value.ends_at, timezone.value),
  }),
)
</script>

<template>
  <section aria-labelledby="programmation-titre">
    <!-- SÉLECTEUR D'ÉDITION. Les conférences d'abord, puis ce qui ne relève
         d'aucune conférence : deux groupes nommés, jamais une liste mêlée. -->
    <nav class="flex flex-wrap items-center gap-x-6 gap-y-3" :aria-label="t('programme.editions.label')">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('programme.editions.conferences') }}
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
          {{ t('programme.editions.others') }}
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

    <!-- L'ÉDITION CONSULTÉE, NOMMÉE. C'est la correction du défaut qui a motivé
         la sortie de cette section hors de la page d'édition : sans ce titre, on
         lisait le programme du cycle PACO en croyant lire celui de la COP31. -->
    <div class="mt-6 flex flex-wrap items-end justify-between gap-4 border-b border-border pb-4">
      <div class="min-w-0">
        <h2 id="programmation-titre" class="font-display text-2xl">
          {{ t('programme.editions.showing', { edition: tr(selectedEdition.title) }) }}
        </h2>
        <p class="mt-1 text-sm text-text-muted">
          <span class="tabular-nums">{{ period }}</span>
          <span v-if="selectedEdition.city"> · {{ selectedEdition.city }}</span>
        </p>
      </div>

      <UiTabs
        v-if="isPublished"
        :model-value="view"
        :items="viewTabs"
        :label="t('programme.views.label')"
        :panel-id="() => 'programmation-panneau'"
        @update:model-value="view = $event === 'calendar' ? 'calendar' : 'grid'"
      />
    </div>

    <UiErrorState
      v-if="failed"
      class="mt-6"
      compact
      :title="t('programme.error.title')"
      :description="t('programme.error.description')"
      @retry="load(selectedId)"
    />

    <UiLoadingState
      v-else-if="loading"
      class="mt-6"
      variant="card"
      :lines="3"
      :label="t('programme.loading')"
    />

    <template v-else>
      <!-- Programme non publié : l'édition reste sélectionnée, et l'écran
           renvoie vers sa page, où l'appel à propositions est encore ouvert. -->
      <UiEmptyState
        v-if="!isPublished"
        class="mt-6"
        icon="calendar"
        :title="t('programme.unpublished.title')"
        :description="t('programme.unpublished.description')"
        :action-label="t('programme.unpublished.backToEvent')"
        :action-to="localePath(`/evenements/${selectedEdition.slug}`)"
      />

      <template v-else>
        <!-- PAS DE BANDEAU DE LÉGENDE. Il énumérait les six états et les journées
             spéciales du programme, et chacun de ces repères est déjà porté par
             la séance elle-même : `UiSessionCard` écrit son état en toutes
             lettres et nomme ses journées spéciales avec leur pastille de
             couleur, le bloc du calendrier fait de même. La règle
             d'accessibilité — un repère qui ne repose pas uniquement sur la
             couleur — reste donc tenue là où elle compte : sur l'activité. -->
        <EventProgrammeFilters
          v-model="filters"
          class="mt-6"
          :days="dayOptions"
          :themes="themeOptions"
          :formats="formatOptions"
          :rooms="roomOptions"
          :result-count="filtered.length"
          :total-count="data.schedule.length"
        />

        <div id="programmation-panneau" class="mt-6" role="region" :aria-label="t('programme.title')">
          <UiEmptyState
            v-if="!filtered.length"
            filtered
            :title="t('programme.empty.title')"
            :description="t('programme.empty.description')"
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
              <UiLoadingState variant="card" :lines="2" :label="t('programme.loading')" />
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

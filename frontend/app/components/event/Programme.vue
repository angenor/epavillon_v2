<script setup lang="ts">
import type { ProgrammeData, ProgrammeStripDay, ProgrammeView } from '~/types/event-programme'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { PublicScheduleTrack } from '~/types/programme/track-page'
import type { IsoDate } from '~/types/shared'
import type { LocationQueryRaw } from 'vue-router'

/**
 * Le corps de `/programmations`, sous le bandeau qui nomme l'édition : une
 * barre d'outils (recherche, bascule, fuseau), puis la semaine ou la liste.
 * Liste éditoriale du 04/10, habillée en direction condensée le 01/11. La
 * recherche est le seul filtre ; les thématiques ont été retirées.
 *
 * Les deux vues lisent le même programme et les mêmes filtres. L'URL porte
 * l'édition, la vue et le jour : un lien vers « la liste du 12 novembre » se
 * colle dans un courriel. Un programme chargé est gardé : revenir à une
 * édition déjà vue ne rappelle pas l'API.
 */

interface Props {
  edition: PublicEditionRow
  initial: ProgrammeData
  editions: PublicEditionRow[]
  /** Changements postérieurs à la dernière visite ; zéro à la première. */
  changesSinceVisit?: number
}

const props = withDefaults(defineProps<Props>(), { changesSinceVisit: 0 })
const emit = defineEmits<{ 'update:edition': [edition: PublicEditionRow]; changes: [] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { zoneLabel, zoneOffsetShort } = useDateTime()
const { isLive, declareFrom } = useLiveSession()
const api = useApi()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()
const visitedId = useState<string | null>('programme-visited', () => null)

const editionById = computed(() => new Map(props.editions.map((edition) => [edition.id, edition])))
const selectedId = ref(props.edition.id)
const selectedEdition = computed(() => editionById.value.get(selectedId.value) ?? props.edition)

const loaded = reactive(new Map<string, ProgrammeData>([[props.edition.id, props.initial]]))
const loading = ref(false)
const failed = ref(false)
const data = computed<ProgrammeData>(() => loaded.get(selectedId.value) ?? { schedule: [], days: [], rooms: [], tracks: [] })

async function load(eventId: string): Promise<void> {
  if (loaded.has(eventId)) {
    if (eventId !== selectedId.value) void reload(eventId)
    return
  }
  loading.value = true
  failed.value = false
  try {
    const [schedule, days, rooms, tracks] = await Promise.all([
      api.sessions.schedule(eventId),
      api.events.days(eventId),
      api.events.rooms(eventId),
      api.sessions.scheduleTracks(eventId).catch(() => []),
    ])
    loaded.set(eventId, { schedule, days, rooms, tracks })
  } catch {
    failed.value = true
  } finally {
    loading.value = false
  }
}

/** Relecture au signal, sans squelette ni message : une panne garde le programme affiché. */
async function reload(eventId: string): Promise<void> {
  try {
    const [schedule, days, rooms, tracks] = await Promise.all([
      api.sessions.schedule(eventId),
      api.events.days(eventId),
      api.events.rooms(eventId),
      api.sessions.scheduleTracks(eventId).catch(() => []),
    ])
    loaded.set(eventId, { schedule, days, rooms, tracks })
  } catch {
    // Le prochain signal, ou la prochaine visite, rattrape.
  }
}

// Une édition gardée en mémoire mais quittée n'est plus suivie : elle se relit en y revenant.
useRealtime(
  () => publicEditionTopics([selectedId.value], PROGRAMME_CHANNELS),
  () => reload(selectedId.value),
)

async function select(eventId: string): Promise<void> {
  if (eventId === selectedId.value) return
  await load(eventId)
  if (failed.value) return
  selectedId.value = eventId
  emit('update:edition', selectedEdition.value)
  search.value = ''
  day.value = null
}

defineExpose({ select })

const view = ref<ProgrammeView>(route.query.vue === 'liste' ? 'list' : 'week')
const search = ref('')
const day = ref<IsoDate | null>(typeof route.query.jour === 'string' ? route.query.jour : null)

const timezone = computed(() => selectedEdition.value.timezone)
const zoneNote = computed(() => {
  const edition = selectedEdition.value
  return t('programme.toolbar.timezone', {
    zone: zoneLabel(edition.timezone, edition.city ?? undefined),
    offset: zoneOffsetShort(edition.timezone, edition.starts_at),
  })
})

const dayOf = (session: PublicScheduleRow): IsoDate => dayKeyInZone(session.starts_at, timezone.value)

const trackIndex = computed(() => indexTracks(data.value.tracks))

/** Le jour de l'atelier compte, même sans activité : son encadré y vit. */
const sideDays = computed(() =>
  trackIndex.value.sides.flatMap((side) => {
    const day = trackDayOf(side, timezone.value)
    return day ? [day] : []
  }),
)

const programmeDays = computed(() => [...new Set([...data.value.schedule.map(dayOf), ...sideDays.value])].sort())

/** Le jour ouvert : celui de l'URL s'il porte des activités, sinon le premier. */
const selectedDay = computed<IsoDate>(() => {
  const days = programmeDays.value
  return day.value && days.includes(day.value) ? day.value : (days[0] ?? dayKeyInZone(selectedEdition.value.starts_at, timezone.value))
})

const searching = computed(() => foldText(search.value.trim()) !== '')

/** Le nom d'une journée retrouve toutes ses activités : chercher « finance durable » ouvre son cadre entier. */
function matches(session: PublicScheduleRow): boolean {
  const needle = foldText(search.value.trim())
  if (!needle) return true
  const tracks = session.tracks.map((track) => tr(track.title))
  return foldText([tr(session.title), session.organization_name ?? '', session.organization_acronym ?? '', ...tracks].join(' ')).includes(needle)
}

function matchesTrack(track: PublicScheduleTrack): boolean {
  const needle = foldText(search.value.trim())
  if (!needle) return true
  return foldText([tr(track.title), track.venue_name ? tr(track.venue_name) : ''].join(' ')).includes(needle)
}

// L'horloge ne vit que dans le navigateur : l'heure du rendu serveur serait périmée à l'hydratation.
const today = ref<IsoDate | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

function tick(): void {
  today.value = dayKeyInZone(Date.now(), timezone.value)
}

const list = useTemplateRef<{ scrollToDay: (date: IsoDate, smooth?: boolean) => void }>('list')
const strip = useTemplateRef<HTMLElement>('strip')
const toolbar = useTemplateRef<HTMLElement>('toolbar')
const stickyOffset = ref(0)

// Sur téléphone la bande des jours reste figée, sur grand écran la barre d'outils : on mesure celle qui l'est.
const pinnedHeight = (element: HTMLElement | null): number =>
  element && getComputedStyle(element).position === 'sticky' ? element.offsetHeight : 0

function measure(): void {
  const root = getComputedStyle(document.documentElement)
  const nav = parseFloat(root.getPropertyValue('--nav-height')) * parseFloat(root.fontSize)
  stickyOffset.value = (Number.isFinite(nav) ? nav : 0) + pinnedHeight(strip.value) + pinnedHeight(toolbar.value)
}

async function showInList(date: IsoDate, smooth = true): Promise<void> {
  day.value = date
  if (today.value && date < today.value) pastHidden.value = false
  view.value = 'list'
  await nextTick()
  measure()
  list.value?.scrollToDay(date, smooth)
}

onMounted(() => {
  tick()
  clock = setInterval(tick, 60_000)
  const fromUrl = day.value
  if (!day.value && today.value && programmeDays.value.includes(today.value)) day.value = today.value
  syncQuery()
  measure()
  window.addEventListener('resize', measure)
  if (view.value === 'list' && fromUrl) void showInList(fromUrl, false)
})
onBeforeUnmount(() => {
  clearInterval(clock)
  window.removeEventListener('resize', measure)
})
watch(timezone, tick)

const stripDays = computed<ProgrammeStripDay[]>(() =>
  programmeDays.value.map((date, index) => {
    const tracks = trackIndex.value
    const own = data.value.schedule.filter((session) => dayOf(session) === date)
    const kept = own.filter((session) => matches(session) && !sideOf(session, tracks))
    const previous = programmeDays.value[index - 1]
    const special = own.map((session) => specialOf(session, tracks)).find((track) => track !== null) ?? null
    return {
      date,
      count: kept.length,
      colors: kept.map((session) => session.themes[0]?.color ?? null),
      dayColor: special?.color_hex ?? null,
      isSide: sideDays.value.includes(date),
      isToday: date === today.value,
      gapBefore: previous ? daysBetween(previous, date) - 1 : 0,
    }
  }),
)

/** Le calendrier montre tout le programme d'une traite, le jour de repos à sa place. */
const restDay = computed(() => {
  const edition = selectedEdition.value
  return restDayOf(programmeDays.value, dayKeyInZone(edition.starts_at, timezone.value), dayKeyInZone(edition.ends_at, timezone.value))
})

const sortedSchedule = computed(() => [...data.value.schedule].sort((a, b) => a.starts_at.localeCompare(b.starts_at)))

function syncQuery(): void {
  const query: LocationQueryRaw = { ...route.query, edition: selectedEdition.value.slug }
  if (view.value === 'list') query.vue = 'liste'
  else delete query.vue
  if (day.value) query.jour = day.value
  else delete query.jour
  void router.replace({ query })
}

watch([selectedId, view, day], syncQuery)

const isPublished = computed(() => selectedEdition.value.programme_published_at !== null)

/** Les journées passées se replient tant qu'il en reste à venir : la page s'ouvre sur aujourd'hui. */
const pastHidden = ref(true)
const hasPast = computed(() => {
  const now = today.value
  return now !== null && programmeDays.value.some((date) => date < now) && programmeDays.value.some((date) => date >= now)
})

// Même règle que l'accueil : la séance marquée en direct est déclarée une fois pour toute l'application.
watch(() => data.value.schedule, declareFrom, { immediate: true })

const liveSession = computed(() => data.value.schedule.find((session) => isLive(session.id)) ?? null)

const views = computed(() => [
  { value: 'list' as const, label: t('programme.toolbar.list') },
  { value: 'week' as const, label: t('programme.toolbar.week') },
])

</script>
<template>
  <section class="full-bleed -mb-8 bg-surface text-text sm:-mb-10" aria-labelledby="programmation-titre">
    <div class="mx-auto flex w-full max-w-[1280px] flex-col px-4 pb-14 sm:px-6 lg:pb-18">
      <!-- Figée sur grand écran dans la liste : le jour à gauche se cale dessous. -->
      <header
        ref="toolbar"
        class="z-20 flex flex-wrap items-center gap-x-4 gap-y-3 border-b border-border-subtle bg-surface py-3.5"
        :class="{ 'min-[960px]:sticky min-[960px]:top-(--nav-height)': view === 'list' }"
      >
        <h2 id="programmation-titre" class="heading-condensed text-[1.6rem] text-heading">
          {{ t('programme.board.title') }}
        </h2>
        <template v-if="isPublished && (data.schedule.length || sideDays.length)">
          <UiSearchInput
            v-model="search"
            class="min-w-0 flex-[1_1_280px] sm:max-w-90"
            :label="t('programme.filters.search')"
            hide-label
            :placeholder="t('programme.filters.searchPlaceholder')"
          />
          <div
            role="group"
            class="inline-flex shrink-0 gap-0.5 rounded-md border border-border-subtle bg-surface-sunken p-0.75"
            :aria-label="t('programme.views.label')"
          >
            <UiButton
              v-for="option in views"
              :key="option.value"
              variant="ghost"
              :pressed="view === option.value"
              class="justify-center whitespace-nowrap"
              @click="option.value === 'list' ? showInList(selectedDay) : (view = 'week')"
            >
              {{ option.label }}
            </UiButton>
          </div>
        </template>
        <div class="flex basis-full flex-col items-start min-[860px]:ml-auto min-[860px]:basis-auto min-[860px]:items-end">
          <p class="text-sm text-text-muted">{{ zoneNote }}</p>
          <button
            v-if="props.changesSinceVisit > 0"
            type="button"
            aria-haspopup="dialog"
            class="inline-flex min-h-11 cursor-pointer items-center gap-1.5 text-[0.85rem] font-bold text-live underline underline-offset-3 before:size-1.75 before:rounded-full before:bg-current hover:decoration-2 lg:min-h-10"
            @click="emit('changes')"
          >
            {{ t('programme.toolbar.sinceVisit', props.changesSinceVisit) }}
          </button>
        </div>
      </header>

      <UiErrorState
        v-if="failed"
        class="mt-8"
        compact
        :title="t('programme.error.title')"
        :description="t('programme.error.description')"
        @retry="load(selectedId)"
      />

      <UiLoadingState v-else-if="loading" class="mt-8" variant="card" :lines="3" :label="t('programme.loading')" />

      <UiEmptyState
        v-else-if="!isPublished"
        class="mt-8"
        icon="calendar"
        :title="t('programme.unpublished.title')"
        :description="t('programme.unpublished.description')"
        :action-label="t('programme.unpublished.backToEvent')"
        :action-to="localePath(`/evenements/${selectedEdition.slug}`)"
      />

      <UiEmptyState
        v-else-if="!data.schedule.length && !sideDays.length"
        class="mt-8"
        icon="calendar"
        :title="t('programme.empty.title')"
        :description="t('programme.empty.description')"
      />

      <EventProgrammeWeek
        v-else-if="view === 'week'"
        :days="programmeDays"
        :rest-day="restDay"
        :sessions="data.schedule"
        :matches="matches"
        :tracks="trackIndex"
        :matches-track="matchesTrack"
        :timezone="timezone"
        :selected="selectedDay"
        :today="today"
        :visited-id="visitedId"
        :edition-slug="selectedEdition.slug"
        :edition-past="selectedEdition.temporal_state === 'past'"
      />

      <div v-else>
        <!-- Sur téléphone, la bande des dates remplace la colonne des jours, figée. -->
        <div ref="strip" class="sticky top-(--nav-height) z-20 bg-surface min-[960px]:hidden">
          <EventProgrammeDayStrip
            :days="stripDays"
            :selected="selectedDay"
            :timezone="timezone"
            :has-past="hasPast"
            :past-hidden="pastHidden"
            @select="showInList"
            @toggle-past="pastHidden = !pastHidden"
          />
        </div>
        <button
          v-if="hasPast"
          type="button"
          class="mt-2 hidden min-h-11 cursor-pointer items-center text-sm font-bold text-accent hover:underline min-[960px]:inline-flex"
          @click="pastHidden = !pastHidden"
        >
          {{ t(pastHidden ? 'programme.days.showPast' : 'programme.days.hidePast') }}
        </button>
        <EventProgrammeLiveLead
          v-if="liveSession"
          class="mt-6"
          :session="liveSession"
          :edition-slug="selectedEdition.slug"
          :timezone="timezone"
          :city="selectedEdition.city"
        />
        <EventProgrammeDayList
          ref="list"
          :days="programmeDays"
          :sessions="sortedSchedule"
          :matches="matches"
          :tracks="trackIndex"
          :matches-track="matchesTrack"
          :searching="searching"
          :edition-start="dayKeyInZone(selectedEdition.starts_at, timezone)"
          :timezone="timezone"
          :today="today"
          :hide-past="hasPast && pastHidden"
          :visited-id="visitedId"
          :edition-slug="selectedEdition.slug"
          :edition-past="selectedEdition.temporal_state === 'past'"
          :sticky-offset="stickyOffset"
          @reading="day = $event"
          @reset="search = ''"
        />
      </div>
    </div>
  </section>
</template>

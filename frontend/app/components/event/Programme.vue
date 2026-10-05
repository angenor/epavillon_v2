<script setup lang="ts">
import type { ProgrammeData, ProgrammeStripDay, ProgrammeView } from '~/types/event-programme'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { IsoDate } from '~/types/shared'
import type { LocationQueryRaw } from 'vue-router'

/**
 * Le corps de `/programmations`, sous le bandeau qui nomme l'édition : un
 * en-tête, puis la semaine ou la liste. Direction « liste éditoriale » arbitrée
 * le 04/10 : elle remplace l'affiche, qui ne vaut plus que pour la page d'une
 * activité. La recherche est le seul filtre ; les thématiques ont été retirées.
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
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:edition': [edition: PublicEditionRow] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { zoneLabel, zoneOffsetShort, dateRange } = useDateTime()
const { isLive, setLive } = useLiveSession()
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
const data = computed<ProgrammeData>(() => loaded.get(selectedId.value) ?? { schedule: [], days: [], rooms: [] })

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
  emit('update:edition', selectedEdition.value)
  search.value = ''
  day.value = null
}

defineExpose({ select })

const view = ref<ProgrammeView>(route.query.vue === 'liste' ? 'list' : 'week')
const search = ref('')
const day = ref<IsoDate | null>(typeof route.query.jour === 'string' ? route.query.jour : null)

const timezone = computed(() => selectedEdition.value.timezone)
const subtitle = computed(() => {
  const edition = selectedEdition.value
  return t('programme.board.subtitle', {
    edition: edition.acronym ?? edition.edition_label ?? tr(edition.title),
    dates: dateRange(edition.starts_at, edition.ends_at, edition.timezone),
    zone: zoneLabel(edition.timezone, edition.city ?? undefined),
    offset: zoneOffsetShort(edition.timezone, edition.starts_at),
  })
})

const dayOf = (session: PublicScheduleRow): IsoDate => dayKeyInZone(session.starts_at, timezone.value)

const programmeDays = computed(() => [...new Set(data.value.schedule.map(dayOf))].sort())

/** Le jour ouvert : celui de l'URL s'il porte des activités, sinon le premier. */
const selectedDay = computed<IsoDate>(() => {
  const days = programmeDays.value
  return day.value && days.includes(day.value) ? day.value : (days[0] ?? dayKeyInZone(selectedEdition.value.starts_at, timezone.value))
})

function matches(session: PublicScheduleRow): boolean {
  const needle = foldText(search.value.trim())
  if (!needle) return true
  return foldText([tr(session.title), session.organization_name ?? '', session.organization_acronym ?? ''].join(' ')).includes(needle)
}

// L'horloge ne vit que dans le navigateur : l'heure du rendu serveur serait périmée à l'hydratation.
const today = ref<IsoDate | null>(null)
let clock: ReturnType<typeof setInterval> | undefined

function tick(): void {
  today.value = dayKeyInZone(Date.now(), timezone.value)
}

const list = useTemplateRef<{ scrollToDay: (date: IsoDate, smooth?: boolean) => void }>('list')
const strip = useTemplateRef<HTMLElement>('strip')
const stickyOffset = ref(0)

function measure(): void {
  const root = getComputedStyle(document.documentElement)
  const nav = parseFloat(root.getPropertyValue('--nav-height')) * parseFloat(root.fontSize)
  stickyOffset.value = (Number.isFinite(nav) ? nav : 0) + (strip.value?.offsetHeight ?? 0)
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
    const kept = data.value.schedule.filter((session) => dayOf(session) === date && matches(session))
    const previous = programmeDays.value[index - 1]
    return {
      date,
      count: kept.length,
      colors: kept.map((session) => session.themes[0]?.color ?? null),
      isToday: date === today.value,
      gapBefore: previous ? daysBetween(previous, date) - 1 : 0,
    }
  }),
)

const weeks = computed(() => [...new Set(programmeDays.value.map(weekStart))])
const weekIndex = computed(() => weeks.value.indexOf(weekStart(selectedDay.value)))

function moveWeek(step: -1 | 1): void {
  const target = weeks.value[weekIndex.value + step]
  const first = programmeDays.value.find((date) => weekStart(date) === target)
  if (first) day.value = first
}

/** Un clic sur l'en-tête d'un jour de la semaine ouvre sa liste. */
function onWeekDay(date: IsoDate): void {
  void showInList(date)
}

const weekDays = computed(() => {
  const start = weekStart(selectedDay.value)
  return programmeDays.value.filter((date) => weekStart(date) === start)
})

const weekSessions = computed(() => {
  const days = new Set(weekDays.value)
  return data.value.schedule.filter((session) => days.has(dayOf(session)))
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
watch(
  () => data.value.schedule,
  (schedule) => {
    const live = schedule.find((session) => session.status === 'live')
    if (live) setLive(live.id)
  },
  { immediate: true },
)

const liveSession = computed(() => data.value.schedule.find((session) => isLive(session.id)) ?? null)

const views = computed(() => [
  { value: 'list' as const, icon: 'list', label: t('programme.views.list') },
  { value: 'week' as const, icon: 'calendar', label: t('programme.views.week') },
])

</script>
<template>
  <section class="full-bleed -mb-8 bg-surface font-sans text-text sm:-mb-10" aria-labelledby="programmation-titre">
    <div class="mx-auto flex w-full max-w-[1280px] flex-col px-4 pt-10 pb-14 sm:px-6 lg:pt-14 lg:pb-18">
      <header class="flex flex-wrap items-end justify-between gap-4 border-b border-border-subtle pb-4">
        <div class="min-w-0">
          <h2 id="programmation-titre" class="font-sans text-[30px] leading-tight font-normal text-text">
            {{ t('programme.board.title') }}
          </h2>
          <p class="mt-1.5 text-base text-text-muted">{{ subtitle }}</p>
        </div>
        <div v-if="isPublished && data.schedule.length" class="flex w-full flex-col gap-3 sm:w-auto sm:flex-row sm:items-center sm:gap-2">
          <UiSearchInput
            v-model="search"
            class="min-w-0 sm:w-[300px]"
            :label="t('programme.filters.search')"
            hide-label
            :placeholder="t('programme.filters.searchPlaceholder')"
          />
          <!-- Sur téléphone, la bascule passe sous le titre, avant la recherche, en deux moitiés. -->
          <div
            role="group"
            class="order-first grid shrink-0 grid-cols-2 gap-0.5 rounded-[10px] border border-border p-0.75 sm:order-none sm:flex"
            :aria-label="t('programme.views.label')"
          >
            <UiButton
              v-for="option in views"
              :key="option.value"
              variant="ghost"
              :icon="option.icon"
              :pressed="view === option.value"
              class="justify-center whitespace-nowrap"
              @click="option.value === 'list' ? showInList(selectedDay) : (view = 'week')"
            >
              {{ option.label }}
            </UiButton>
          </div>
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
        v-else-if="!data.schedule.length"
        class="mt-8"
        icon="calendar"
        :title="t('programme.empty.title')"
        :description="t('programme.empty.description')"
      />

      <EventProgrammeWeek
        v-else-if="view === 'week'"
        :days="weekDays"
        :sessions="weekSessions"
        :matches="matches"
        :timezone="timezone"
        :selected="selectedDay"
        :today="today"
        :visited-id="visitedId"
        :edition-slug="selectedEdition.slug"
        :week="weekIndex + 1"
        :weeks="weeks.length"
        :has-previous="weekIndex > 0"
        :has-next="weekIndex < weeks.length - 1"
        @week="moveWeek"
        @day="onWeekDay"
      />

      <div v-else>
        <div ref="strip" class="sticky top-(--nav-height) z-20 bg-surface">
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
        <EventProgrammeLiveLead
          v-if="liveSession"
          :session="liveSession"
          :edition-slug="selectedEdition.slug"
          :timezone="timezone"
        />
        <EventProgrammeDayList
          ref="list"
          :days="programmeDays"
          :sessions="sortedSchedule"
          :matches="matches"
          :timezone="timezone"
          :today="today"
          :hide-past="hasPast && pastHidden"
          :visited-id="visitedId"
          :edition-slug="selectedEdition.slug"
          :sticky-offset="stickyOffset"
          @reading="day = $event"
          @reset="search = ''"
        />
      </div>
    </div>
  </section>
</template>

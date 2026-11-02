<script setup lang="ts">
import type { ActiveIncident } from '~/types/live'
import type { PublicTrackEntry, PublicTrackPage } from '~/types/programme/track-page'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { ProgrammeRowData } from '~/components/event/ProgrammeRow.vue'
import type { AttachedImage } from '~/types/media'
import type { SessionOrganization } from '~/types/programme/session'

/**
 * Le déroulé d'un fil (journée spéciale, atelier) : une partie = une étiquette
 * posée sur un filet à la couleur de la journée (« 1 Partie 1 », ses heures),
 * son intitulé, puis ses lignes (`EventProgrammeRow` en `compact`). Activités
 * et moments sans activité y sont mêlés, dans l'ordre de l'heure ; ce qui
 * n'appartient à aucune partie vient en tête.
 *
 * Chaque activité porte une ancre (`trackActivityAnchor`) : `/programmations`
 * y mène, et l'activité visée s'encadre quelques secondes à l'arrivée.
 *
 * Aucun « Rejoindre » par ligne : la journée se suit par un seul direct, que
 * le billet porte. Les activités gardées par qui s'est inscrit le disent.
 */

interface Props {
  page: PublicTrackPage
  entryState: (entry: PublicTrackEntry) => SessionDisplayState
  /** La photo de l'édition : la vignette des activités sans couverture. */
  editionImage?: AttachedImage | null
  /** Activités gardées par la personne inscrite à la journée ; nul : pas inscrite. */
  kept?: ReadonlySet<string> | null
  /** Messages actifs : celui d'une activité se pose sur sa ligne. */
  incidents?: ActiveIncident[]
}

const props = withDefaults(defineProps<Props>(), { editionImage: null, kept: null, incidents: () => [] })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link, duration } = useProgrammeSession()
const { partsOf } = useJointSession()
const route = useRoute()

const timezone = computed(() => props.page.edition.timezone)

/** L'atelier, sans images ni diffusion, se passe de la colonne des vignettes. */
const media = computed(() =>
  props.page.agenda.some(
    (entry) =>
      entry.kind === 'activity' &&
      (entry.activity.session.cover || entry.activity.session.is_streamed || entry.activity.session.proposals?.some((proposal) => proposal.cover)),
  ),
)

const label = (entry: SessionOrganization) =>
  organizationShortLabel({ name: entry.name, acronym: entry.acronym, acronymWellKnown: entry.acronym_well_known })
const list = computed(() => new Intl.ListFormat(locale.value, { type: 'conjunction' }))

// Une précision par ligne : « Mot de l'OIF », « Mot du Secrétariat de la CCNUCC ».
function lines(text: string): string[] {
  return text.split('\n').map((line) => line.trim()).filter(Boolean)
}

function row(entry: PublicTrackEntry, index: number): { key: string; anchor: string | undefined; highlight: boolean; data: ProgrammeRowData } {
  const current = props.entryState(entry)
  const stateLabel = current === 'live' ? t('programme.list.live') : t(`session-card.state.${current}`)
  const faded = current === 'past' || current === 'cancelled'
  if (entry.kind === 'moment') {
    return {
      key: `m-${index}`,
      anchor: undefined,
      highlight: false,
      data: {
        id: entry.moment.id,
        to: null,
        start: time(entry.starts_at, timezone.value),
        until: entry.ends_at ? t('track.agenda.until', { end: time(entry.ends_at, timezone.value) }) : '',
        title: { lead: tr(entry.moment.label), rest: '' },
        acronym: null,
        organization: null,
        country: '',
        cover: null,
        points: lines(entry.moment.description ? tr(entry.moment.description) : ''),
        current,
        faded,
        stateLabel,
      },
    }
  }
  const { session, organizations, is_highlight: highlight } = entry.activity
  const [lead, ...others] = organizations
  const to = link(props.page.edition.slug, session)
  const leadAcronym = lead ? (lead.acronym ?? null) : session.organization_acronym
  const leadName = (lead?.name ?? session.organization_name ?? '') || leadAcronym || ''
  return {
    key: `a-${session.id}`,
    anchor: trackActivityAnchor(session.slug),
    highlight,
    data: {
      id: session.id,
      to,
      start: time(session.starts_at, timezone.value),
      until: t('programme.list.untilShort', { end: time(session.ends_at, timezone.value), duration: duration(session) }),
      title: { lead: tr(session.title), rest: '' },
      acronym: leadAcronym,
      // Le nom suit le sigle, sauf s'il le répète.
      organization: leadName && leadName !== leadAcronym ? leadName : null,
      country: '',
      others: others.length ? t('track.agenda.with', { list: list.value.format(others.map(label).filter(Boolean)) }) : '',
      cover: session.cover,
      placeholder: leadAcronym ?? initialsOf(leadName),
      logo: lead?.logo ?? session.organization_logo ?? null,
      points: lines(session.summary ? tr(session.summary) : ''),
      mine: props.kept?.has(session.id) ? t('track.agenda.mine') : undefined,
      notice: incidentFor(props.incidents, session),
      current,
      faded,
      stateLabel,
      parts: partsOf(session),
    },
  }
}

const groups = computed(() => {
  const rows = props.page.agenda.map((entry, index) => ({ partId: entry.part_id, ...row(entry, index) }))
  const known = new Set(props.page.parts.map((part) => part.id))
  const loose = rows.filter((item) => !item.partId || !known.has(item.partId))
  const parts = props.page.parts.map((part) => ({
    id: part.id,
    anchor: `partie-${part.number}`,
    number: part.number,
    label: part.label ? tr(part.label) : t('track.part.default', { number: part.number }),
    title: part.title ? tr(part.title) : '',
    hours:
      part.starts_at && part.ends_at
        ? t('track.part.hours', { start: time(part.starts_at, timezone.value), end: time(part.ends_at, timezone.value) })
        : '',
    rows: rows.filter((item) => item.partId === part.id),
  }))
  return { loose, parts: parts.filter((part) => part.rows.length) }
})

const FLASH_MS = 3500
const flashed = ref<string | null>(null)
let flashTimer: ReturnType<typeof setTimeout> | undefined

/** Arrivée par une ancre d'activité : on y descend, puis l'encadré s'efface. */
async function reach(hash: string): Promise<void> {
  const id = decodeURIComponent(hash.replace(/^#/, ''))
  if (!id.startsWith(trackActivityAnchor(''))) return
  await nextTick()
  const element = document.getElementById(id)
  if (!element) return
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  element.scrollIntoView({ block: 'start', behavior: reduce ? 'auto' : 'smooth' })
  flashed.value = id
  clearTimeout(flashTimer)
  flashTimer = setTimeout(() => (flashed.value = null), FLASH_MS)
}

onMounted(() => reach(route.hash))
watch(() => route.hash, reach)
onBeforeUnmount(() => clearTimeout(flashTimer))

const itemClass = (item: { anchor: string | undefined; data: ProgrammeRowData }) => [
  item.data.to === null ? 'border-b border-dashed border-border-subtle' : 'border-b border-border-subtle last:border-b-0',
  item.data.current === 'live' ? 'border-transparent' : '',
  flashed.value && flashed.value === item.anchor ? 'rounded-md outline-accent' : 'outline-transparent',
]
</script>

<template>
  <div class="mt-7">
    <ol v-if="groups.loose.length" class="mb-10">
      <li
        v-for="item in groups.loose"
        :id="item.anchor"
        :key="item.key"
        class="scroll-mt-[calc(var(--nav-height)+1.5rem)] outline-2 outline-offset-4 motion-safe:transition-[outline-color] motion-safe:duration-700"
        :class="itemClass(item)"
      >
        <EventProgrammeRow :entry="item.data" :media="media" :highlight="item.highlight" :edition-image="props.editionImage" tone="day" compact state-surface />
      </li>
    </ol>

    <section
      v-for="(part, partIndex) in groups.parts"
      :id="part.anchor"
      :key="part.id"
      class="scroll-mt-28"
      :class="partIndex ? 'mt-10' : ''"
      :aria-labelledby="`${part.anchor}-titre`"
      :style="{ '--rule-solid': 'var(--color-day-solid)', '--rule-on': 'var(--color-day-on-solid)' }"
    >
      <header class="rule-line">
        <component :is="part.title ? 'p' : 'h3'" :id="part.title ? undefined : `${part.anchor}-titre`" class="rule-label">
          <b class="text-[1.6rem] leading-[0.9] tabular-nums" aria-hidden="true">{{ part.number }}</b>{{ part.label }}
        </component>
        <p v-if="part.hours" class="pb-1.5 text-sm text-text-secondary tabular-nums">{{ part.hours }}</p>
      </header>
      <h3
        v-if="part.title"
        :id="`${part.anchor}-titre`"
        class="mt-3.5 mb-1 font-condensed text-2xl leading-[1.1] font-semibold text-balance text-(--color-day)"
      >
        {{ part.title }}
      </h3>
      <ol>
        <li
          v-for="item in part.rows"
          :id="item.anchor"
          :key="item.key"
          class="scroll-mt-[calc(var(--nav-height)+1.5rem)] outline-2 outline-offset-4 motion-safe:transition-[outline-color] motion-safe:duration-700"
          :class="itemClass(item)"
        >
          <EventProgrammeRow :entry="item.data" :media="media" :highlight="item.highlight" :edition-image="props.editionImage" tone="day" compact state-surface />
        </li>
      </ol>
    </section>
  </div>
</template>

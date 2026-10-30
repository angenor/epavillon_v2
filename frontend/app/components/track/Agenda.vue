<script setup lang="ts">
import type { PublicTrackEntry, PublicTrackPage } from '~/types/programme/track-page'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { ProgrammeRowData } from '~/components/event/ProgrammeRow.vue'

/**
 * Le déroulé d'une journée spéciale : une partie = un grand numéro à la couleur
 * de la journée, puis les lignes de la liste éditoriale (`EventProgrammeRow`).
 * Activités et moments sans activité y sont mêlés, dans l'ordre de l'heure ; ce
 * qui n'appartient à aucune partie vient en tête.
 */

interface Props {
  page: PublicTrackPage
  entryState: (entry: PublicTrackEntry) => SessionDisplayState
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link, duration, titleParts } = useProgrammeSession()

const timezone = computed(() => props.page.edition.timezone)

/** L'atelier, sans images ni diffusion, se passe de la colonne des vignettes. */
const media = computed(() =>
  props.page.agenda.some((entry) => entry.kind === 'activity' && (entry.activity.session.cover || entry.activity.session.is_streamed)),
)

const label = (entry: { acronym?: string | null; name?: string }) => entry.acronym ?? entry.name ?? ''

function row(entry: PublicTrackEntry, index: number) {
  const current = props.entryState(entry)
  const stateLabel = current === 'live' ? t('programme.list.live') : t(`session-card.state.${current}`)
  if (entry.kind === 'moment') {
    const data: ProgrammeRowData = {
      id: entry.moment.id,
      to: null,
      start: time(entry.starts_at, timezone.value),
      until: entry.ends_at ? t('track.agenda.until', { end: time(entry.ends_at, timezone.value) }) : '',
      title: { lead: tr(entry.moment.label), rest: '' },
      acronym: null,
      organization: null,
      country: '',
      cover: null,
      current,
      faded: current === 'past',
      stateLabel,
    }
    return { key: `m-${index}`, data, highlight: false, points: entry.moment.description ? [tr(entry.moment.description)] : [] }
  }
  const { session, organizations, is_highlight: highlight } = entry.activity
  const [lead, ...others] = organizations
  const data: ProgrammeRowData = {
    id: session.id,
    to: link(props.page.edition.slug, session),
    start: time(session.starts_at, timezone.value),
    until: t('programme.list.until', { end: time(session.ends_at, timezone.value), duration: duration(session) }),
    title: titleParts(tr(session.title)),
    acronym: lead ? label(lead) : session.organization_acronym,
    placeholder: session.organization_acronym ?? lead?.acronym ?? '',
    organization: others.length ? others.map(label).join(' · ') : null,
    country: '',
    cover: session.cover,
    current,
    faded: current === 'cancelled' || current === 'past',
    stateLabel,
  }
  const summary = session.summary ? tr(session.summary) : ''
  return {
    key: `a-${session.id}`,
    data,
    highlight,
    points: summary.split('\n').map((line) => line.trim()).filter(Boolean),
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
</script>

<template>
  <div>
    <ol v-if="groups.loose.length" class="mt-8">
      <li v-for="(item, index) in groups.loose" :key="item.key" :class="{ 'border-t border-separator': index > 0 && !item.highlight }">
        <EventProgrammeRow :entry="item.data" density="day" :media="media" :highlight="item.highlight" quiet-upcoming>
          <template v-if="item.highlight" #kicker>
            <TrackHighlightMark />
          </template>
          <TrackAgendaPoints :points="item.points" />
        </EventProgrammeRow>
      </li>
    </ol>

    <section
      v-for="(part, partIndex) in groups.parts"
      :id="part.anchor"
      :key="part.id"
      class="scroll-mt-28 border-t border-text pt-8"
      :class="partIndex === 0 && !groups.loose.length ? 'mt-8' : 'mt-10'"
      :aria-labelledby="`${part.anchor}-titre`"
    >
      <header class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <p class="text-5xl leading-[0.9] font-light text-(--color-day) tabular-nums" aria-hidden="true">{{ part.number }}</p>
        <h3
          :id="`${part.anchor}-titre`"
          class="font-sans text-xs font-bold text-text-muted uppercase"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ part.label }}
        </h3>
        <p v-if="part.hours" class="text-sm text-text-muted tabular-nums">{{ part.hours }}</p>
      </header>
      <p v-if="part.title" class="mt-4 mb-2 text-xl leading-snug font-bold text-balance text-heading">{{ part.title }}</p>
      <ol :class="part.title ? '' : 'mt-4'">
        <li
          v-for="(item, index) in part.rows"
          :key="item.key"
          :class="{ 'border-t border-separator': index > 0 && !item.highlight && !part.rows[index - 1]?.highlight }"
        >
          <EventProgrammeRow :entry="item.data" density="day" :media="media" :highlight="item.highlight" quiet-upcoming>
            <template v-if="item.highlight" #kicker>
              <TrackHighlightMark />
            </template>
            <TrackAgendaPoints :points="item.points" />
          </EventProgrammeRow>
        </li>
      </ol>
    </section>
  </div>
</template>

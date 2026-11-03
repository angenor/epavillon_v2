<script setup lang="ts">
import type { TimeZoneName } from '~/types/shared'
import type { TrackBlock } from '~/utils/programme-tracks'

/**
 * Une journée spéciale ou l'atelier préparatoire dans `/programmations`, en un
 * seul bloc à la couleur du fil (maquette « journées en un bloc », 03/11) :
 * bandeau, heures, titre, sous-titre, ses temps, son temps fort, ce qu'il
 * compte, « Voir la journée ». Tout le bloc mène à la page du fil (lien
 * couvrant posé sur le titre). `card` : la colonne du calendrier ; `row` : la
 * liste, heure à gauche.
 */

interface Props {
  block: TrackBlock
  to: string
  timezone: TimeZoneName
  layout?: 'card' | 'row'
  /** Jour passé d'une édition en cours : le bloc s'estompe, comme les cartes. */
  faded?: boolean
}

const props = withDefaults(defineProps<Props>(), { layout: 'card', faded: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const track = computed(() => props.block.track)
const isDay = computed(() => track.value.kind === 'special_day')
const style = computed(() => trackDayStyle(track.value.color_hex))
const icon = computed(() => trackIcon(track.value.icon_code))
const at = (instant: string) => time(instant, props.timezone)

const parts = computed(() =>
  props.block.parts.map((part) => ({
    id: part.id,
    label: part.label ? tr(part.label) : t(part.number === 1 ? 'programme.tracks.block.partFirst' : 'programme.tracks.block.partNth', { n: part.number }),
    hours: part.starts_at && part.ends_at ? t('programme.tracks.block.hours', { start: at(part.starts_at), end: at(part.ends_at) }) : '',
  })),
)

const highlight = computed(() => {
  const value = props.block.highlight
  return value ? { time: at(value.at), title: tr(value.title) } : null
})

const counts = computed(() => {
  const { activities, moments } = props.block
  const parts = []
  // L'atelier n'a que des moments : « aucune activité » n'apprendrait rien.
  if (activities || !moments) parts.push(t('programme.tracks.block.activities', { count: activities }, activities))
  if (moments) parts.push(t('programme.tracks.block.moments', { count: moments }, moments))
  return parts.join(', ')
})

const rowNote = computed(() =>
  [highlight.value ? t('programme.tracks.block.highlightInline', { time: highlight.value.time, title: highlight.value.title }) : '', counts.value]
    .filter(Boolean)
    .join(' '),
)
</script>

<template>
  <article class="tb" :class="[`tb-${props.layout}`, { 'is-faded': props.faded }]" :style="style">
    <template v-if="props.layout === 'card'">
      <p class="tb-band">
        <UiIcon v-if="icon" :name="icon" size="1.05em" :stroke-width="2" />
        {{ t(isDay ? 'programme.tracks.kicker' : 'programme.tracks.side.kicker') }}
      </p>
      <div class="tb-body">
        <p v-if="props.block.start" class="tb-time">
          <b>{{ at(props.block.start) }}</b>
          <span v-if="props.block.end">{{ t('programme.week.until', { end: at(props.block.end) }) }}</span>
        </p>
        <h4 class="tb-title">
          <NuxtLink :to="props.to" class="tb-link">{{ tr(track.title) }}</NuxtLink>
        </h4>
        <p v-if="track.subtitle" class="tb-subtitle">{{ tr(track.subtitle) }}</p>
        <ol v-if="parts.length" class="tb-parts">
          <li v-for="part in parts" :key="part.id">
            <b>{{ part.label }}</b>
            <span v-if="part.hours">{{ part.hours }}</span>
          </li>
        </ol>
        <p v-if="highlight" class="tb-highlight">
          <UiIcon name="star" size="1rem" :stroke-width="2" />
          <span><b>{{ t('programme.tracks.block.highlight', { time: highlight.time }) }}</b><br>{{ highlight.title }}</span>
        </p>
        <div class="tb-foot">
          <span>{{ counts }}</span>
          <span class="tb-open" aria-hidden="true">
            {{ t(isDay ? 'programme.tracks.block.open' : 'programme.tracks.block.openSide') }}
            <UiIcon name="arrow-right" size="1rem" :stroke-width="2.2" />
          </span>
        </div>
      </div>
    </template>

    <template v-else>
      <p class="tb-time tb-time-row">
        <b>{{ props.block.start ? at(props.block.start) : '' }}</b>
        <span v-if="props.block.end">{{ t('programme.week.until', { end: at(props.block.end) }) }}</span>
      </p>
      <div class="min-w-0">
        <p class="tb-tag">
          <UiIcon v-if="icon" :name="icon" size="0.95em" :stroke-width="2" />
          {{ t(isDay ? 'programme.tracks.kicker' : 'programme.tracks.side.kicker') }}
        </p>
        <h4 class="tb-title">
          <NuxtLink :to="props.to" class="tb-link">{{ tr(track.title) }}</NuxtLink>
        </h4>
        <p v-if="track.subtitle" class="tb-subtitle">{{ tr(track.subtitle) }}</p>
        <ul v-if="parts.length" class="tb-parts">
          <li v-for="part in parts" :key="part.id">
            <b>{{ part.label }}</b>
            <span v-if="part.hours">{{ part.hours }}</span>
          </li>
        </ul>
        <div class="tb-foot">
          <span>{{ rowNote }}</span>
          <span class="tb-open" aria-hidden="true">
            {{ t(isDay ? 'programme.tracks.block.open' : 'programme.tracks.block.openSide') }}
            <UiIcon name="arrow-right" size="1rem" :stroke-width="2.2" />
          </span>
        </div>
      </div>
    </template>
  </article>
</template>

<style scoped>
.tb {
  position: relative;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--color-day-line) 35%, transparent);
  border-radius: var(--radius-lg);
  background: var(--color-day-surface);
  transition: border-color 200ms ease, box-shadow 200ms ease;
}

.tb:hover {
  border-color: var(--color-day-line);
  box-shadow: var(--shadow-md);
}

.tb:focus-within {
  outline: 3px solid var(--color-focus);
  outline-offset: 2px;
}

.tb.is-faded {
  opacity: 0.72;
}

.tb-band {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 12px;
  background: var(--color-day-solid);
  color: var(--color-day-on-solid);
  font-family: var(--font-condensed);
  font-size: 0.95rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.tb-body {
  padding: 12px 12px 14px;
}

.tb-time {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  font-variant-numeric: tabular-nums;
}

.tb-time b {
  font-family: var(--font-condensed);
  font-size: 1.7rem;
  font-weight: 700;
  line-height: 0.9;
  color: var(--color-day);
}

.tb-time span {
  font-size: 0.76rem;
  color: var(--color-text-muted);
}

.tb-title {
  margin-top: 8px;
  font-family: var(--font-condensed);
  font-size: 1.35rem;
  font-weight: 700;
  line-height: 1.1;
  text-transform: uppercase;
  text-wrap: balance;
  color: var(--color-day);
}

/* Lien couvrant : tout le bloc mène à la page du fil, sans second lien à tabuler. */
.tb-link {
  color: inherit;
  text-decoration: none;
  outline: none;
}

.tb-link::after {
  content: "";
  position: absolute;
  inset: 0;
}

.tb:hover .tb-link {
  text-decoration: underline;
  text-decoration-thickness: 2px;
  text-underline-offset: 4px;
}

.tb-subtitle {
  margin-top: 6px;
  font-size: 0.82rem;
  line-height: 1.4;
  color: var(--color-text-secondary);
}

.tb-parts {
  display: grid;
  gap: 7px;
  margin-top: 12px;
  padding-left: 12px;
  border-left: 3px solid var(--color-day-line);
  font-size: 0.82rem;
  line-height: 1.35;
}

.tb-parts b {
  display: block;
  font-weight: 600;
  color: var(--color-text);
}

.tb-parts span {
  color: var(--color-text-muted);
  font-variant-numeric: tabular-nums;
}

.tb-highlight {
  display: flex;
  gap: 8px;
  margin-top: 12px;
  padding: 8px 10px;
  border-radius: var(--radius-md);
  background: var(--color-surface-raised);
  font-size: 0.82rem;
  line-height: 1.35;
  color: var(--color-text-secondary);
}

.tb-highlight :deep(svg) {
  flex: none;
  margin-top: 1px;
  color: var(--color-day);
}

.tb-highlight b {
  font-weight: 700;
  color: var(--color-text);
}

.tb-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 6px 10px;
  margin-top: 12px;
  font-size: 0.8rem;
  color: var(--color-text-muted);
}

.tb-open {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 2rem;
  font-family: var(--font-condensed);
  font-size: 1rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--color-day);
}

/* La liste : l'heure à gauche, un filet épais à la couleur du fil. */
.tb-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 8px;
  padding: 18px 16px;
  border-left: 6px solid var(--color-day-solid);
}

@media (min-width: 640px) {
  .tb-row {
    grid-template-columns: 112px minmax(0, 1fr);
    gap: 20px;
    padding: 18px 20px;
  }
}

@media (min-width: 640px) {
  .tb-time-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  .tb-time-row b {
    font-size: 2.2rem;
  }
}

.tb-time-row span {
  font-size: 0.82rem;
}

.tb-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: var(--radius-full);
  background: var(--color-day-solid);
  color: var(--color-day-on-solid);
  font-size: 0.78rem;
  font-weight: 700;
}

.tb-row .tb-title {
  font-size: clamp(1.35rem, 2.4vw, 1.6rem);
}

.tb-row .tb-subtitle {
  margin-top: 4px;
  font-size: 0.92rem;
}

.tb-row .tb-parts {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 24px;
  margin-top: 14px;
  padding-left: 0;
  border-left: 0;
  font-size: 0.88rem;
}

.tb-row .tb-parts b {
  display: inline;
  margin-right: 6px;
}

.tb-row .tb-foot {
  margin-top: 14px;
  gap: 8px 16px;
  font-size: 0.85rem;
}

.tb-row .tb-open {
  font-size: 1.05rem;
}

@media (prefers-reduced-motion: reduce) {
  .tb {
    transition: none;
  }
}
</style>

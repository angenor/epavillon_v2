<script setup lang="ts">
import type { PublicTrackSummary } from '~/types/programme/track-page'

/**
 * Les autres fils publiés de l'édition, en cartes photo voilées : on passe de
 * l'atelier aux journées sans revenir à la programmation. Sans photo
 * (l'atelier), l'aplat de sa couleur, le pictogramme en filigrane.
 */

interface Props {
  tracks: PublicTrackSummary[]
  editionSlug: string
  editionLabel: string
  timezone: string
  /** L'horloge de la page : « Demain », « Terminé ». */
  now: number
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const localePath = useLocalePath()

const today = computed(() => dayKeyInZone(props.now, props.timezone))

function tag(track: PublicTrackSummary): { label: string; done: boolean } {
  const first = track.starts_at ? dayKeyInZone(track.starts_at, props.timezone) : track.starts_on
  const last = track.ends_at ? dayKeyInZone(track.ends_at, props.timezone) : (track.ends_on ?? first)
  if (last && last < today.value) return { label: t(`track.others.done.${track.kind === 'special_day' ? 'day' : 'other'}`), done: true }
  if (first && first <= today.value) return { label: t('track.others.today'), done: false }
  if (first) {
    const gap = Math.round((Date.parse(`${first}T00:00:00Z`) - Date.parse(`${today.value}T00:00:00Z`)) / 86_400_000)
    if (gap === 1) return { label: t('track.others.tomorrow'), done: false }
  }
  return { label: t(`track.kind.${track.kind}`), done: false }
}

const items = computed(() =>
  props.tracks
    .filter((track) => !track.is_current)
    .map((track) => {
      const instant = track.starts_at ? Date.parse(track.starts_at) : track.starts_on ? Date.parse(`${track.starts_on}T12:00:00Z`) : null
      const zone = track.starts_at ? props.timezone : 'UTC'
      const part = (options: Intl.DateTimeFormatOptions) =>
        instant === null ? '' : new Intl.DateTimeFormat(locale.value, { ...options, timeZone: zone }).format(instant)
      const venue = track.venue_name ? tr(track.venue_name) : ''
      const hours = track.starts_at && track.ends_at ? t('track.part.hours', { start: time(track.starts_at, props.timezone), end: time(track.ends_at, props.timezone) }) : ''
      return {
        slug: track.slug,
        to: localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: track.slug } }),
        day: part({ day: 'numeric' }),
        weekday: part({ weekday: 'long' }),
        month: part({ month: 'long' }),
        title: tr(track.title),
        line: [venue, track.is_off_site ? '' : hours].filter(Boolean).join(', '),
        icon: trackIcon(track.icon_code),
        cover: track.cover,
        tag: tag(track),
        style: trackDayStyle(track.color_hex),
      }
    }),
)
</script>

<template>
  <section v-if="items.length" aria-labelledby="journee-autres-titre">
    <h2 id="journee-autres-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">
      {{ t('track.others.title', { edition: props.editionLabel }) }}
    </h2>
    <ul class="mt-5 grid gap-5 md:grid-cols-2">
      <li v-for="item in items" :key="item.slug" :style="item.style">
        <NuxtLink
          :to="item.to"
          class="group relative isolate flex h-full min-h-70 flex-col justify-end overflow-hidden rounded-lg px-6 pt-16 pb-6 no-underline shadow-md transition duration-300 hover:shadow-lg motion-safe:hover:-translate-y-1"
          :class="item.cover
            ? 'bg-surface-inverse text-text-on-inverse hover:text-text-on-inverse'
            : 'bg-(--color-day-solid) text-(--color-day-on-solid) hover:text-(--color-day-on-solid)'"
        >
          <TrackCardPhoto v-if="item.cover" :image="item.cover" sizes="(min-width: 768px) 36rem, 100vw" />
          <UiIcon
            v-else-if="item.icon"
            :name="item.icon"
            size="13.75rem"
            :stroke-width="1.4"
            class="absolute -top-2.5 -right-6 -z-10 opacity-10"
            aria-hidden="true"
          />
          <span
            class="absolute top-4.5 left-4.5 inline-flex rounded-full px-2.5 py-1 text-[0.72rem] leading-tight font-bold uppercase"
            :class="item.tag.done ? 'border border-neutral-border bg-neutral-surface text-neutral' : 'bg-surface-inverse-selected text-text-on-inverse-selected'"
            :style="{ letterSpacing: '0.05em' }"
          >
            {{ item.tag.label }}
          </span>
          <p v-if="item.day" class="mb-2.5 flex items-baseline gap-2.5 font-condensed uppercase">
            <b class="text-[3.4rem] leading-[0.85] font-bold tabular-nums">{{ item.day }}</b>
            <span class="text-[1.05rem] leading-[1.15] font-semibold tracking-[0.04em]">{{ item.weekday }}<br>{{ item.month }}</span>
          </p>
          <h3 class="heading-condensed text-[1.85rem] text-inherit">{{ item.title }}</h3>
          <p class="mt-4.5 flex items-center justify-between gap-3 border-t border-current/25 pt-3.5 text-[0.88rem]">
            <span class="min-w-0 opacity-90">{{ item.line }}</span>
            <UiIcon name="arrow-right" size="1.2rem" class="shrink-0 transition-transform motion-safe:group-hover:translate-x-1" />
          </p>
        </NuxtLink>
      </li>
    </ul>
  </section>
</template>

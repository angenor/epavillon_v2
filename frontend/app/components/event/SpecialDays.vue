<script setup lang="ts">
import type { ProgrammeTrack } from '~/types/event/edition'
import type { AttachedImage } from '~/types/media'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES JOURNÉES SPÉCIALES publiées de l'édition, en grandes cartes qui mènent
 * chacune à la page de la journée.
 *
 * Une journée spéciale n'est pas un jour du calendrier : l'IFDD la compose à
 * la main parmi les activités retenues (règle métier n° 7), et `starts_on` est
 * une date annoncée, indicative.
 *
 * La carte porte la photo du fil, voilée. Sans photo, l'aplat de la couleur
 * de la journée (`trackDayStyle` : le texte posé dessus est choisi d'après la
 * luminance de la teinte saisie), à défaut le bleu nuit, le pictogramme du
 * fil en filigrane.
 */

interface Props {
  /** Fils publiés de l'édition, journées spéciales d'abord. */
  tracks: ProgrammeTrack[]
  editionSlug: string
  timezone: TimeZoneName
  /** Nombre d'activités rattachées, par identifiant de fil. */
  sessionCounts?: Record<string, number>
  /** Photo de chaque fil, par adresse (`GET /schedule/tracks`). */
  covers?: Record<string, AttachedImage | null>
}

const props = withDefaults(defineProps<Props>(), { sessionCounts: () => ({}), covers: () => ({}) })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()

type DayState = 'past' | 'today' | 'tomorrow'

const STATE_INTENT: Record<DayState, 'neutral' | 'warning' | 'info'> = {
  past: 'neutral',
  today: 'warning',
  tomorrow: 'info',
}

/** Figé au rendu : serveur et client doivent s'accorder. */
const today = dayKeyInZone(new Date(), props.timezone)
const tomorrow = new Date(Date.parse(`${today}T12:00:00Z`) + 86_400_000).toISOString().slice(0, 10)

const format = (date: string, options: Intl.DateTimeFormatOptions) =>
  new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(new Date(`${date}T12:00:00Z`))

const cards = computed(() =>
  props.tracks.map((track) => {
    const start = track.starts_on
    const end = track.ends_on ?? start
    let state: DayState | null = null
    if (start && end) {
      if (end < today) state = 'past'
      else if (start <= today) state = 'today'
      else if (start === tomorrow) state = 'tomorrow'
    }
    const count = props.sessionCounts[track.id] ?? 0
    return {
      id: track.id,
      to: localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: track.slug } }),
      style: trackDayStyle(track.color_hex),
      icon: trackIcon(track.icon_code),
      cover: props.covers[track.slug] ?? null,
      title: tr(track.title),
      subtitle: track.subtitle ? tr(track.subtitle) : '',
      day: start ? format(start, { day: 'numeric' }) : '',
      weekday: start ? format(start, { weekday: 'long' }) : '',
      month: start ? format(start, { month: 'long' }) : '',
      state,
      count: count ? t('event.public.specialDays.activityCount', { count }, count) : t('event.public.specialDays.open'),
    }
  }),
)
</script>

<template>
  <section v-if="props.tracks.length" aria-labelledby="journees-titre">
    <h2 id="journees-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">
      {{ t('event.public.specialDays.title') }}
    </h2>
    <p class="mt-2 max-w-[64ch] text-text-secondary">
      {{ t('event.public.specialDays.description') }}
    </p>

    <ul class="mt-5 grid list-none gap-4 p-0 sm:grid-cols-2 xl:grid-cols-3">
      <li v-for="card in cards" :key="card.id" :style="card.style">
        <NuxtLink
          :to="card.to"
          class="group relative isolate flex h-full min-h-[300px] cursor-pointer flex-col justify-end overflow-hidden rounded-lg px-4.5 pt-16 pb-4.5 no-underline shadow-md transition-[box-shadow,transform] duration-300 hover:shadow-xl motion-safe:hover:-translate-y-1"
          :class="card.cover
            ? 'bg-surface-inverse text-text-on-inverse hover:text-text-on-inverse'
            : 'bg-(--color-day-solid) text-(--color-day-on-solid) hover:text-(--color-day-on-solid)'"
        >
          <TrackCardPhoto v-if="card.cover" :image="card.cover" sizes="(min-width: 1280px) 26rem, (min-width: 640px) 50vw, 100vw" />
          <UiIcon
            v-else-if="card.icon"
            :name="card.icon"
            size="220px"
            :stroke-width="1.4"
            class="pointer-events-none absolute -top-2.5 -right-6 -z-10 opacity-10"
            aria-hidden="true"
          />
          <UiBadge v-if="card.state" :intent="STATE_INTENT[card.state]" size="sm" class="absolute top-4.5 left-4.5">
            {{ t(`event.public.specialDays.state.${card.state}`) }}
          </UiBadge>

          <p v-if="card.day" class="flex items-baseline gap-2.5 font-condensed uppercase">
            <b class="text-[2.8rem] leading-[0.85] font-bold tabular-nums">{{ card.day }}</b>
            <span class="text-[1.05rem] leading-[1.15] font-semibold tracking-[0.04em]">
              <span class="block">{{ card.weekday }}</span>
              <span class="block">{{ card.month }}</span>
            </span>
          </p>
          <h3 class="heading-condensed mt-2.5 text-[1.45rem] text-inherit group-hover:underline">{{ card.title }}</h3>
          <p v-if="card.subtitle" class="mt-1.5 text-sm leading-snug opacity-85">{{ card.subtitle }}</p>

          <span class="mt-4.5 flex items-center justify-between gap-3 border-t border-current/25 pt-3.5 text-sm">
            <span>{{ card.count }}</span>
            <UiIcon name="arrow-right" size="1.2rem" />
          </span>
        </NuxtLink>
      </li>
    </ul>
  </section>
</template>

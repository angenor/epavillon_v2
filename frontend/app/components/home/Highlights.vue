<script setup lang="ts">
import type { PublicScheduleTrack, PublicTrackPage } from '~/types/programme/track-page'
import type { IsoDateTime } from '~/types/shared'
import type { PublicEditionRow } from '~/types/views'

/**
 * LES TEMPS FORTS DE L'ÉDITION DU PAVILLON : une grande carte photo par journée
 * spéciale ou atelier publié (maquette du 01/11). Règle n° 7 : ces journées
 * sont composées à la main par l'IFDD, ce ne sont pas des jours du calendrier.
 *
 * DEUX LECTURES DE L'API PUBLIQUE, AUCUNE ROUTE NOUVELLE. `GET /schedule/tracks`
 * donne les fils publiés de l'édition (titre, couleur, pictogramme, dates,
 * photo, heures lues sur leur déroulé) ; la page de chaque fil
 * (`GET /events/{édition}/tracks/{fil}`) donne ce que le résumé n'a pas :
 * accroche et nombre d'activités. Une lecture manquée efface la section, ou
 * seulement ces deux mentions de la carte, sans message : elle n'est pas
 * l'objet de l'accueil.
 *
 * Sans photo (l'atelier), l'aplat bleu nuit institutionnel, avec le
 * pictogramme du fil en filigrane. La couleur saisie au back-office ne porte
 * que la pastille, par `trackDayStyle`, qui choisit le texte d'après sa
 * luminance.
 */

interface Props {
  edition: PublicEditionRow
  /** Instant de composition de l'accueil : l'horloge qui fait autorité. */
  now: IsoDateTime
}

interface HighlightCard {
  track: PublicScheduleTrack
  page: PublicTrackPage | null
}

const props = defineProps<Props>()

const api = useApi()
const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale, time, zoneLabel } = useDateTime()
const localePath = useLocalePath()

const { data } = await useAsyncData(
  () => `home-highlights-${props.edition.id}`,
  async (): Promise<HighlightCard[]> => {
    const tracks = await api.sessions.scheduleTracks(props.edition.id)
    const kept = tracks
      .filter((track) => track.kind !== 'thematic_track')
      .sort((a, b) => (a.starts_on ?? a.starts_at ?? '').localeCompare(b.starts_on ?? b.starts_at ?? ''))
    const pages = await Promise.all(
      kept.map((track) => api.sessions.trackPage(props.edition.slug, track.slug).catch(() => null)),
    )
    return kept.map((track, index) => ({ track, page: pages[index] ?? null }))
  },
  { lazy: true, default: () => [] },
)

const cards = computed(() => data.value ?? [])

const timezone = computed(() => props.edition.timezone)
const zone = computed(() => zoneLabel(timezone.value, props.edition.city ?? undefined))

/** Le jour annoncé du fil, à midi UTC : la date civile ne glisse pas d'un fuseau à l'autre. */
function dayOf(track: PublicScheduleTrack): string | null {
  return track.starts_on ? `${track.starts_on}T12:00:00Z` : track.starts_at
}

function dateParts(track: PublicScheduleTrack): { day: string; weekday: string; month: string } | null {
  const value = dayOf(track)
  if (!value) return null
  const at = new Date(value)
  const format = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(intlLocale.value, { ...options, timeZone: timezone.value }).format(at)
  return { day: format({ day: 'numeric' }), weekday: format({ weekday: 'long' }), month: format({ month: 'long' }) }
}

/** Passée, aujourd'hui, demain, ou dans N jours : lu dans le fuseau de l'édition. */
function relation(track: PublicScheduleTrack): { past: boolean; label: string } | null {
  const start = dayOf(track)
  if (!start) return null
  const end = track.ends_on ? `${track.ends_on}T12:00:00Z` : (track.ends_at ?? start)
  if (daysBetweenInZone(props.now, end, timezone.value) < 0) return { past: true, label: t('home.highlights.past') }
  const ahead = daysBetweenInZone(props.now, start, timezone.value)
  if (ahead <= 0) return { past: false, label: t('home.highlights.today') }
  if (ahead === 1) return { past: false, label: t('home.highlights.tomorrow') }
  return { past: false, label: t('home.highlights.inDays', ahead) }
}

/** « 9 activités, 08:00 à 18:00 » ; hors du pavillon, le lieu. */
function footer(card: HighlightCard): string {
  const venue = card.track.venue_name ? tr(card.track.venue_name) : ''
  if (card.track.is_off_site && venue) return venue
  const count = card.page ? card.page.agenda.filter((entry) => entry.kind === 'activity').length : 0
  // Celles du résumé valent celles de la page : lues sur le même déroulé, accueil compris.
  const { starts_at: from, ends_at: to } = card.track
  const hours = from && to ? t('home.highlights.hours', { start: time(from, timezone.value), end: time(to, timezone.value) }) : ''
  const activities = count ? t('home.highlights.activities', count) : ''
  return [activities, hours].filter(Boolean).join(', ') || venue
}

function summary(card: HighlightCard): string {
  const header = card.page?.track
  if (!header) return ''
  return tr(header.tagline) || tr(header.subtitle)
}

const trackTo = (track: PublicScheduleTrack) =>
  localePath({ name: 'track-edition-track', params: { edition: props.edition.slug, track: track.slug } })

const editionName = computed(() => props.edition.edition_label ?? props.edition.acronym ?? tr(props.edition.title))
</script>

<template>
  <section v-if="cards.length" aria-labelledby="temps-forts-titre">
    <div class="mb-8 flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
      <div class="min-w-0">
        <h2 id="temps-forts-titre" class="heading-condensed text-[clamp(2rem,4vw,2.75rem)]">
          {{ t('home.highlights.title', { edition: editionName }) }}
        </h2>
        <p class="mt-2 max-w-[62ch] text-text-secondary">
          {{ t('home.highlights.description', { zone }) }}
        </p>
      </div>
      <UiButton variant="secondary" :to="localePath(`/programmations?edition=${props.edition.slug}`)">
        {{ t('home.highlights.all') }}
      </UiButton>
    </div>

    <ul class="m-0 grid list-none grid-cols-1 gap-6 p-0 lg:grid-cols-3">
      <li v-for="card in cards" :key="card.track.slug" class="flex">
        <article
          class="group relative isolate flex min-h-[17.5rem] w-full flex-col justify-end overflow-hidden rounded-xl bg-surface-inverse px-6 pt-16 pb-6 text-text-on-inverse shadow-md transition duration-300 hover:shadow-xl motion-safe:hover:-translate-y-1 lg:min-h-[22.5rem]"
        >
          <TrackCardPhoto v-if="card.track.cover" :image="card.track.cover" sizes="(min-width: 1024px) 26rem, 100vw" />
          <UiIcon
            v-else
            :name="trackIcon(card.track.icon_code) ?? 'calendar'"
            size="13.75rem"
            class="absolute -top-2.5 -right-6 -z-10 text-text-on-inverse/10"
            aria-hidden="true"
          />

          <span v-if="relation(card.track)" class="absolute top-[1.125rem] left-[1.125rem]">
            <UiStatusBadge v-if="relation(card.track)?.past" state="past" size="sm" :label="relation(card.track)?.label ?? ''" />
            <span
              v-else
              class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[0.72rem] leading-tight font-bold tracking-[0.05em] uppercase"
              :class="card.track.color_hex ? 'border-transparent bg-(--color-day-solid) text-(--color-day-on-solid)' : 'border-border-on-inverse bg-surface-inverse-raised text-text-on-inverse'"
              :style="card.track.color_hex ? trackDayStyle(card.track.color_hex) : undefined"
            >
              <UiIcon v-if="trackIcon(card.track.icon_code)" :name="trackIcon(card.track.icon_code) ?? ''" size="0.9rem" />
              {{ relation(card.track)?.label }}
            </span>
          </span>

          <p v-if="dateParts(card.track)" class="mb-2.5 flex items-baseline gap-2.5 font-condensed uppercase">
            <b class="text-[3.4rem] leading-[0.85] font-bold tabular-nums">{{ dateParts(card.track)?.day }}</b>
            <span class="text-[1.05rem] leading-[1.15] font-semibold tracking-[0.04em]">
              {{ dateParts(card.track)?.weekday }}<br>{{ dateParts(card.track)?.month }}
            </span>
          </p>
          <h3 class="heading-condensed text-[1.85rem] text-text-on-inverse">
            <!-- Lien couvrant : toute la carte mène à la journée. -->
            <NuxtLink :to="trackTo(card.track)" class="text-text-on-inverse no-underline after:absolute after:inset-0 group-hover:underline">
              {{ tr(card.track.title) }}
            </NuxtLink>
          </h3>
          <p v-if="summary(card)" class="mt-2 line-clamp-2 text-[0.95rem] text-text-on-inverse-muted">{{ summary(card) }}</p>
          <p
            class="mt-4.5 flex items-center justify-between gap-3 border-t pt-3.5 text-[0.88rem] text-text-on-inverse-muted"
            :class="card.track.cover ? 'border-glass-border' : 'border-border-on-inverse'"
          >
            <span>{{ footer(card) }}</span>
            <UiIcon name="arrow-right" size="1.2rem" class="shrink-0 text-text-on-inverse" />
          </p>
        </article>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import type { EventEdition, ProgrammeTrack } from '~/types/event/edition'
import type { PublicScheduleRow } from '~/types/views'

/**
 * RENVOI VERS LA PROGRAMMATION, en aplat bleu nuit : le renvoi le plus suivi
 * de la page une fois le programme publié, il se repère en balayant l'écran.
 * L'aplat ne s'inverse pas au thème sombre.
 *
 * La programmation a sa page (`/programmations`, qui porte un sélecteur
 * d'édition) : le lien part avec cette édition sélectionnée. Il annonce ce
 * qu'il y a derrière (activités, organisations, jours publiés), puis une case
 * par jour qui ouvre la programmation à ce jour ; le trait d'une case prend la
 * couleur de la journée spéciale qui s'y tient. Rien n'est inventé : les jours
 * sont ceux des séances publiées.
 */

interface Props {
  edition: EventEdition
  /** La programmation de l'édition, ou la liste des programmations tant qu'elle n'est pas publiée. */
  to: string
  /** Séances publiées de l'édition (`programme.v_public_schedule`). */
  schedule: PublicScheduleRow[]
  /** Fils publiés de l'édition. */
  tracks: ProgrammeTrack[]
  sessionCount: number
  /** Jours qui portent au moins une activité. */
  dayCount: number
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { dayLong } = useDateTime()
const localePath = useLocalePath()

const isPublished = computed(() => props.edition.programme_published_at !== null)

/** Les organisations porteuses : la vue publique ne porte pas les co-organisateurs. */
const organizationCount = computed(() => {
  const ids = new Set<string>()
  for (const session of props.schedule) {
    if (session.organization_id) ids.add(session.organization_id)
  }
  return ids.size
})

const figures = computed(() => [
  { key: 'sessions', value: props.sessionCount },
  { key: 'organizations', value: organizationCount.value },
  { key: 'days', value: props.dayCount },
])

interface DayTile {
  date: string
  weekday: string
  number: string
  to: string
  /** Couleur du trait : la journée spéciale du jour, l'atelier, ou rien. */
  mark: string | null
  label: string
}

const days = computed<DayTile[]>(() => {
  if (!isPublished.value) return []
  const zone = props.edition.timezone
  const specials = new Map(props.tracks.filter((track) => track.kind === 'special_day').map((track) => [track.slug, track]))
  const sides = new Set(props.tracks.filter((track) => track.kind === 'side_programme').map((track) => track.slug))

  const byDay = new Map<string, { special: ProgrammeTrack | null; side: boolean }>()
  for (const session of props.schedule) {
    const key = dayKeyInZone(session.starts_at, zone)
    const entry = byDay.get(key) ?? { special: null, side: false }
    for (const badge of session.tracks) {
      if (!entry.special && badge.kind === 'special_day') entry.special = specials.get(badge.slug) ?? null
      if (sides.has(badge.slug)) entry.side = true
    }
    byDay.set(key, entry)
  }

  // Midi UTC : la clé est déjà un jour civil de l'édition, aucun fuseau ne doit le décaler.
  const format = (date: string, options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(locale.value, { ...options, timeZone: 'UTC' }).format(new Date(`${date}T12:00:00Z`))
  return [...byDay.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([date, entry]) => {
      const special = entry.special
      const mark = special
        ? (special.color_hex ?? 'var(--color-accent-on-inverse)')
        : entry.side
          ? 'var(--color-accent-on-inverse)'
          : null
      const day = dayLong(`${date}T12:00:00Z`, 'UTC')
      return {
        date,
        weekday: format(date, { weekday: 'short' }),
        number: format(date, { day: 'numeric' }),
        to: localePath({ name: 'programme', query: { edition: props.edition.slug, jour: date } }),
        mark,
        label: special ? `${day}, ${tr(special.title)}` : day,
      }
    })
})
</script>

<template>
  <section
    id="programmation"
    class="scroll-mt-24 overflow-hidden rounded-lg bg-surface-inverse p-6 text-text-on-inverse sm:p-7"
    aria-labelledby="programmation-lien-titre"
  >
    <div class="flex flex-wrap items-end justify-between gap-x-7 gap-y-5">
      <div class="min-w-0">
        <h2 id="programmation-lien-titre" class="heading-condensed text-[2.2rem] text-text-on-inverse">
          {{ t('event.public.programmeLink.title') }}
        </h2>

        <!-- Les chiffres décident du clic : un lien nu ne dit pas s'il mène à
             quarante activités ou à une page vide. -->
        <dl v-if="isPublished" class="mt-3.5 flex flex-wrap gap-x-7 gap-y-3">
          <div v-for="figure in figures" :key="figure.key" class="flex flex-col-reverse">
            <dt class="text-[0.72rem] font-semibold tracking-[0.06em] text-text-on-inverse-muted uppercase">
              {{ t(`event.public.programmeLink.figures.${figure.key}`, figure.value) }}
            </dt>
            <dd class="font-condensed text-[2.6rem] leading-none font-bold tabular-nums">{{ figure.value }}</dd>
          </div>
        </dl>
        <p v-else class="mt-4 max-w-(--measure) text-sm text-text-on-inverse-muted">
          {{ t('event.public.programmeLink.unpublished') }}
        </p>
      </div>

      <UiButton
        variant="on-inverse"
        size="lg"
        :to="props.to"
        icon-trailing="arrow-right"
        :label="isPublished ? t('event.public.programmeLink.see') : t('event.public.programmeLink.seePast')"
      />
    </div>

    <nav
      v-if="days.length"
      class="mt-5.5 flex gap-1.5 overflow-x-auto border-t border-border-on-inverse pt-4.5 [scrollbar-width:none]"
      :aria-label="t('event.public.programmeLink.daysLabel')"
    >
      <NuxtLink
        v-for="day in days"
        :key="day.date"
        :to="day.to"
        :aria-label="day.label"
        class="flex min-h-16 w-14 shrink-0 cursor-pointer flex-col items-center justify-center rounded-lg text-text-on-inverse-muted no-underline transition-colors hover:bg-surface-inverse-raised hover:text-text-on-inverse"
      >
        <small class="text-[0.68rem] font-semibold tracking-[0.05em] uppercase" aria-hidden="true">{{ day.weekday }}</small>
        <b class="font-condensed text-[1.6rem] leading-none font-bold text-text-on-inverse tabular-nums" aria-hidden="true">
          {{ day.number }}
        </b>
        <i class="mt-1 block h-1 w-4.5 rounded-full" :style="day.mark ? { backgroundColor: day.mark } : undefined" aria-hidden="true" />
      </NuxtLink>
    </nav>
  </section>
</template>

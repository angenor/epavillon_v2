<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

/**
 * Une ligne du déroulé d'une journée, dans la direction condensée : grande heure
 * grasse à la couleur de la journée, vignette de couverture, titre condensé,
 * logo et sigle de l'organisation devant son nom, points du programme, état.
 * L'activité en direct passe sur fond rouge pâle avec « Rejoindre ». Sans `to`,
 * c'est un moment sans activité (accueil, déjeuner, clôture) : ligne pointillée.
 */

export interface TrackAgendaRowData {
  to: string | null
  start: string
  /** « à 09:15, 15 min », déjà composé ; vide pour une heure seule. */
  until: string
  title: string
  lead: { name: string; acronym: string | null; logo: AttachedImage | null } | null
  /** « avec l'OCDE et le GCF », déjà composé. */
  others: string
  cover: AttachedImage | null
  /** Ce que porte une vignette sans image : le sigle. */
  placeholder: string
  current: SessionDisplayState
  stateLabel: string
  /** Le lecteur de la page de l'activité, quand elle est en direct. */
  liveTo: string | null
  points: string[]
  highlight: boolean
}

interface Props {
  entry: TrackAgendaRowData
  /** Colonne de vignette ; l'atelier, sans images, s'en passe. */
  media?: boolean
}

const props = withDefaults(defineProps<Props>(), { media: true })

const { t } = useI18n()

const isMoment = computed(() => props.entry.to === null)
const live = computed(() => props.entry.current === 'live')
const faded = computed(() => props.entry.current === 'past' || props.entry.current === 'cancelled')
const ongoing = computed(() => props.entry.current === 'ongoing')

const leadParts = computed(() => {
  const lead = props.entry.lead
  if (!lead) return null
  return lead.acronym ? { mark: lead.acronym, rest: lead.name === lead.acronym ? '' : lead.name } : { mark: lead.name, rest: '' }
})

const timeColor = computed(() => (live.value ? 'text-live' : isMoment.value ? 'text-text-secondary' : 'text-(--color-day)'))
</script>

<template>
  <article
    v-if="isMoment"
    class="grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 py-3 text-[0.92rem] text-text-secondary md:grid-cols-[96px_minmax(0,1fr)] md:gap-x-5.5"
    :class="[ongoing ? '-mx-3.5 rounded-md bg-warning-surface px-3.5' : '', faded ? 'opacity-60' : '']"
  >
    <p class="font-condensed text-[1.3rem] leading-tight font-bold tabular-nums">{{ props.entry.start }}</p>
    <div class="min-w-0 self-center">
      <h4 class="font-text text-[0.92rem] font-normal text-text-secondary">
        {{ props.entry.title }}<span v-if="props.entry.until" class="text-text-muted tabular-nums">, {{ props.entry.until }}</span>
      </h4>
      <TrackAgendaPoints :points="props.entry.points" />
    </div>
  </article>

  <article
    v-else
    class="group relative grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 gap-y-2 py-5 md:gap-x-5.5"
    :class="[
      props.media ? 'md:grid-cols-[96px_160px_minmax(0,1fr)_auto]' : 'md:grid-cols-[96px_minmax(0,1fr)_auto]',
      live ? '-mx-3.5 rounded-md bg-live-surface px-3.5' : ongoing ? '-mx-3.5 rounded-md bg-warning-surface px-3.5' : '',
      faded ? 'opacity-60' : '',
      props.entry.highlight && !live && !ongoing
        ? 'before:absolute before:top-5 before:bottom-5 before:-left-3 before:w-1 before:rounded-sm before:bg-(--color-day-solid) md:before:-left-5'
        : '',
    ]"
  >
    <p class="col-start-1 row-start-1 font-condensed">
      <b class="block text-[1.95rem] leading-[0.9] font-bold tracking-[0.01em] tabular-nums md:text-[2.6rem]" :class="timeColor">
        {{ props.entry.start }}
      </b>
      <span v-if="props.entry.until" class="mt-1.5 block font-text text-[0.82rem] text-text-muted tabular-nums">{{ props.entry.until }}</span>
    </p>

    <div
      v-if="props.media"
      class="relative hidden aspect-video overflow-hidden rounded-md bg-surface-sunken md:col-start-2 md:row-start-1 md:block"
    >
      <UiImage
        v-if="props.entry.cover"
        :image="props.entry.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-400 motion-safe:group-hover:scale-105"
        :class="{ grayscale: faded }"
        sizes="160px"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center rounded-md border border-border-subtle bg-(--color-day-surface) font-condensed text-[1.6rem] font-bold tracking-[0.04em] text-(--color-day)"
        aria-hidden="true"
      >
        {{ props.entry.placeholder }}
      </span>
    </div>

    <div class="col-start-2 row-start-1 min-w-0" :class="props.media ? 'md:col-start-3' : 'md:col-start-2'">
      <h4
        class="font-condensed text-[1.25rem] leading-[1.12] font-semibold tracking-[0.005em] text-pretty text-heading md:text-[1.45rem]"
        :class="{ 'line-through': props.entry.current === 'cancelled' }"
      >
        <!-- Lien couvrant : toute la ligne mène à l'activité, sans second lien à tabuler. -->
        <NuxtLink
          :to="props.entry.to ?? undefined"
          class="text-inherit no-underline decoration-2 underline-offset-4 after:absolute after:inset-0 hover:text-accent hover:underline"
        >
          {{ props.entry.title }}
        </NuxtLink>
      </h4>
      <p v-if="leadParts" class="mt-2 flex items-start gap-2.5 text-[0.9rem] leading-snug text-text-secondary">
        <OrganizationLogoMark v-if="props.entry.lead" :name="props.entry.lead.name" :acronym="props.entry.lead.acronym" :logo="props.entry.lead.logo" size="sm" />
        <span class="min-w-0">
          <b class="font-bold tracking-[0.02em] text-text">{{ leadParts.mark }}</b>{{ [leadParts.rest, props.entry.others].filter(Boolean).map((text) => ` ${text}`).join('') }}
        </span>
      </p>
      <TrackAgendaPoints :points="props.entry.points" />
      <TrackHighlightMark v-if="props.entry.highlight" />
    </div>

    <div
      class="col-start-2 row-start-2 flex flex-wrap items-center gap-2.5 md:row-start-1 md:flex-col md:flex-nowrap md:items-end"
      :class="props.media ? 'md:col-start-4' : 'md:col-start-3'"
    >
      <UiStatusBadge :state="props.entry.current" size="sm" :label="props.entry.stateLabel" />
      <UiButton v-if="live && props.entry.liveTo" :to="props.entry.liveTo" variant="live" icon="broadcast" class="relative z-10">{{ t('track.agenda.join') }}</UiButton>
    </div>
  </article>
</template>

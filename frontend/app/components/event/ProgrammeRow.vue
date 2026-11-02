<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { JointPart } from '~/composables/useJointSession'
import type { ActiveIncident } from '~/types/live'

/**
 * Une ligne d'activité, direction condensée : grande heure, vignette, titre,
 * organisation, état, « Rejoindre » en direct. Celle de `/programmations` et,
 * en `compact`, celle du déroulé d'une journée spéciale (`TrackAgenda`).
 *
 * Sans `to`, c'est un moment sans activité (accueil, déjeuner, clôture) :
 * ligne basse, sans lien ni vignette.
 *
 * Séance commune (`parts`) : les couvertures des dossiers se relaient dans la
 * vignette, les logos se chevauchent, les titres d'origine suivent, numérotés.
 */

export interface ProgrammeRowData {
  id: string
  to: string | null
  start: string
  /** « à 12:30, 1 h », déjà composé ; vide pour une heure seule. */
  until: string
  title: { lead: string; rest: string }
  acronym: string | null
  /** Ce que porte une vignette sans image ; à défaut le sigle. */
  placeholder?: string | null
  organization: string | null
  country: string
  /** Les autres organisations, déjà composé : « avec l'OCDE et le GCF ». */
  others?: string
  cover: AttachedImage | null
  /** Logo de l'organisation porteuse, devant son nom ; à défaut, son sigle. */
  logo?: AttachedImage | null
  room?: string | null
  /** Points du programme, puces à la couleur de la journée. */
  points?: string[]
  /** Le direct de l'activité, quand elle est en direct. */
  joinTo?: string | null
  /** « Dans mon programme » : activité gardée par qui s'est inscrit à la journée. */
  mine?: string
  current: SessionDisplayState
  faded: boolean
  stateLabel: string
  /** Séance commune : ses dossiers, dans l'ordre ; vide ou absent pour une séance ordinaire. */
  parts?: JointPart[]
  /** Le message de la régie qui vise l'activité, en une ligne sous le titre. */
  notice?: ActiveIncident | null
}

interface Props {
  entry: ProgrammeRowData
  /** Colonne de vignette ; l'atelier, sans images, s'en passe. */
  media?: boolean
  /** Dans une journée spéciale : l'heure et la tuile au sigle prennent sa couleur. */
  tone?: 'plain' | 'day'
  /** Temps fort : un trait de la couleur de la journée dans la marge, la mention sous le titre. */
  highlight?: boolean
  /** Le déroulé d'une journée, pleine largeur : colonnes plus serrées dès 768 px, nom en gras à défaut de sigle. */
  compact?: boolean
  /** L'en cours et le direct se posent sur un fond (jaune, rouge pâle). */
  stateSurface?: boolean
  /** La photo de l'édition : la vignette d'une activité sans couverture. */
  editionImage?: AttachedImage | null
}

const props = withDefaults(defineProps<Props>(), {
  media: true,
  tone: 'plain',
  highlight: false,
  compact: false,
  stateSurface: false,
  editionImage: null,
})

const { t } = useI18n()
const { namesOf, countLabel, slidesOf } = useJointSession()

const parts = computed(() => props.entry.parts ?? [])
const isJoint = computed(() => parts.value.length > 1)
const slides = computed(() => slidesOf(parts.value, props.entry.cover, props.entry.id))

const isMoment = computed(() => props.entry.to === null)
const title = computed(() => props.entry.title.lead + props.entry.title.rest)

const surface = computed(() => {
  if (!props.stateSurface) return ''
  if (props.entry.current === 'live') return '-mx-3.5 rounded-md bg-live-surface px-3.5'
  if (props.entry.current === 'ongoing') return '-mx-3.5 rounded-md bg-warning-surface px-3.5'
  return ''
})

const timeColor = computed(() => {
  if (props.entry.current === 'live') return 'text-live'
  return props.tone === 'day' ? 'text-(--color-day)' : 'text-heading'
})

/** Le gras : le sigle, ou le nom dans le déroulé ; le reste suit. */
const mark = computed(() => props.entry.acronym ?? (props.compact ? props.entry.organization : null))
const tail = computed(() => {
  const { acronym, organization, country, others } = props.entry
  const rest = mark.value === acronym ? organization : null
  return [mark.value && rest ? ' ' : '', rest ?? '', country ? `, ${country}` : '', others ? ` ${others}` : ''].join('')
})

// Deux géométries : la liste, rétrécie par la colonne du jour, passe en colonnes à 860 px ; le déroulé dès 768 px.
const layout = computed(() =>
  props.compact
    ? {
        moment: 'md:grid-cols-[96px_minmax(0,1fr)] md:gap-x-5.5',
        grid: props.media ? 'md:grid-cols-[96px_160px_minmax(0,1fr)_auto] md:gap-x-5.5' : 'md:grid-cols-[96px_minmax(0,1fr)_auto] md:gap-x-5.5',
        bar: 'md:before:-left-5',
        time: 'col-start-1 row-start-1',
        clock: 'md:text-[2.6rem]',
        thumb: 'rounded-md bg-surface-sunken md:col-start-2 md:row-start-1 md:block',
        placeholder: 'rounded-md border border-border-subtle bg-(--color-day-surface) text-(--color-day)',
        image: 'duration-400',
        sizes: '160px',
        body: ['col-start-2 row-start-1', props.media ? 'md:col-start-3' : 'md:col-start-2'],
        heading: 'md:text-[1.45rem]',
        side: ['row-start-2 md:row-start-1 md:flex-col md:flex-nowrap md:items-end', props.media ? 'md:col-start-4' : 'md:col-start-3'],
      }
    : {
        moment: 'min-[860px]:grid-cols-[112px_minmax(0,1fr)] min-[860px]:gap-x-5.5',
        grid: props.media
          ? 'min-[860px]:grid-cols-[112px_176px_minmax(0,1fr)_auto] min-[860px]:gap-x-5.5'
          : 'min-[860px]:grid-cols-[112px_minmax(0,1fr)_auto] min-[860px]:gap-x-5.5',
        bar: 'min-[860px]:before:-left-5',
        time: 'row-span-2 min-[860px]:row-span-1',
        clock: 'min-[860px]:text-[2.6rem]',
        thumb: 'rounded-lg border border-border-subtle bg-[var(--color-day-surface,var(--color-surface-sunken))] min-[860px]:block',
        placeholder: 'text-[var(--color-day,var(--color-heading))]',
        image: 'duration-300',
        sizes: '176px',
        body: [],
        heading: 'line-clamp-3 min-[860px]:text-[1.45rem]',
        side: [props.media ? 'min-[860px]:col-start-4' : 'min-[860px]:col-start-3', 'min-[860px]:flex-col min-[860px]:flex-nowrap min-[860px]:items-end'],
      },
)
</script>

<template>
  <article
    v-if="isMoment"
    class="grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 py-3 text-[0.92rem] text-text-secondary"
    :class="[layout.moment, surface, { 'opacity-60': props.entry.faded }]"
  >
    <p class="font-condensed text-[1.3rem] leading-tight font-bold tabular-nums">{{ props.entry.start }}</p>
    <div class="min-w-0 self-center">
      <h4 class="font-text text-[0.92rem] font-normal text-text-secondary">
        {{ title }}<span v-if="props.entry.until" class="text-text-muted tabular-nums">, {{ props.entry.until }}</span>
      </h4>
      <TrackAgendaPoints v-if="props.entry.points" :points="props.entry.points" />
    </div>
  </article>

  <!-- Heure | vignette | titre et organisation | état. -->
  <article
    v-else
    class="group relative grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 gap-y-2 py-5"
    :class="[
      layout.grid,
      surface,
      { 'opacity-60': props.entry.faded },
      // Sur un fond d'état, le trait du temps fort n'a plus de marge où se poser.
      props.highlight && !surface
        ? ['before:absolute before:inset-y-5 before:-left-3 before:w-1 before:rounded-full before:bg-(--color-day-solid)', layout.bar]
        : '',
    ]"
  >
    <p class="font-condensed tabular-nums" :class="layout.time">
      <span class="block text-[1.95rem] leading-[0.9] font-bold tracking-[0.01em]" :class="[timeColor, layout.clock]">
        {{ props.entry.start }}
      </span>
      <span v-if="props.entry.until" class="mt-1.5 block font-text text-[0.82rem] text-text-muted">{{ props.entry.until }}</span>
    </p>

    <!-- Sans couverture, la photo de l'édition voilée au sigle ; masquée avec la vignette sur téléphone. -->
    <div v-if="props.media" class="relative hidden aspect-video overflow-hidden" :class="layout.thumb">
      <UiSlideshow
        v-if="isJoint && slides.length"
        :slides="slides"
        :interval="5000"
        :image-class="['transition motion-safe:group-hover:scale-105', layout.image, { grayscale: props.compact && props.entry.faded }]"
        :sizes="layout.sizes"
      />
      <UiImage
        v-else-if="props.entry.cover"
        :image="props.entry.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition motion-safe:group-hover:scale-105"
        :class="[layout.image, { grayscale: props.compact && props.entry.faded }]"
        :sizes="layout.sizes"
      />
      <ActivityCoverFallback
        v-else
        :image="props.editionImage"
        :label="props.entry.placeholder ?? props.entry.acronym"
        :sizes="layout.sizes"
        :faded="props.compact && props.entry.faded"
        :image-class="['transition motion-safe:group-hover:scale-105', layout.image]"
      >
        <span
          class="absolute inset-0 flex items-center justify-center font-condensed text-[1.6rem] font-bold tracking-[0.04em]"
          :class="layout.placeholder"
          aria-hidden="true"
        >
          {{ props.entry.placeholder ?? props.entry.acronym }}
        </span>
      </ActivityCoverFallback>
    </div>

    <div class="min-w-0" :class="layout.body">
      <h4
        class="font-condensed text-[1.25rem] leading-[1.12] font-semibold tracking-[0.005em] text-pretty text-heading"
        :class="[layout.heading, { 'line-through': props.entry.current === 'cancelled' }]"
      >
        <!-- Lien couvrant : toute la ligne mène à l'activité, sans second lien à tabuler. -->
        <NuxtLink
          :to="props.entry.to ?? undefined"
          class="text-inherit no-underline decoration-2 underline-offset-4 after:absolute after:inset-0 hover:text-accent hover:underline"
        >
          {{ title }}
        </NuxtLink>
      </h4>
      <ActivityIncidentNotice v-if="props.entry.notice" :incident="props.entry.notice" variant="flag" class="mt-1.5" />
      <p
        v-if="props.entry.mine"
        class="mt-1.5 inline-flex items-center gap-1 text-xs font-bold tracking-[0.04em] text-success uppercase"
      >
        <UiIcon name="check" size="0.875rem" :stroke-width="2.6" />{{ props.entry.mine }}
      </p>
      <template v-if="isJoint">
        <p class="mt-2 flex items-center gap-2.5 text-[0.9rem] text-text-secondary" :class="{ 'leading-snug': props.compact }">
          <span class="flex shrink-0">
            <OrganizationLogoMark
              v-for="(part, index) in parts"
              :key="part.id"
              :name="part.name"
              :acronym="part.acronym"
              :logo="part.logo"
              :size="props.compact ? 'sm' : 'md'"
              :class="{ '-ml-3 ring-2 ring-surface': index > 0 }"
            />
          </span>
          <span class="min-w-0">
            <b class="font-bold tracking-[0.02em] text-text">{{ namesOf(parts) }}</b> {{ t('programme.joint.with') }}
          </span>
        </p>
        <p class="mt-2 inline-flex items-center gap-1.5 text-[0.72rem] font-bold tracking-[0.06em] text-[var(--color-day,var(--color-accent))] uppercase">
          <UiIcon name="users" size="1.05em" :stroke-width="2" />{{ countLabel(parts.length) }}
        </p>
        <ol class="mt-1 text-[0.85rem] text-text-secondary">
          <li v-for="part in parts" :key="part.id" class="flex gap-2 py-0.5">
            <b
              class="mt-0.5 grid size-4.5 flex-none place-items-center rounded-full bg-[var(--color-day-solid,var(--color-text))] text-[0.7rem] text-[var(--color-day-on-solid,var(--color-surface))]"
              aria-hidden="true"
            >
              {{ part.number }}
            </b>
            <span class="min-w-0">{{ t('programme.joint.part', { title: part.title, organization: part.label }) }}</span>
          </li>
        </ol>
      </template>
      <p
        v-else-if="props.entry.organization || props.entry.acronym"
        class="mt-2 flex items-start gap-2.5 text-[0.9rem] text-text-secondary"
        :class="{ 'leading-snug': props.compact }"
      >
        <OrganizationLogoMark
          :name="props.entry.organization ?? props.entry.acronym ?? ''"
          :acronym="props.entry.acronym"
          :logo="props.entry.logo ?? null"
          :size="props.compact ? 'sm' : 'md'"
        />
        <span class="min-w-0" :class="{ 'self-center': !props.compact }">
          <b v-if="mark" class="font-bold tracking-[0.02em] text-text">{{ mark }}</b>{{ tail }}
        </span>
      </p>
      <p v-if="props.entry.room" class="mt-2 flex items-center gap-1.5 text-[0.82rem] text-text-muted">
        <UiIcon name="map-pin" size="1em" />{{ props.entry.room }}
      </p>
      <TrackAgendaPoints v-if="props.entry.points" :points="props.entry.points" />
      <TrackHighlightMark v-if="props.highlight" />
    </div>

    <div class="col-start-2 flex flex-wrap items-center gap-2.5" :class="layout.side">
      <UiStatusBadge :state="props.entry.current" size="sm" :label="props.entry.stateLabel" />
      <!-- Au-dessus du lien couvrant : le direct se rejoint sans passer par la fiche. -->
      <UiButton v-if="props.entry.joinTo" :to="props.entry.joinTo" variant="live" icon="broadcast" class="relative z-[1]">
        {{ t('programme.list.join') }}
      </UiButton>
    </div>
  </article>
</template>

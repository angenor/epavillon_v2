<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

/**
 * Une ligne d'activité : grande heure, vignette, titre, organisation, état.
 * Celle de `/programmations` (`density="list"`, direction condensée du 01/11)
 * et celle du déroulé d'une journée spéciale (`density="day"`, plus serrée).
 *
 * Sans `to`, c'est un moment sans activité (accueil, déjeuner) : ni lien, ni
 * vignette, en retrait.
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
  cover: AttachedImage | null
  /** Logo de l'organisation porteuse, devant son nom ; à défaut, son sigle. */
  logo?: AttachedImage | null
  room?: string | null
  /** Le direct de l'activité, quand elle est en direct (`density="list"`). */
  joinTo?: string | null
  current: SessionDisplayState
  faded: boolean
  stateLabel: string
}

interface Props {
  entry: ProgrammeRowData
  density?: 'list' | 'day'
  /** Colonne de vignette ; l'atelier, sans images, s'en passe. */
  media?: boolean
  /** Dans le cadre d'une journée de `/programmations` : l'heure et la tuile au sigle prennent sa couleur. */
  tone?: 'plain' | 'day'
  highlight?: boolean
  /** Le déroulé d'une journée tait « à venir » : seuls l'en cours et le passé se disent. */
  quietUpcoming?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  density: 'list',
  media: true,
  tone: 'plain',
  highlight: false,
  quietUpcoming: false,
})

const { t } = useI18n()

const isMoment = computed(() => props.entry.to === null)
const showBadge = computed(() => !(props.quietUpcoming && props.entry.current === 'upcoming'))
const title = computed(() => props.entry.title.lead + props.entry.title.rest)

const background = computed(() => {
  const state = props.entry.current
  if (state === 'live') return 'bg-live-surface'
  if (state === 'ongoing') return props.highlight ? 'bg-live-surface' : 'bg-warning-surface'
  return props.highlight ? 'bg-(--color-day-surface)' : ''
})

const grid = computed(() =>
  props.media
    ? 'grid-cols-[4.75rem_minmax(0,1fr)] md:grid-cols-[88px_128px_minmax(0,1fr)_auto] md:gap-x-6'
    : 'grid-cols-[4.75rem_minmax(0,1fr)] md:grid-cols-[88px_minmax(0,1fr)_auto] md:gap-x-6',
)

const timeColor = computed(() => {
  if (props.entry.faded) return 'text-text-subtle'
  if (props.entry.current === 'live') return 'text-live'
  return isMoment.value ? 'text-text-muted' : 'text-text'
})

const listTimeColor = computed(() => {
  if (props.entry.current === 'live') return 'text-live'
  return props.tone === 'day' ? 'text-(--color-day)' : 'text-heading'
})
</script>

<template>
  <!-- Liste de `/programmations` : heure | vignette | titre et organisation | état. -->
  <article
    v-if="props.density === 'list'"
    class="group relative grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 gap-y-2 py-5 min-[860px]:grid-cols-[112px_176px_minmax(0,1fr)_auto] min-[860px]:gap-x-5.5"
    :class="[
      { 'opacity-60': props.entry.faded },
      // Temps fort : un trait de la couleur de la journée, dans la marge du cadre.
      props.highlight
        ? 'before:absolute before:inset-y-5 before:-left-2.5 before:w-1 before:rounded-full before:bg-(--color-day-solid) min-[860px]:before:-left-5'
        : '',
    ]"
  >
    <p class="row-span-2 font-condensed tabular-nums min-[860px]:row-span-1">
      <span class="block text-[1.95rem] leading-[0.9] font-bold tracking-[0.01em] min-[860px]:text-[2.6rem]" :class="listTimeColor">
        {{ props.entry.start }}
      </span>
      <span v-if="props.entry.until" class="mt-1.5 block font-text text-[0.82rem] text-text-muted">{{ props.entry.until }}</span>
    </p>

    <!-- Sans couverture, une tuile au sigle ; masquée avec la vignette sur téléphone. -->
    <div
      class="relative hidden aspect-video overflow-hidden rounded-lg border border-border-subtle bg-[var(--color-day-surface,var(--color-surface-sunken))] min-[860px]:block"
    >
      <UiImage
        v-if="props.entry.cover"
        :image="props.entry.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-300 motion-safe:group-hover:scale-105"
        sizes="176px"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center font-condensed text-[1.6rem] font-bold tracking-[0.04em] text-[var(--color-day,var(--color-heading))]"
        aria-hidden="true"
      >
        {{ props.entry.placeholder ?? props.entry.acronym }}
      </span>
    </div>

    <div class="min-w-0">
      <h4
        class="line-clamp-3 font-condensed text-[1.25rem] leading-[1.12] font-semibold tracking-[0.005em] text-pretty text-heading min-[860px]:text-[1.45rem]"
        :class="{ 'line-through': props.entry.current === 'cancelled' }"
      >
        <!-- Lien couvrant : toute la ligne mène à l'activité, sans second lien à tabuler. -->
        <NuxtLink
          v-if="props.entry.to"
          :to="props.entry.to"
          class="text-inherit no-underline decoration-2 underline-offset-4 after:absolute after:inset-0 hover:text-accent hover:underline"
        >
          {{ title }}
        </NuxtLink>
        <template v-else>{{ title }}</template>
      </h4>
      <p v-if="props.entry.organization || props.entry.acronym" class="mt-2 flex items-start gap-2.5 text-[0.9rem] text-text-secondary">
        <OrganizationLogoMark
          :name="props.entry.organization ?? props.entry.acronym ?? ''"
          :acronym="props.entry.acronym"
          :logo="props.entry.logo ?? null"
        />
        <span class="min-w-0 self-center">
          <b v-if="props.entry.acronym" class="font-bold tracking-[0.02em] text-text">{{ props.entry.acronym }}</b>
          <template v-if="props.entry.acronym && props.entry.organization">{{ ' ' }}</template>{{ props.entry.organization }}<template v-if="props.entry.country">, {{ props.entry.country }}</template>
        </span>
      </p>
      <p v-if="props.entry.room" class="mt-2 flex items-center gap-1.5 text-[0.82rem] text-text-muted">
        <UiIcon name="map-pin" size="1em" />{{ props.entry.room }}
      </p>
    </div>

    <div class="col-start-2 flex flex-wrap items-center gap-2.5 min-[860px]:col-start-4 min-[860px]:flex-col min-[860px]:flex-nowrap min-[860px]:items-end">
      <UiStatusBadge :state="props.entry.current" size="sm" :label="props.entry.stateLabel" />
      <!-- Au-dessus du lien couvrant : le direct se rejoint sans passer par la fiche. -->
      <UiButton v-if="props.entry.joinTo" :to="props.entry.joinTo" variant="live" icon="broadcast" class="relative z-[1]">
        {{ t('programme.list.join') }}
      </UiButton>
    </div>
  </article>

  <article
    v-else
    class="group relative -ml-3 grid gap-x-4 gap-y-1.5 rounded-md px-3 md:items-center md:gap-y-0"
    :class="[grid, background, isMoment ? 'py-3' : 'py-4']"
  >
    <p class="col-start-1 row-start-1 self-start md:pt-0.5">
      <span
        class="block leading-none font-light tabular-nums"
        :class="[timeColor, isMoment ? 'text-lg md:text-xl' : 'text-xl md:text-[34px]']"
      >
        {{ props.entry.start }}
      </span>
      <span class="mt-1.5 block text-[13px] text-text-muted tabular-nums">
        {{ props.entry.until || ' ' }}
      </span>
    </p>

    <div
      v-if="props.media && !isMoment"
      class="relative hidden aspect-video self-start overflow-hidden rounded-lg bg-surface-inverse md:col-start-2 md:row-start-1 md:block md:self-center"
    >
      <UiImage
        v-if="props.entry.cover"
        :image="props.entry.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
        :class="{ grayscale: props.entry.faded }"
        sizes="128px"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center text-sm font-bold text-text-on-inverse/20"
        aria-hidden="true"
      >
        {{ props.entry.placeholder ?? props.entry.acronym }}
      </span>
    </div>
    <span v-else-if="props.media" class="hidden md:block" aria-hidden="true" />

    <div class="col-start-2 min-w-0 md:row-start-1" :class="props.media ? 'md:col-start-3' : 'md:col-start-2'">
      <slot name="kicker" />
      <h4
        class="font-sans leading-[1.25] text-pretty"
        :class="[
          isMoment ? 'text-base text-text-muted' : 'text-lg',
          isMoment ? '' : props.entry.faded ? 'text-text-muted' : 'text-text',
          { 'line-through': props.entry.current === 'cancelled' },
        ]"
      >
        <!-- Lien couvrant : toute la ligne mène à l'activité, sans second lien à tabuler. -->
        <NuxtLink
          v-if="props.entry.to"
          :to="props.entry.to"
          class="font-normal text-inherit no-underline after:absolute after:inset-0 hover:underline"
        >
          <b class="font-bold">{{ props.entry.title.lead }}</b>{{ props.entry.title.rest }}
        </NuxtLink>
        <template v-else>{{ props.entry.title.lead }}{{ props.entry.title.rest }}</template>
      </h4>
      <p v-if="props.entry.organization || props.entry.acronym" class="mt-1.5 text-sm text-text-muted">
        <b v-if="props.entry.acronym" class="font-bold text-text">{{ props.entry.acronym }}</b>
        <template v-if="props.entry.acronym && props.entry.organization"> · </template>{{ props.entry.organization }}<template v-if="props.entry.country"> · {{ props.entry.country }}</template>
      </p>
      <slot />
    </div>

    <div
      v-if="showBadge || !isMoment"
      class="col-start-2 flex items-center gap-3 justify-self-start md:row-start-1 md:justify-self-end"
      :class="[props.media ? 'md:col-start-4' : 'md:col-start-3', { 'max-md:hidden': !showBadge }]"
    >
      <UiStatusBadge v-if="showBadge" :state="props.entry.current" size="sm" :label="props.entry.stateLabel" />
      <span v-if="!isMoment" class="hidden text-accent md:block" aria-hidden="true">
        <UiIcon name="arrow-right" size="20px" class="transition-transform duration-200 motion-safe:group-hover:translate-x-1" />
      </span>
    </div>
  </article>
</template>

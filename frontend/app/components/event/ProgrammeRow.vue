<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

/**
 * Une ligne de la liste éditoriale : grande heure, vignette, titre au gras de
 * tête, organisation, état. Celle de `/programmations` (`density="list"`) et
 * celle du déroulé d'une journée spéciale (`density="day"`, plus serrée).
 *
 * Sans `to`, c'est un moment sans activité (accueil, déjeuner) : ni lien, ni
 * vignette, en retrait.
 */

export interface ProgrammeRowData {
  id: string
  to: string | null
  start: string
  /** « à 12:30 · 1 h », déjà composé ; vide pour une heure seule. */
  until: string
  title: { lead: string; rest: string }
  acronym: string | null
  /** Ce que porte une vignette sans image ; à défaut le sigle. */
  placeholder?: string | null
  organization: string | null
  country: string
  cover: AttachedImage | null
  current: SessionDisplayState
  faded: boolean
  stateLabel: string
}

interface Props {
  entry: ProgrammeRowData
  density?: 'list' | 'day'
  /** Colonne de vignette ; l'atelier, sans images, s'en passe. */
  media?: boolean
  striped?: boolean
  /** Dans le cadre d'une journée de `/programmations` : l'alternance prend la teinte de la journée, à bords droits. */
  tone?: 'plain' | 'day'
  highlight?: boolean
  /** Le déroulé d'une journée tait « à venir » : seuls l'en cours et le passé se disent. */
  quietUpcoming?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  density: 'list',
  media: true,
  striped: false,
  tone: 'plain',
  highlight: false,
  quietUpcoming: false,
})

const isMoment = computed(() => props.entry.to === null)
const day = computed(() => props.density === 'day')
const showBadge = computed(() => !(props.quietUpcoming && props.entry.current === 'upcoming'))

const background = computed(() => {
  const state = props.entry.current
  if (state === 'live') return 'bg-live-surface'
  if (day.value && state === 'ongoing') return props.highlight ? 'bg-live-surface' : 'bg-warning-surface'
  if (props.highlight) return 'bg-(--color-day-surface)'
  if (!props.striped) return ''
  return props.tone === 'day' ? 'bg-(--color-day-surface-strong)' : 'bg-text/3'
})

const grid = computed(() => {
  if (!day.value) return 'grid-cols-[7rem_minmax(0,1fr)] lg:grid-cols-[112px_208px_minmax(0,1fr)_172px_24px] lg:gap-x-7 lg:py-4.5'
  return props.media
    ? 'grid-cols-[4.75rem_minmax(0,1fr)] md:grid-cols-[88px_128px_minmax(0,1fr)_auto] md:gap-x-6'
    : 'grid-cols-[4.75rem_minmax(0,1fr)] md:grid-cols-[88px_minmax(0,1fr)_auto] md:gap-x-6'
})

const timeColor = computed(() => {
  if (props.entry.faded) return 'text-text-subtle'
  if (props.entry.current === 'live') return 'text-live'
  return isMoment.value ? 'text-text-muted' : 'text-text'
})
</script>

<template>
  <article
    class="group relative grid gap-x-4 gap-y-1.5 px-3 lg:items-center lg:gap-y-0"
    :class="[grid, background, props.tone === 'day' ? '' : '-ml-3 rounded-md', day ? (isMoment ? 'py-3' : 'py-4') : 'py-4', day ? 'md:items-center md:gap-y-0' : '']"
  >
    <p
      class="row-start-1 self-start"
      :class="day ? 'col-start-1 md:pt-0.5' : 'col-start-2 flex flex-wrap items-baseline gap-x-2 lg:col-start-1 lg:block lg:pt-1'"
    >
      <span
        class="block leading-none font-light tabular-nums"
        :class="[timeColor, day ? (isMoment ? 'text-lg md:text-xl' : 'text-xl md:text-[34px]') : 'text-xl lg:text-[38px]']"
      >
        {{ props.entry.start }}
      </span>
      <span
        class="block text-[13px] text-text-muted tabular-nums"
        :class="day ? 'mt-1.5' : 'lg:mt-1.5'"
      >
        {{ props.entry.until || ' ' }}
      </span>
    </p>

    <div
      v-if="props.media && !isMoment"
      class="relative aspect-video self-start overflow-hidden rounded-lg bg-surface-inverse"
      :class="
        day
          ? 'hidden md:col-start-2 md:row-start-1 md:block md:self-center'
          : 'col-start-1 row-span-3 row-start-1 lg:col-start-2 lg:row-span-1 lg:w-52 lg:self-center'
      "
    >
      <UiImage
        v-if="props.entry.cover"
        :image="props.entry.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 transition duration-200 motion-safe:group-hover:scale-[1.03]"
        :class="{ grayscale: props.entry.faded }"
        :sizes="day ? '128px' : '(min-width: 1024px) 208px, 7rem'"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center text-sm font-bold text-text-on-inverse/20"
        :class="day ? '' : 'lg:text-[22px]'"
        aria-hidden="true"
      >
        {{ props.entry.placeholder ?? props.entry.acronym }}
      </span>
    </div>
    <span v-else-if="props.media && day" class="hidden md:block" aria-hidden="true" />

    <div class="col-start-2 min-w-0" :class="day ? (props.media ? 'md:col-start-3' : 'md:col-start-2') + ' md:row-start-1' : 'lg:col-start-3 lg:row-start-1'">
      <slot name="kicker" />
      <h4
        class="font-sans leading-[1.25] text-pretty"
        :class="[
          day ? (isMoment ? 'text-base text-text-muted' : 'text-lg') : 'line-clamp-3 text-base sm:text-lg lg:text-[21px]',
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
      v-if="showBadge || (day && !isMoment)"
      class="col-start-2 flex items-center gap-3 justify-self-start"
      :class="[
        day ? (props.media ? 'md:col-start-4' : 'md:col-start-3') + ' md:row-start-1 md:justify-self-end' : 'lg:col-start-4 lg:row-start-1',
        { 'max-md:hidden': !showBadge },
      ]"
    >
      <UiStatusBadge v-if="showBadge" :state="props.entry.current" size="sm" :label="props.entry.stateLabel" />
      <span v-if="day && !isMoment" class="hidden text-accent md:block" aria-hidden="true">
        <UiIcon name="arrow-right" size="20px" class="transition-transform duration-200 motion-safe:group-hover:translate-x-1" />
      </span>
    </div>

    <span v-if="!day" class="hidden text-accent lg:col-start-5 lg:row-start-1 lg:block" aria-hidden="true">
      <UiIcon
        name="arrow-right"
        size="22px"
        class="transition-transform duration-200 motion-safe:group-hover:translate-x-1"
      />
    </span>
  </article>
</template>

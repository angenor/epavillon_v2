<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'
import type { AttachedImage } from '~/types/media'

/**
 * Les autres activités du même jour, en lignes comme la liste de programmation :
 * on passe de l'une à l'autre sans revenir au programme. L'étiquette sur filet
 * porte la journée spéciale quand l'activité en fait partie, le jour sinon.
 */

interface Props {
  sessions: PublicScheduleRow[]
  currentId: string
  editionSlug: string
  timezone: TimeZoneName
  dayLabel: string
  /** La journée spéciale, sinon la liste du jour entier. */
  dayTo?: string
  specialDay?: { title: string; to: string } | null
  /** La photo de l'édition : la vignette des activités sans couverture. */
  editionImage?: AttachedImage | null
}

const props = withDefaults(defineProps<Props>(), { editionImage: null })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, link, duration } = useProgrammeSession()
const { partsOf, slidesOf, namesOf } = useJointSession()

const rows = computed(() =>
  props.sessions.map((session) => {
    const current = state(session)
    const here = session.id === props.currentId
    const parts = partsOf(session)
    return {
      id: session.id,
      parts,
      slides: parts.length ? slidesOf(parts, session.cover, session.id) : [],
      jointNames: namesOf(parts),
      to: link(props.editionSlug, session),
      start: time(session.starts_at, props.timezone),
      until: t('activity.sameDay.until', { end: time(session.ends_at, props.timezone), duration: duration(session) }),
      title: tr(session.title),
      cover: session.cover,
      organization: session.organization_name,
      acronym: session.organization_acronym,
      logo: session.organization_logo ?? null,
      state: current,
      stateLabel: current === 'live' ? t('activity.state.live') : t(`session-card.state.${current}`),
      here,
      live: current === 'live',
      cancelled: current === 'cancelled',
      faded: !here && (current === 'past' || current === 'cancelled'),
    }
  }),
)

const span = computed(() => {
  const first = props.sessions[0]
  const last = props.sessions.at(-1)
  if (!first || !last) return ''
  return t('activity.ticket.hours', { start: time(first.starts_at, props.timezone), end: time(last.ends_at, props.timezone) })
})

const ruleStyle = { '--rule-solid': 'var(--color-day-solid)', '--rule-on': 'var(--color-day-on-solid)' }
</script>

<template>
  <section v-if="rows.length > 1" aria-labelledby="meme-jour-titre">
    <h2 id="meme-jour-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
      {{ t('activity.sameDay.title') }}
    </h2>

    <div class="rule-line mt-4.5 mb-1" :style="ruleStyle">
      <NuxtLink
        v-if="props.specialDay"
        :to="props.specialDay.to"
        class="rule-label min-h-11 no-underline hover:underline"
      >
        {{ props.specialDay.title }}
      </NuxtLink>
      <span v-else class="rule-label">{{ props.dayLabel }}</span>
      <span class="pb-1.5 text-sm text-text-secondary">
        <template v-if="props.specialDay"><span class="inline-block first-letter:uppercase">{{ props.dayLabel }}</span>, </template>{{ span }}
      </span>
    </div>

    <ol>
      <li v-for="row in rows" :key="row.id">
        <article
          class="group relative grid grid-cols-[76px_minmax(0,1fr)] items-start gap-x-3.5 gap-y-1.5 border-b border-border-subtle py-5 md:grid-cols-[96px_160px_minmax(0,1fr)_auto] md:gap-x-5.5"
          :class="[row.here ? '-mx-3.5 rounded-md border-transparent bg-(--color-day-surface) px-3.5' : '', { 'opacity-60': row.faded }]"
        >
          <p class="font-condensed">
            <b
              class="block text-[1.95rem] leading-[0.9] font-bold tracking-[0.01em] tabular-nums md:text-[2.4rem]"
              :class="row.live ? 'text-live' : 'text-(--color-day)'"
            >
              {{ row.start }}
            </b>
            <span class="mt-1.5 block font-text text-[0.82rem] text-text-muted">{{ row.until }}</span>
          </p>

          <span class="relative hidden aspect-video overflow-hidden rounded-md bg-(--color-day-surface) md:block" aria-hidden="true">
            <UiSlideshow
              v-if="row.slides.length"
              :slides="row.slides"
              :interval="5000"
              :image-class="['transition-transform duration-500 group-hover:scale-105 motion-reduce:transition-none motion-reduce:group-hover:scale-100', { grayscale: row.faded }]"
              sizes="160px"
            />
            <UiImage
              v-else-if="row.cover"
              :image="row.cover"
              ratio="auto"
              frame-class="size-full"
              class="absolute inset-0 transition-transform duration-500 group-hover:scale-105 motion-reduce:transition-none motion-reduce:group-hover:scale-100"
              :class="{ grayscale: row.faded }"
              sizes="160px"
            />
            <ActivityCoverFallback
              v-else
              :image="props.editionImage"
              :label="(row.acronym ?? initialsOf(row.organization ?? row.title)).slice(0, 6)"
              text-class="text-2xl"
              :faded="row.faded"
              :image-class="['transition-transform duration-500 group-hover:scale-105 motion-reduce:transition-none motion-reduce:group-hover:scale-100']"
              sizes="160px"
            >
              <span class="absolute inset-0 flex items-center justify-center font-condensed text-2xl font-bold tracking-[0.04em] text-(--color-day)">
                {{ (row.acronym ?? initialsOf(row.organization ?? row.title)).slice(0, 6) }}
              </span>
            </ActivityCoverFallback>
          </span>

          <div class="min-w-0">
            <h3 class="font-condensed text-[1.25rem] leading-[1.12] font-semibold tracking-[0.005em] text-heading md:text-[1.45rem]">
              <span v-if="row.here" :class="{ 'line-through': row.cancelled }">{{ row.title }}</span>
              <NuxtLink
                v-else
                :to="row.to"
                class="no-underline after:absolute after:inset-0 hover:text-accent group-hover:underline group-hover:decoration-2 group-hover:underline-offset-4"
                :class="{ 'line-through': row.cancelled }"
              >
                {{ row.title }}
              </NuxtLink>
            </h3>
            <p v-if="row.parts.length" class="mt-2 flex items-center gap-2.5 text-sm text-text-secondary">
              <span class="flex shrink-0">
                <OrganizationLogoMark
                  v-for="(part, index) in row.parts"
                  :key="part.id"
                  size="sm"
                  :name="part.name"
                  :acronym="part.acronym"
                  :logo="part.logo"
                  :class="{ '-ml-3 ring-2 ring-surface': index > 0 }"
                />
              </span>
              <span class="flex min-w-0 flex-wrap gap-x-1.5">
                <b class="font-bold text-text">{{ row.jointNames }}</b>
                <span>{{ t('programme.joint.with') }}</span>
                <span v-if="row.here">{{ t('activity.sameDay.here') }}</span>
              </span>
            </p>
            <p v-else-if="row.organization || row.here" class="mt-2 flex items-center gap-2.5 text-sm text-text-secondary">
              <OrganizationLogoMark v-if="row.organization" size="sm" :name="row.organization" :acronym="row.acronym" :logo="row.logo" />
              <span class="flex min-w-0 flex-wrap gap-x-1.5">
                <b v-if="row.organization" class="font-bold text-text">{{ row.acronym ?? row.organization }}</b>
                <span v-if="row.here">{{ t('activity.sameDay.here') }}</span>
              </span>
            </p>
          </div>

          <div v-if="!row.here" class="col-start-2 flex items-center md:col-start-auto md:justify-end">
            <UiStatusBadge :state="row.state" :label="row.stateLabel" size="sm" />
          </div>
        </article>
      </li>
    </ol>

    <UiButton v-if="props.dayTo" :to="props.dayTo" variant="secondary" icon-trailing="arrow-right" class="mt-4">
      {{ t('activity.sameDay.all') }}
    </UiButton>
  </section>
</template>

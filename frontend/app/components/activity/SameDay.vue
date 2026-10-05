<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/** Les autres activités du même jour : on passe de l'une à l'autre sans revenir au programme. */

interface Props {
  sessions: PublicScheduleRow[]
  currentId: string
  editionSlug: string
  timezone: TimeZoneName
  dayLabel: string
  /** La liste de la journée entière, quand la page sait la désigner. */
  dayTo?: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, link } = useProgrammeSession()

const rows = computed(() =>
  props.sessions.map((session) => {
    const current = state(session)
    const here = session.id === props.currentId
    return {
      id: session.id,
      to: link(props.editionSlug, session),
      start: time(session.starts_at, props.timezone),
      title: tr(session.title),
      cover: session.cover,
      here,
      live: current === 'live',
      cancelled: current === 'cancelled',
      faded: !here && (current === 'past' || current === 'cancelled'),
    }
  }),
)
</script>

<template>
  <section v-if="rows.length > 1" aria-labelledby="meme-jour-titre" class="font-sans">
    <div class="flex flex-wrap items-baseline justify-between gap-x-4 border-b border-text pb-2.5">
      <h2 id="meme-jour-titre" class="font-sans text-[22px] leading-tight font-light text-text">
        <b class="font-bold">{{ t('activity.sameDay.title') }}</b>, {{ props.dayLabel }}
      </h2>
      <NuxtLink
        v-if="props.dayTo"
        :to="props.dayTo"
        class="inline-flex min-h-11 items-center gap-1 text-sm font-bold text-accent hover:underline"
      >
        {{ t('activity.sameDay.all') }}
        <UiIcon name="arrow-right" size="1rem" />
      </NuxtLink>
    </div>

    <ol>
      <li v-for="(row, index) in rows" :key="row.id">
        <article
          class="group relative -ml-2.5 grid grid-cols-[52px_88px_minmax(0,1fr)] items-center gap-x-3 rounded-md px-2.5 py-3 sm:grid-cols-[64px_104px_minmax(0,1fr)] sm:gap-x-3.5"
          :class="row.here ? 'bg-accent/8' : index % 2 ? 'bg-text/3 hover:bg-surface-hover' : 'hover:bg-surface-hover'"
        >
          <span
            class="text-lg font-light tabular-nums sm:text-[22px]"
            :class="row.faded ? 'text-text-subtle' : row.live ? 'text-live' : 'text-text'"
          >
            {{ row.start }}
          </span>

          <span class="relative block aspect-video overflow-hidden rounded-md bg-surface-inverse" aria-hidden="true">
            <UiImage
              v-if="row.cover"
              :image="row.cover"
              ratio="auto"
              frame-class="size-full"
              class="absolute inset-0"
              :class="{ grayscale: row.faded }"
              sizes="104px"
            />
          </span>

          <span class="min-w-0 text-sm leading-snug">
            <NuxtLink
              :to="row.to"
              class="font-bold no-underline after:absolute after:inset-0 group-hover:underline"
              :class="[row.faded ? 'text-text-muted' : 'text-text', { 'line-through': row.cancelled }]"
              :aria-current="row.here ? 'page' : undefined"
            >
              <span class="line-clamp-3">{{ row.title }}</span>
            </NuxtLink>
            <span
              v-if="row.here || row.live"
              class="mt-0.5 block text-[11px] font-bold uppercase"
              :class="row.here ? 'text-accent' : 'text-live'"
              :style="{ letterSpacing: 'var(--tracking-caps)' }"
            >
              {{ row.here ? t('activity.sameDay.here') : t('activity.state.live') }}
            </span>
          </span>
        </article>
      </li>
    </ol>
  </section>
</template>

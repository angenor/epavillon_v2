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
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { state, themeColor, link } = useProgrammeSession()

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
      color: themeColor(session),
      here,
      faded: !here && (current === 'past' || current === 'cancelled'),
      tag: here ? t('activity.sameDay.here') : current === 'live' ? t('programme.week.live') : current === 'past' ? t('session-card.state.past') : '',
    }
  }),
)
</script>

<template>
  <section v-if="rows.length > 1" aria-labelledby="meme-jour-titre">
    <div class="flex items-baseline justify-between gap-3 border-b-4 border-poster-ink pb-2">
      <h2 id="meme-jour-titre" class="font-poster text-[1.625rem] font-black uppercase font-stretch-[68%]">
        {{ t('activity.sameDay.title') }}
      </h2>
      <span class="font-poster-mono text-xs text-poster-ink-muted">{{ props.dayLabel }}</span>
    </div>
    <ol>
      <li v-for="row in rows" :key="row.id">
        <NuxtLink
          :to="row.to"
          class="flex min-h-16 items-center gap-3 border-b-2 border-poster-ink px-3 py-2.5"
          :class="[row.here ? 'bg-poster-ink text-poster-on-ink-accent' : 'text-poster-ink hover:bg-poster-paper-sunken', row.faded ? 'opacity-55' : '']"
          :aria-current="row.here ? 'page' : undefined"
        >
          <span class="w-12 shrink-0 font-poster-mono text-sm font-semibold">{{ row.start }}</span>
          <span class="flex w-16 shrink-0 items-center" aria-hidden="true">
            <UiImage
              v-if="row.cover"
              :image="row.cover"
              ratio="16 / 9"
              rounded="rounded-sm"
              frame-class="border-2 border-current"
              class="w-full"
              :class="{ grayscale: row.faded }"
              sizes="4rem"
            />
            <span
              v-else
              class="aspect-video w-full rounded-sm border-2 border-current"
              :style="{ background: row.color ?? 'var(--color-poster-paper-sunken)' }"
            />
          </span>
          <span class="line-clamp-3 min-w-0 flex-1 text-sm leading-snug font-semibold">{{ row.title }}</span>
          <span v-if="row.tag" class="font-poster-mono text-[0.6875rem] font-semibold uppercase">{{ row.tag }}</span>
        </NuxtLink>
      </li>
    </ol>
  </section>
</template>

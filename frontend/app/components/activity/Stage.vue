<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { PublicSessionStream } from '~/types/live'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

/**
 * Ce qui se passe maintenant : le direct avec ses langues, la rediffusion après.
 * Une séance a un flux par langue d'interprétation.
 */

interface Props {
  session: PublicScheduleRow
  state: ProgrammeSessionState
  streams: PublicSessionStream[]
}

const props = defineProps<Props>()

const { t, locale } = useI18n()

const chosen = ref<string | null>(null)

const languageName = (code: string | null) => {
  if (!code) return t('activity.stage.original')
  try {
    return new Intl.DisplayNames([locale.value], { type: 'language' }).of(code) ?? code
  } catch {
    return code
  }
}

const tabs = computed(() =>
  props.streams.map((stream) => ({
    id: stream.id,
    label: languageName(stream.locale),
    primary: stream.is_primary,
  })),
)

const current = computed(
  () => props.streams.find((stream) => stream.id === chosen.value) ?? props.streams[0] ?? null,
)

const isLive = computed(() => props.state === 'live' || (props.state === 'ongoing' && props.streams.length > 0))

const replayMinutes = computed(() =>
  props.session.replay_duration_seconds ? Math.round(props.session.replay_duration_seconds / 60) : null,
)
</script>

<template>
  <section v-if="isLive && props.session.is_streamed" id="direct" aria-labelledby="direct-titre">
    <div class="mb-5 flex flex-wrap items-end justify-between gap-3 border-b-4 border-poster-ink pb-2.5">
      <h2 id="direct-titre" class="font-poster text-[2.375rem] leading-none font-black uppercase font-stretch-[68%]">
        {{ t('activity.stage.liveTitle') }}
      </h2>
      <div
        v-if="tabs.length > 1"
        class="flex overflow-hidden rounded-md border-2 border-poster-ink"
        role="group"
        :aria-label="t('activity.stage.language')"
      >
        <button
          v-for="(tab, index) in tabs"
          :key="tab.id"
          type="button"
          class="h-11 cursor-pointer px-4 text-sm font-bold"
          :class="[
            current?.id === tab.id ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper-raised text-poster-ink',
            index ? 'border-l-2 border-poster-ink' : '',
          ]"
          :aria-pressed="current?.id === tab.id"
          @click="chosen = tab.id"
        >
          {{ tab.label }}<template v-if="tab.primary"> · {{ t('activity.stage.original') }}</template>
        </button>
      </div>
    </div>

    <div class="relative aspect-video overflow-hidden rounded-lg border-2 border-poster-ink bg-poster-ink">
      <iframe
        v-if="current?.embed_url"
        :key="current.id"
        :src="current.embed_url"
        :title="t('activity.stage.player', { language: languageName(current.locale) })"
        class="absolute inset-0 size-full"
        allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
        allowfullscreen
        loading="lazy"
      />
      <div v-else class="absolute inset-0 flex flex-col items-center justify-center gap-4 p-6 text-center text-poster-on-ink">
        <span class="inline-flex size-20 items-center justify-center rounded-full border-[3px] border-poster-on-ink bg-live text-live-contrast">
          <UiIcon name="broadcast" size="2rem" />
        </span>
        <p class="max-w-md text-sm">
          {{ current ? t('activity.stage.external') : t('activity.stage.starting') }}
        </p>
        <a
          v-if="current?.watch_url"
          :href="current.watch_url"
          target="_blank"
          rel="noopener"
          class="inline-flex h-12 items-center gap-2 rounded-md bg-live px-5 font-bold text-live-contrast"
        >
          {{ t('activity.stage.watch') }}
          <UiIcon name="external-link" size="1rem" />
        </a>
      </div>
    </div>
  </section>

  <section v-else-if="props.state === 'past' && props.session.replay_url" aria-labelledby="revoir-titre">
    <h2 id="revoir-titre" class="mb-5 border-b-4 border-poster-ink pb-2.5 font-poster text-[2.375rem] leading-none font-black uppercase font-stretch-[68%]">
      {{ t('activity.stage.replayTitle') }}
    </h2>
    <a
      :href="props.session.replay_url"
      target="_blank"
      rel="noopener"
      class="flex aspect-video flex-col items-center justify-center gap-4 rounded-lg border-2 border-poster-ink bg-poster-ink text-poster-on-ink"
    >
      <span class="inline-flex size-24 items-center justify-center rounded-full border-[3px] border-poster-ink bg-poster-today-strong text-poster-on-today">
        <UiIcon name="video" size="2.25rem" />
      </span>
      <span class="font-poster-mono text-sm">
        {{ replayMinutes ? t('activity.stage.replayDuration', { minutes: replayMinutes }) : t('activity.stage.replay') }}
      </span>
    </a>
  </section>
</template>

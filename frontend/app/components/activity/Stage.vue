<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { PublicSessionStream } from '~/types/live'
import type { AttachedImage } from '~/types/media'
import type { ProgrammeSessionState } from '~/composables/useProgrammeSession'

/**
 * Le média de l'activité, toujours rendu : le direct avec ses langues pendant la
 * diffusion, l'enregistrement après, la couverture sinon — à défaut, le sigle.
 * Une séance a un flux par langue d'interprétation.
 */

interface Props {
  session: PublicScheduleRow
  state: ProgrammeSessionState
  streams: PublicSessionStream[]
  cover: AttachedImage | null
  acronym: string | null
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

const isLive = computed(
  () => props.session.is_streamed && (props.state === 'live' || (props.state === 'ongoing' && props.streams.length > 0)),
)
const replayUrl = computed(() => (props.state === 'past' ? (props.session.replay_url ?? null) : null))
const faded = computed(() => props.state === 'past' || props.state === 'cancelled')

const replayMinutes = computed(() =>
  props.session.replay_duration_seconds ? Math.round(props.session.replay_duration_seconds / 60) : null,
)
</script>

<template>
  <section v-if="isLive" id="direct" class="min-w-0" aria-labelledby="direct-titre">
    <h2 id="direct-titre" class="sr-only">{{ t('activity.stage.liveTitle') }}</h2>
    <div class="relative aspect-video overflow-hidden rounded-lg bg-surface-inverse text-text-on-inverse shadow-lg">
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
      <template v-else>
        <UiImage
          v-if="props.cover"
          :image="props.cover"
          ratio="auto"
          loading="eager"
          class="absolute inset-0 opacity-40"
          frame-class="size-full"
          sizes="(min-width: 1024px) 50rem, 100vw"
        />
        <div class="absolute inset-0 flex flex-col items-center justify-center gap-4 p-6 text-center">
          <span class="inline-flex size-16 items-center justify-center rounded-full bg-live text-live-contrast sm:size-20">
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
            class="inline-flex min-h-12 items-center gap-2 rounded-md bg-live px-5 font-bold text-live-contrast"
          >
            {{ t('activity.stage.watch') }}
            <UiIcon name="external-link" size="1rem" />
          </a>
        </div>
      </template>
      <UiStatusBadge state="live" :label="t('activity.state.live')" class="pointer-events-none absolute top-3 left-3" />
    </div>

    <div v-if="tabs.length > 1" class="mt-3 flex flex-wrap justify-end gap-2" role="group" :aria-label="t('activity.stage.language')">
      <UiButton
        v-for="tab in tabs"
        :key="tab.id"
        variant="ghost"
        :pressed="current?.id === tab.id"
        @click="chosen = tab.id"
      >
        {{ tab.label }}<template v-if="tab.primary"> · {{ t('activity.stage.original') }}</template>
      </UiButton>
    </div>
  </section>

  <section v-else-if="replayUrl" class="min-w-0" aria-labelledby="revoir-titre">
    <h2 id="revoir-titre" class="sr-only">{{ t('activity.stage.replayTitle') }}</h2>
    <a
      :href="replayUrl"
      target="_blank"
      rel="noopener"
      class="group relative block aspect-video overflow-hidden rounded-lg bg-surface-inverse text-text-on-inverse shadow-lg"
    >
      <UiImage
        v-if="props.cover"
        :image="props.cover"
        ratio="auto"
        loading="eager"
        class="absolute inset-0 opacity-50"
        frame-class="size-full grayscale"
        sizes="(min-width: 1024px) 50rem, 100vw"
      />
      <span class="absolute inset-0 flex flex-col items-center justify-center gap-4 p-6 text-center">
        <span
          class="inline-flex size-20 items-center justify-center rounded-full bg-surface-raised text-text shadow-md transition-transform group-hover:scale-105 motion-reduce:transition-none sm:size-22"
        >
          <UiIcon name="video" size="2rem" />
        </span>
        <span class="text-sm font-bold">
          {{ replayMinutes ? t('activity.stage.replayDuration', { minutes: replayMinutes }) : t('activity.stage.replay') }}
        </span>
      </span>
    </a>
  </section>

  <div v-else class="relative aspect-video min-w-0 overflow-hidden rounded-lg bg-surface-inverse shadow-lg">
    <UiImage
      v-if="props.cover"
      :image="props.cover"
      ratio="auto"
      loading="eager"
      class="size-full"
      frame-class="size-full"
      :class="faded ? 'grayscale' : ''"
      sizes="(min-width: 1024px) 50rem, 100vw"
    />
    <span
      v-else-if="props.acronym"
      class="absolute inset-0 flex items-center justify-center overflow-hidden px-6 text-[clamp(3.5rem,14vw,9rem)] leading-none font-bold whitespace-nowrap text-text-on-inverse/15 select-none"
      aria-hidden="true"
    >
      {{ props.acronym }}
    </span>
  </div>
</template>

<script setup lang="ts">
import type { PublicTrackSummary } from '~/types/programme/track-page'

/** Les autres fils publiés de l'édition : on passe de l'atelier aux journées sans revenir à la programmation. */

interface Props {
  tracks: PublicTrackSummary[]
  editionSlug: string
  editionLabel: string
  timezone: string
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const localePath = useLocalePath()
const NuxtLink = resolveComponent('NuxtLink')

const items = computed(() =>
  props.tracks.map((track) => {
    const instant = track.starts_at ? Date.parse(track.starts_at) : track.starts_on ? Date.parse(`${track.starts_on}T12:00:00Z`) : null
    const zone = track.starts_at ? props.timezone : 'UTC'
    const part = (options: Intl.DateTimeFormatOptions) =>
      instant === null ? '' : new Intl.DateTimeFormat(locale.value, { ...options, timeZone: zone }).format(instant)
    const venue = track.venue_name ? tr(track.venue_name) : ''
    const hours = track.starts_at && track.ends_at ? `${time(track.starts_at, props.timezone)} → ${time(track.ends_at, props.timezone)}` : ''
    return {
      slug: track.slug,
      current: track.is_current,
      to: localePath({ name: 'track-edition-track', params: { edition: props.editionSlug, track: track.slug } }),
      weekday: part({ weekday: 'short' }),
      day: part({ day: 'numeric' }),
      title: tr(track.title),
      line: track.is_current ? t('track.others.here') : [venue, track.is_off_site ? '' : hours].filter(Boolean).join(' · '),
      style: trackDayStyle(track.color_hex),
    }
  }),
)
</script>

<template>
  <section v-if="items.length > 1" aria-labelledby="journee-autres-titre">
    <h2 id="journee-autres-titre" class="font-sans text-2xl font-bold text-heading">
      {{ t('track.others.title', { edition: props.editionLabel }) }}
    </h2>
    <ul class="mt-5 grid gap-3 md:grid-cols-3">
      <li v-for="item in items" :key="item.slug" :style="item.style">
        <component
          :is="item.current ? 'div' : NuxtLink"
          :to="item.current ? undefined : item.to"
          :aria-current="item.current ? 'page' : undefined"
          class="grid h-full grid-cols-[auto_minmax(0,1fr)] items-start gap-3 rounded-lg border p-4 text-text no-underline"
          :class="item.current ? 'border-border-subtle bg-surface-sunken' : 'border-border-subtle bg-surface-raised transition-colors hover:border-border-strong'"
        >
          <span
            class="flex w-12 flex-col items-center rounded-md bg-(--color-day-solid) py-1.5 text-[11px] leading-[1.1] font-bold text-(--color-day-on-solid) uppercase"
            aria-hidden="true"
          >
            {{ item.weekday }}<b class="text-xl font-light">{{ item.day }}</b>
          </span>
          <span class="min-w-0">
            <span class="block leading-snug font-bold">{{ item.title }}</span>
            <span class="block text-sm text-text-muted">{{ item.line }}</span>
          </span>
        </component>
      </li>
    </ul>
  </section>
</template>

<script setup lang="ts">
import type { PublicTrackFeature } from '~/types/programme/track-page'

/**
 * L'encart mis en avant (le Guide des négociations pour l'atelier). Documents
 * et liens ont quitté l'encart le 10/11 : ils forment les ressources de la
 * journée (`TrackResources`).
 */

interface Props {
  feature: PublicTrackFeature
}

const props = defineProps<Props>()


const { tr } = useI18nText()
const localePath = useLocalePath()

/** Un chemin du site (« /guide-nego ») reste dans l'onglet ; une adresse absolue en ouvre un. */
const link = computed<{ label: string; to?: string; href?: string } | null>(() => {
  const target = props.feature?.link_url
  if (!target || !props.feature?.link_label) return null
  const label = tr(props.feature.link_label)
  return target.startsWith('/') ? { label, to: localePath(target) } : { label, href: target }
})

</script>

<template>
  <section
    id="encart"
    class="grid scroll-mt-28 items-center gap-5 rounded-lg border-l-4 border-(--color-day-line) bg-surface-raised p-6 shadow-sm sm:grid-cols-[120px_minmax(0,1fr)]"
    aria-labelledby="journee-encart-titre"
  >
    <UiImage v-if="props.feature.visual" :image="props.feature.visual" ratio="3 / 4" rounded="var(--radius-md)" sizes="120px" class="w-30" />
    <div class="min-w-0" :class="{ 'sm:col-span-2': !props.feature.visual }">
      <p v-if="props.feature.kicker" class="heading-condensed text-[1.05rem] tracking-[0.06em] text-(--color-day)">
        {{ tr(props.feature.kicker) }}
      </p>
      <h2 id="journee-encart-titre" class="mt-1 font-condensed text-[1.75rem] leading-[1.08] font-semibold text-heading">{{ tr(props.feature.title) }}</h2>
      <p v-if="props.feature.body" class="mt-2 max-w-(--measure) text-text-secondary">{{ tr(props.feature.body) }}</p>
      <ul class="mt-3 flex flex-wrap gap-x-6">
        <li v-if="link">
          <NuxtLink v-if="link.to" :to="link.to" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
            <UiIcon name="document" size="1.1rem" />{{ link.label }}
          </NuxtLink>
          <a v-else :href="link.href" target="_blank" rel="noopener noreferrer" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
            <UiIcon name="document" size="1.1rem" />{{ link.label }}
          </a>
        </li>
      </ul>
    </div>
  </section>

</template>

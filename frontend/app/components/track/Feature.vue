<script setup lang="ts">
import type { AttachedDocument } from '~/types/media'
import type { PublicTrackFeature } from '~/types/programme/track-page'

/**
 * L'encart mis en avant (le Guide des négociations pour l'atelier) et les
 * documents à télécharger. Sans encart, les documents viennent seuls.
 */

interface Props {
  feature: PublicTrackFeature | null
  documents: AttachedDocument[]
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()

/** Un chemin du site (« /guide-nego ») reste dans l'onglet ; une adresse absolue en ouvre un. */
const link = computed<{ label: string; to?: string; href?: string } | null>(() => {
  const target = props.feature?.link_url
  if (!target || !props.feature?.link_label) return null
  const label = tr(props.feature.link_label)
  return target.startsWith('/') ? { label, to: localePath(target) } : { label, href: target }
})

const size = (bytes: number) =>
  new Intl.NumberFormat(locale.value, { style: 'unit', unit: bytes >= 1_000_000 ? 'megabyte' : 'kilobyte', maximumFractionDigits: 1 }).format(
    bytes >= 1_000_000 ? bytes / 1_000_000 : Math.max(1, Math.round(bytes / 1000)),
  )

const files = computed(() =>
  props.documents.map((document) => ({
    id: document.asset_id,
    url: document.url,
    label: document.caption ? tr(document.caption) : (document.filename ?? t('track.documents.untitled')),
    size: size(document.bytes),
  })),
)
</script>

<template>
  <section
    v-if="props.feature"
    id="encart"
    class="grid scroll-mt-28 items-center gap-5 rounded-lg border-l-4 border-(--color-day-line) bg-surface-raised p-6 shadow-sm sm:grid-cols-[120px_minmax(0,1fr)]"
    aria-labelledby="journee-encart-titre"
  >
    <UiImage v-if="props.feature.visual" :image="props.feature.visual" ratio="3 / 4" rounded="var(--radius-md)" sizes="120px" class="w-30" />
    <div class="min-w-0" :class="{ 'sm:col-span-2': !props.feature.visual }">
      <p v-if="props.feature.kicker" class="text-xs font-bold text-text-muted uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ tr(props.feature.kicker) }}
      </p>
      <h2 id="journee-encart-titre" class="mt-1 font-sans text-xl font-bold text-heading">{{ tr(props.feature.title) }}</h2>
      <p v-if="props.feature.body" class="mt-2 max-w-(--measure) text-text-secondary">{{ tr(props.feature.body) }}</p>
      <p class="mt-3 flex flex-wrap gap-x-5 gap-y-1">
        <UiButton v-if="link" variant="link" icon="document" :to="link.to" :href="link.href">{{ link.label }}</UiButton>
        <UiButton v-for="file in files" :key="file.id" variant="link" icon="download" :href="file.url">
          {{ file.label }}<span class="font-normal text-text-muted"> · {{ file.size }}</span>
        </UiButton>
      </p>
    </div>
  </section>

  <section v-else-if="files.length" aria-labelledby="journee-documents-titre">
    <h2 id="journee-documents-titre" class="font-sans text-2xl font-bold text-heading">{{ t('track.documents.title') }}</h2>
    <ul class="mt-4 flex flex-col gap-1">
      <li v-for="file in files" :key="file.id">
        <UiButton variant="link" icon="download" :href="file.url">
          {{ file.label }}<span class="font-normal text-text-muted"> · {{ file.size }}</span>
        </UiButton>
      </li>
    </ul>
  </section>
</template>

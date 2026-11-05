<script setup lang="ts">
import type { AttachedDocument } from '~/types/media'
import type { PublicTrackFeature, PublicTrackLink } from '~/types/programme/track-page'

/**
 * L'encart mis en avant (le Guide des négociations pour l'atelier) et les
 * documents à télécharger, puis les liens vers une page de téléchargement
 * (nouvel onglet, nom du site en regard). Sans encart, ils viennent seuls.
 */

interface Props {
  feature: PublicTrackFeature | null
  documents: AttachedDocument[]
  links: PublicTrackLink[]
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

/** Un chemin du site reste dans l'onglet et ne nomme pas de site. */
const pages = computed(() =>
  props.links.map((link) => {
    const internal = link.url.startsWith('/')
    let site: string | null = null
    if (!internal) {
      try {
        site = new URL(link.url).hostname.replace(/^www\./, '')
      } catch {
        site = null
      }
    }
    return { url: link.url, label: tr(link.label), site, to: internal ? localePath(link.url) : undefined }
  }),
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
        <li v-for="file in files" :key="file.id">
          <a :href="file.url" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
            <UiIcon name="download" size="1.1rem" />{{ file.label }}<small class="text-sm font-normal text-text-muted">{{ file.size }}</small>
          </a>
        </li>
        <li v-for="page in pages" :key="page.url">
          <NuxtLink v-if="page.to" :to="page.to" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
            <UiIcon name="external-link" size="1.1rem" />{{ page.label }}
          </NuxtLink>
          <a v-else :href="page.url" target="_blank" rel="noopener noreferrer" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
            <UiIcon name="external-link" size="1.1rem" />{{ page.label }}<small v-if="page.site" class="text-sm font-normal text-text-muted">{{ page.site }}</small>
          </a>
        </li>
      </ul>
    </div>
  </section>

  <section v-else-if="files.length || pages.length" aria-labelledby="journee-documents-titre">
    <h2 id="journee-documents-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">{{ t('track.documents.title') }}</h2>
    <ul class="mt-3 flex flex-col">
      <li v-for="file in files" :key="file.id">
        <a :href="file.url" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
          <UiIcon name="document" size="1.1rem" />{{ file.label }}<small class="text-sm font-normal text-text-muted">{{ file.size }}</small>
        </a>
      </li>
      <li v-for="page in pages" :key="page.url">
        <NuxtLink v-if="page.to" :to="page.to" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
          <UiIcon name="external-link" size="1.1rem" />{{ page.label }}
        </NuxtLink>
        <a v-else :href="page.url" target="_blank" rel="noopener noreferrer" class="inline-flex min-h-11 items-center gap-2.5 font-semibold text-accent no-underline hover:underline">
          <UiIcon name="external-link" size="1.1rem" />{{ page.label }}<small v-if="page.site" class="text-sm font-normal text-text-muted">{{ page.site }}</small>
        </a>
      </li>
    </ul>
  </section>
</template>

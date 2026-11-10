<script setup lang="ts">
import type { AttachedDocument } from '~/types/media'
import type { PublicTrackLink } from '~/types/programme/track-page'

/**
 * LES RESSOURCES D'UNE JOURNÉE (10/11), maquette `docs/maquettes/ressources-journee/`,
 * direction « dossier » : chaque ressource a la forme de ce qu'elle est. La
 * première vidéo tient la grande place, vignette en plein cadre, et se regarde
 * dans une fenêtre ; une page est une petite fenêtre de navigateur avec son
 * adresse ; un PDF, déposé ou lié, une feuille au coin plié. Le genre se lit
 * dans l'adresse (`resourceKindOf`). Un lien daté n'arrive qu'à sa date : l'API
 * ne le rend pas avant.
 */

interface Props {
  documents: AttachedDocument[]
  links: PublicTrackLink[]
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const k = (key: string, params: Record<string, unknown> = {}) => t(`track.resources.${key}`, params)

interface Resource {
  key: string
  kind: 'video' | 'pdf' | 'page'
  label: string
  href: string
  /** Chemin du site : il reste dans l'onglet. */
  to?: string
  /** Hébergé ailleurs : un nouvel onglet. */
  external: boolean
  videoId: string | null
  /** Le site, ou le poids d'un PDF déposé. */
  meta: string | null
  /** Le chemin après le site, pour la barre d'adresse d'une page. */
  path: string
}

const size = (bytes: number) =>
  new Intl.NumberFormat(locale.value, { style: 'unit', unit: bytes >= 1_000_000 ? 'megabyte' : 'kilobyte', maximumFractionDigits: 1 }).format(
    bytes >= 1_000_000 ? bytes / 1_000_000 : Math.max(1, Math.round(bytes / 1000)),
  )

function pathOf(address: string): string {
  try {
    const url = new URL(address)
    return `${url.pathname}${url.search}`.replace(/\/$/, '')
  } catch {
    return ''
  }
}

const resources = computed<Resource[]>(() => [
  ...props.links.map((link, index) => {
    const kind = resourceKindOf(link.url)
    const internal = link.url.startsWith('/')
    return {
      key: `lien-${index}`,
      kind: kind.kind,
      label: tr(link.label),
      href: link.url,
      to: internal ? localePath(link.url) : undefined,
      external: !internal,
      videoId: kind.videoId,
      meta: kind.kind === 'video' ? 'YouTube' : kind.site,
      path: pathOf(link.url),
    }
  }),
  ...props.documents.map((document) => ({
    key: document.asset_id,
    kind: 'pdf' as const,
    label: document.caption ? tr(document.caption) : (document.filename ?? t('track.documents.untitled')),
    href: document.url,
    external: false,
    videoId: null,
    meta: size(document.bytes),
    path: '',
  })),
])

/** La première vidéo à la une ; les suivantes rejoignent la colonne. */
const featured = computed(() => resources.value.find((resource) => resource.kind === 'video') ?? null)
const others = computed(() => resources.value.filter((resource) => resource !== featured.value))

const NuxtLink = resolveComponent('NuxtLink')

const playing = ref<Resource | null>(null)
const isPlaying = computed({
  get: () => playing.value !== null,
  set: (value: boolean) => {
    if (!value) playing.value = null
  },
})
</script>

<template>
  <section v-if="resources.length" id="ressources" class="scroll-mt-28" aria-labelledby="journee-ressources-titre">
    <h2 id="journee-ressources-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">{{ k('title') }}</h2>

    <div class="mt-4 grid gap-3.5" :class="featured ? 'md:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]' : 'sm:grid-cols-[repeat(auto-fill,minmax(15rem,1fr))]'">
      <button
        v-if="featured?.videoId"
        type="button"
        class="resource-video group relative isolate min-h-0 cursor-pointer overflow-hidden rounded-lg bg-surface-inverse text-left shadow-md max-md:aspect-video md:row-span-3 md:min-h-75"
        :aria-label="k('watchNamed', { title: featured.label })"
        @click="playing = featured"
      >
        <img
          :src="youtubeThumbnail(featured.videoId)"
          alt=""
          loading="lazy"
          class="absolute inset-0 -z-10 size-full scale-[1.34] object-cover transition-transform duration-500 group-hover:scale-[1.39]"
        >
        <span class="absolute inset-0 -z-10 scrim-fade-bottom" aria-hidden="true" />
        <span
          class="absolute top-[42%] left-1/2 grid size-14 -translate-1/2 place-items-center rounded-full bg-live text-live-contrast ring-5 ring-text-on-inverse/80 transition-transform group-hover:scale-110 md:size-17"
          aria-hidden="true"
        >
          <svg viewBox="0 0 12 14" class="ms-1 h-5 w-4.5" fill="currentColor"><path d="M0 0l12 7-12 7z" /></svg>
        </span>
        <span class="absolute inset-x-0 bottom-0 p-4 text-text-on-inverse md:p-5.5">
          <span class="font-condensed text-[0.9rem] font-bold tracking-[0.08em] uppercase opacity-85">{{ k('video') }} · YouTube</span>
          <span class="mt-1 block font-condensed text-[1.3rem] leading-[1.08] font-semibold text-balance md:text-[1.65rem]">{{ featured.label }}</span>
        </span>
      </button>

      <template v-for="resource in others" :key="resource.key">
        <!-- Une page : une fenêtre de navigateur, sa barre et son adresse. -->
        <component
          :is="resource.to ? NuxtLink : 'a'"
          v-if="resource.kind === 'page'"
          :to="resource.to"
          :href="resource.to ? undefined : resource.href"
          :target="resource.to ? undefined : '_blank'"
          :rel="resource.to ? undefined : 'noopener noreferrer'"
          class="resource-card group overflow-hidden rounded-md border border-border-subtle bg-surface-raised text-inherit no-underline shadow-sm hover:shadow-md"
        >
          <span class="flex items-center gap-2.5 bg-surface-sunken px-3 py-2" aria-hidden="true">
            <span class="flex gap-1.25"><i v-for="dot in 3" :key="dot" class="size-2 rounded-full bg-border-strong opacity-55" /></span>
            <span class="min-w-0 flex-1 truncate rounded-full bg-surface-raised px-2.5 py-0.5 text-[0.74rem] text-text-muted">
              <b class="font-semibold text-text-secondary">{{ resource.meta ?? k('thisSite') }}</b>{{ resource.path }}
            </span>
          </span>
          <span class="block px-4 pt-3.5 pb-3.5">
            <span class="block font-condensed text-[0.85rem] font-bold tracking-[0.08em] text-(--color-day) uppercase">{{ k('page') }}</span>
            <span class="mt-1 line-clamp-4 font-condensed text-[1.14rem] leading-[1.15] font-semibold text-heading">{{ resource.label }}</span>
            <span class="mt-2 flex w-fit items-center gap-1.5 text-[0.88rem] font-semibold text-accent group-hover:underline">
              {{ k('open') }}<UiIcon :name="resource.to ? 'arrow-right' : 'external-link'" size="0.95rem" />
            </span>
          </span>
        </component>

        <!-- Une vidéo de plus : une vignette réduite, la même fenêtre. -->
        <button
          v-else-if="resource.kind === 'video' && resource.videoId"
          type="button"
          class="resource-card group grid cursor-pointer grid-cols-[6rem_minmax(0,1fr)] items-center gap-3.5 rounded-md border border-border-subtle bg-surface-raised p-3 text-left shadow-sm hover:shadow-md"
          :aria-label="k('watchNamed', { title: resource.label })"
          @click="playing = resource"
        >
          <span class="relative aspect-video overflow-hidden rounded-sm bg-surface-inverse" aria-hidden="true">
            <img :src="youtubeThumbnail(resource.videoId)" alt="" loading="lazy" class="size-full scale-[1.34] object-cover">
            <span class="absolute top-1/2 left-1/2 grid size-7 -translate-1/2 place-items-center rounded-full bg-live text-live-contrast">
              <svg viewBox="0 0 12 14" class="ms-0.5 h-2.5 w-2" fill="currentColor"><path d="M0 0l12 7-12 7z" /></svg>
            </span>
          </span>
          <span class="min-w-0">
            <span class="block font-condensed text-[0.85rem] font-bold tracking-[0.08em] text-(--color-day) uppercase">{{ k('video') }}</span>
            <span class="mt-0.5 line-clamp-3 font-condensed text-[1.06rem] leading-[1.15] font-semibold text-heading">{{ resource.label }}</span>
            <span class="mt-1.5 flex w-fit items-center gap-1.5 text-[0.88rem] font-semibold text-accent group-hover:underline">{{ k('watch') }}</span>
          </span>
        </button>

        <!-- Un PDF : une feuille au coin plié. -->
        <a
          v-else
          :href="resource.href"
          :target="resource.external ? '_blank' : undefined"
          :rel="resource.external ? 'noopener noreferrer' : undefined"
          class="resource-card group grid grid-cols-[4rem_minmax(0,1fr)] items-center gap-3.5 rounded-md border border-border-subtle bg-surface-raised py-3 ps-3 pe-4 text-inherit no-underline shadow-sm hover:shadow-md"
        >
          <span class="resource-sheet relative aspect-[3/4] rounded-xs bg-(--color-day-surface)" aria-hidden="true">
            <b class="absolute start-1.5 bottom-1.5 rounded-xs bg-(--color-day-solid) px-1.25 py-px font-condensed text-[0.72rem] font-bold tracking-[0.04em] text-(--color-day-on-solid)">PDF</b>
          </span>
          <span class="min-w-0">
            <span class="line-clamp-3 font-condensed text-[1.06rem] leading-[1.15] font-semibold text-heading">{{ resource.label }}</span>
            <span v-if="resource.meta" class="mt-0.5 block text-[0.8rem] text-text-muted">{{ resource.meta }}</span>
            <span class="mt-1.5 flex w-fit items-center gap-1.5 text-[0.88rem] font-semibold text-accent group-hover:underline">
              {{ k('download') }}<UiIcon name="download" size="0.95rem" />
            </span>
          </span>
        </a>
      </template>
    </div>

    <UiModal v-model:open="isPlaying" :title="playing?.label ?? ''" size="xl">
      <div v-if="playing?.videoId" class="grid gap-3">
        <iframe
          :src="youtubeEmbed(playing.videoId)"
          :title="playing.label"
          class="aspect-video w-full rounded-md border-0 bg-surface-inverse"
          allow="autoplay; encrypted-media; picture-in-picture; fullscreen"
          allowfullscreen
          referrerpolicy="strict-origin-when-cross-origin"
        />
        <a :href="youtubeWatch(playing.videoId)" target="_blank" rel="noopener noreferrer" class="inline-flex min-h-11 w-fit items-center gap-2 text-sm font-semibold text-accent">
          {{ k('openOnYoutube') }}<UiIcon name="external-link" size="0.95rem" />
        </a>
      </div>
    </UiModal>
  </section>
</template>

<style scoped>
.resource-card {
  transition:
    transform var(--duration-fast) ease-out,
    box-shadow var(--duration-fast) ease-out;
}

/* Le coin plié, et les lignes d'un texte esquissé. */
.resource-sheet {
  clip-path: polygon(0 0, 72% 0, 100% 22%, 100% 100%, 0 100%);
}

.resource-sheet::before {
  content: "";
  position: absolute;
  top: 0;
  right: 0;
  width: 28%;
  height: 22%;
  background: color-mix(in srgb, var(--color-day-solid) 45%, var(--color-surface-raised));
  clip-path: polygon(0 0, 100% 100%, 0 100%);
}

.resource-sheet::after {
  content: "";
  position: absolute;
  inset: 40% 12px auto 10px;
  height: 22%;
  background: repeating-linear-gradient(to bottom, color-mix(in srgb, var(--color-day) 30%, transparent) 0 2px, transparent 2px 6px);
}

@media (prefers-reduced-motion: no-preference) {
  .resource-card:hover,
  .resource-video:hover {
    transform: translateY(-2px);
  }
}
</style>

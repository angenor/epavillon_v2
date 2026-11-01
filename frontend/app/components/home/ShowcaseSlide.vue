<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'

/**
 * UNE DIAPOSITIVE DE LA VITRINE : une ligne de `content.v_showcase`, rendue.
 *
 * PARTAGÉE AVEC LE BACK-OFFICE : l'aperçu de `/admin/vitrine/[id]` rend ce
 * composant, pas une seconde mise en page, sans quoi l'éditeur compose à
 * l'aveugle. D'où une interface de props CLOSE : tout entre par `slide`, rien
 * ne vient du contexte (ni route, ni store, ni instant). Le composant remplit
 * la boîte qu'on lui donne ; l'appelant décide du cadre.
 *
 * Le fond (vidéo, image, aplat, surface institutionnelle) se décide dans
 * `utils/showcase.ts`. Le voile est le jeton `--color-scrim`, et le texte ne
 * s'inverse pas en thème sombre : une photographie voilée est sombre dans les
 * deux thèmes.
 *
 * `body` n'est pas montré : c'est le texte long de la page de détail, il
 * crèverait le bandeau.
 *
 * Direction condensée (01/11) : citation en Barlow Condensed 600 sans
 * capitales, guillemets en cyan clair, auteur en capitales condensées dessous.
 * Les polices sont déclarées pour tout le site : l'aperçu du back-office les a
 * aussi.
 */

interface Props {
  /** La ligne de `content.v_showcase`. Seule source de la diapositive. */
  slide: ShowcaseRow
  /** Ne pas tenter la vidéo : elle a échoué, ou la personne a demandé moins d'animations. */
  skipVideo?: boolean
  /** Le défilement est en pause : le fond animé s'arrête avec lui. */
  paused?: boolean
  /**
   * Classes du bloc de texte, posées DANS le cadre centré : c'est par là que le
   * bandeau réserve la place du rail (en bas) et du panneau « À venir » (à
   * droite sur grand écran).
   */
  contentClass?: string
  /** Aperçu du back-office : mêmes éléments, échelle réduite. */
  compact?: boolean
  /** Chargement immédiat de l'image : vrai pour la première diapositive. */
  eager?: boolean
}

const props = withDefaults(defineProps<Props>(), { compact: false })

/** Un fond animé qui ne charge pas fait passer à la suivante : quinze secondes de noir valent moins qu'une diapositive sautée. */
const emit = defineEmits<{ 'media-error': [] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const background = computed(() => showcaseBackground(props.slide, { skipVideo: props.skipVideo }))

const title = computed(() => tr(props.slide.title))
const quote = computed(() => tr(props.slide.quote))

/** La citation porte l'écran quand elle existe ; sinon c'est le titre. */
const headline = computed(() => quote.value || title.value)
const hasQuote = computed(() => Boolean(quote.value))

/** « Organisation (SIGLE), Pays » : les deux moitiés sont facultatives. */
const affiliation = computed(() => {
  const org = props.slide.organization_name
  const acronym = props.slide.organization_acronym
  const country = tr(props.slide.country_name)
  const named = org ? (acronym && acronym !== org ? `${org} (${acronym})` : org) : ''
  return [named, country].filter(Boolean).join(', ')
})

const role = computed(() => [tr(props.slide.author_title), affiliation.value].filter(Boolean).join(', '))

const linkLabel = computed(() => tr(props.slide.link_label) || t('home.showcase.discover'))

/**
 * La taille suit la longueur, en caractères et non en mots (le français aligne
 * des mots longs) : une accroche courte porte l'écran en très grand, un
 * paragraphe descend d'un cran. `line-clamp` reste en dernier recours.
 */
const headlineSize = computed(() => {
  const length = headline.value.length
  if (length <= 95) return 'max-w-[22ch] text-[clamp(2.2rem,4.6vw,4rem)]'
  if (length <= 170) return 'max-w-[28ch] text-[clamp(1.9rem,3.4vw,3rem)]'
  return 'max-w-[36ch] text-[clamp(1.5rem,2.4vw,2.1rem)]'
})

/**
 * Le fond animé suit le carrousel : sourdine et pause avec lui. Pas d'attribut
 * `autoplay` : le navigateur lancerait la lecture après notre `pause()`, la
 * lecture est donc commandée ici, au montage et à `loadeddata`.
 */
const video = ref<HTMLVideoElement | null>(null)

function syncVideo(): void {
  const element = video.value
  if (!element) return
  element.muted = true
  if (props.paused) element.pause()
  else void element.play().catch(() => undefined)
}

onMounted(syncVideo)
// `flush: 'post'` : la balise `<video>` naît et meurt avec le type de fond.
watch(() => [props.paused, background.value.kind], syncVideo, { flush: 'post' })
</script>

<template>
  <article
    class="relative isolate flex h-full w-full flex-col justify-end overflow-hidden bg-surface-inverse text-text-on-inverse"
    :aria-roledescription="t('home.showcase.slideRole')"
    :aria-label="title"
  >
    <div
      class="absolute inset-0 -z-10"
      :style="background.kind === 'color' ? { backgroundColor: background.color } : undefined"
      aria-hidden="true"
    >
      <video
        v-if="background.kind === 'video'"
        ref="video"
        :src="background.video.url"
        :poster="background.poster?.url"
        class="size-full object-cover"
        muted
        loop
        playsinline
        preload="metadata"
        tabindex="-1"
        @loadeddata="syncVideo"
        @error="emit('media-error')"
      />
      <UiImage
        v-else-if="background.kind === 'image'"
        :image="background.image"
        ratio="auto"
        frame-class="size-full"
        class="size-full"
        :loading="props.eager ? 'eager' : 'lazy'"
        sizes="100vw"
      />
    </div>

    <div class="absolute inset-0 -z-10 bg-scrim/45 lg:bg-scrim/30" aria-hidden="true" />
    <div class="scrim-fade-bottom absolute inset-x-0 bottom-0 -z-10 h-4/5" aria-hidden="true" />

    <div class="w-full" data-showcase-text>
      <div
        class="mx-auto w-full max-w-[1280px]"
        :class="props.compact ? 'px-4 py-5 sm:px-6' : 'px-4 pt-10 pb-8 sm:px-6 lg:px-8'"
      >
        <figure class="m-0 min-w-0" :class="props.contentClass">
          <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
            <HomeNatureBadge
              :label="props.slide.nature_label"
              :color="props.slide.nature_color"
              tone="inverse"
              :size="props.compact ? 'sm' : 'md'"
            />
            <span v-if="hasQuote" class="text-sm font-medium text-text-on-inverse-muted">{{ title }}</span>
          </div>

          <blockquote
            class="m-0 font-condensed leading-[1.02] font-semibold tracking-[0.005em] text-balance text-text-on-inverse"
            :class="props.compact ? 'mt-3 line-clamp-4 text-2xl' : `mt-4 line-clamp-5 ${headlineSize}`"
          >
            <span v-if="hasQuote" class="text-accent-on-inverse" aria-hidden="true">{{ t('home.showcase.quoteOpen') }}</span>{{ headline }}<span v-if="hasQuote" class="text-accent-on-inverse" aria-hidden="true">{{ t('home.showcase.quoteClose') }}</span>
          </blockquote>

          <figcaption :class="props.compact ? 'mt-3 text-sm' : 'mt-5 text-[0.95rem]'">
            <!-- Un témoignage sans son auteur n'engage personne. -->
            <span
              v-if="props.slide.author_name"
              class="block font-condensed font-bold tracking-[0.04em] text-text-on-inverse uppercase"
              :class="props.compact ? 'text-base' : 'text-xl'"
            >
              {{ props.slide.author_name }}
            </span>
            <span v-if="role" class="block text-text-on-inverse-muted">{{ role }}</span>
            <span
              v-if="props.slide.event_title || props.slide.session_title"
              class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-text-on-inverse-muted"
            >
              <span v-if="props.slide.event_title" class="font-medium">{{ tr(props.slide.event_title) }}</span>
              <span v-if="props.slide.session_title">{{ tr(props.slide.session_title) }}</span>
              <UiZonedTime
                v-if="props.slide.session_starts_at && props.slide.session_timezone"
                :start="props.slide.session_starts_at"
                :end="props.slide.session_ends_at"
                :timezone="props.slide.session_timezone"
                format="short"
              />
            </span>
            <!-- Un lien et non un bouton : « En savoir plus » déplace, il n'engage pas (règle n° 1). -->
            <a
              v-if="props.slide.link_url"
              :href="props.slide.link_url"
              target="_blank"
              rel="noopener noreferrer"
              class="mt-1 inline-flex min-h-(--target-min) items-center gap-2 border-b-(length:--border-medium) border-text-on-inverse font-medium text-text-on-inverse no-underline transition-colors hover:border-accent-solid"
            >
              {{ linkLabel }}
              <UiIcon name="arrow-up-right" size="1rem" />
              <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
            </a>
          </figcaption>
        </figure>
      </div>
    </div>
  </article>
</template>

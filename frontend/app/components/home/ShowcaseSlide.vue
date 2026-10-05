<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'

/**
 * UNE DIAPOSITIVE DE LA VITRINE — une ligne de `content.v_showcase`, rendue.
 *
 * ── CE COMPOSANT EST PARTAGÉ AVEC LE BACK-OFFICE, ET C'EST SA RAISON D'ÊTRE ──
 *
 * L'aperçu du formulaire de vitrine (`/admin/vitrine/[id]`) rend CE composant,
 * pas une seconde mise en page. Une maquette d'aperçu écrite à part diverge en
 * quelques semaines, et l'éditeur compose alors à l'aveugle : il voit un objet
 * qui n'est pas celui que le visiteur verra.
 *
 * D'où une interface de props CLOSE : tout ce que la diapositive affiche entre
 * par `slide`, rien ne vient du contexte — ni route, ni store, ni instant.
 * `api.adminShowcase.form()` rend d'ailleurs `preview: ShowcaseRow`, exactement
 * le type attendu ici.
 *
 *     <div class="aspect-[16/9]">
 *       <HomeShowcaseSlide :slide="form.preview" compact />
 *     </div>
 *
 * Le composant remplit la boîte qu'on lui donne (`h-full w-full`) et n'impose
 * aucune hauteur : c'est l'appelant qui décide du cadre, comme pour `UiImage`.
 *
 * ── LE FOND SE DÉCIDE D'APRÈS LE MÉDIA REÇU ─────────────────────────────────
 *
 * Vidéo, puis image, puis aplat, puis surface institutionnelle : l'échelle est
 * dans `utils/showcase.ts` et n'est écrite qu'une fois. Aucune colonne ne
 * prétend savoir s'il y a une vidéo — la vue ne sert que des objets `ready`,
 * une vidéo en cours de traitement arrive donc `null` et le repli joue seul.
 *
 * ── LE VOILE EST UN JETON, PAS UN NOIR À 50 % ───────────────────────────────
 *
 * `--color-scrim` existe précisément pour ce cas : un voile léger sur toute
 * l'image, et le fondu `.scrim-fade-bottom` sous le texte, posé en bas de cadre
 * depuis le 03/10 pour laisser la photographie libre en son milieu.
 *
 * ── LE BANDEAU NE PORTE PAS `body` ──────────────────────────────────────────
 *
 * `content.highlights.body` est documenté comme le texte long, « pour la page
 * de détail ». Le poser ici allongeait la diapositive au-delà de la hauteur du
 * bandeau, qui la rognait alors en haut comme en bas. Ce que la diapositive
 * montre est ce que la maquette annonce : éyclette, citation, attribution.
 *
 * ── LE TEXTE NE S'INVERSE PAS ───────────────────────────────────────────────
 *
 * `--color-text-on-inverse` et `--color-text-on-inverse-muted` gardent la même
 * valeur en thème sombre : une photographie voilée est sombre dans les deux
 * thèmes, et un texte qui s'y inverserait deviendrait illisible la nuit.
 */

interface Props {
  /** La ligne de `content.v_showcase`. Seule source de la diapositive. */
  slide: ShowcaseRow
  /**
   * Ne pas tenter la vidéo. Deux appelants, deux raisons : la vidéo a déjà
   * échoué au chargement, ou la personne a demandé moins d'animations. Le
   * composant ne connaît ni l'une ni l'autre — il obéit.
   */
  skipVideo?: boolean
  /** Le défilement est en pause : le fond animé s'arrête avec lui. */
  paused?: boolean
  /**
   * Classes du bloc de texte. C'est par là que le bandeau dégage la colonne
   * « À venir » posée à sa gauche sur grand écran — un décalage explicite, et
   * non une valeur devinée depuis le contexte.
   */
  contentClass?: string
  /** Aperçu du back-office : mêmes éléments, échelle réduite. */
  compact?: boolean
  /** Chargement immédiat de l'image — vrai pour la première diapositive. */
  eager?: boolean
}

const props = withDefaults(defineProps<Props>(), { compact: false })

/**
 * L'erreur de chargement d'un fond animé fait passer à la diapositive suivante.
 * C'était le comportement de la v1 et il est juste : quinze secondes de cadre
 * noir sont pires qu'une diapositive sautée.
 */
const emit = defineEmits<{ 'media-error': [] }>()

const { t } = useI18n()
const { tr } = useI18nText()

const background = computed(() => showcaseBackground(props.slide, { skipVideo: props.skipVideo }))

const title = computed(() => tr(props.slide.title))
const quote = computed(() => tr(props.slide.quote))

/** La citation porte l'écran quand elle existe ; sinon c'est le titre. */
const headline = computed(() => quote.value || title.value)
const hasQuote = computed(() => Boolean(quote.value))

/** « Organisation (SIGLE) · Pays » — les deux moitiés sont facultatives. */
const affiliation = computed(() => {
  const org = props.slide.organization_name
  const acronym = props.slide.organization_acronym
  const country = tr(props.slide.country_name)
  const named = org ? (acronym && acronym !== org ? `${org} (${acronym})` : org) : ''
  return [named, country].filter(Boolean).join(' · ')
})

const hasAside = computed(() =>
  Boolean(
    props.slide.author_name ||
      affiliation.value ||
      props.slide.author_title ||
      props.slide.event_title ||
      props.slide.session_title ||
      props.slide.link_url,
  ),
)

const linkLabel = computed(() => tr(props.slide.link_label) || t('home.showcase.discover'))

/**
 * LA TAILLE DE LA CITATION SUIT SA LONGUEUR, et c'est une règle d'édition avant
 * d'être une règle de mise en page.
 *
 * Le bandeau fait désormais exactement un écran : la citation ne peut plus
 * s'étirer, et une taille unique tronquait quatre diapositives sur sept —
 * toujours au milieu d'un mot, ce qui est le pire endroit. Trois paliers :
 * une accroche courte porte l'écran en très grand, une citation de deux phrases
 * se lit en corps intermédiaire, un paragraphe entier descend au titre de
 * section. C'est ce que fait un maquettiste devant la même contrainte.
 *
 * LES SEUILS SONT EN CARACTÈRES ET NON EN MOTS : le français aligne des mots
 * longs, et compter les mots ferait passer « Institutionnalisation » pour une
 * unité aussi courte que « et ».
 *
 * `line-clamp` reste, en dernier recours : une citation de quatre cents signes
 * dépasserait encore, et il vaut mieux une fin coupée qu'un bandeau crevé.
 * Mais il n'a plus à s'exercer sur les extraits d'une longueur normale.
 */
const headlineSize = computed(() => {
  const length = headline.value.length
  if (length <= 95) return 'text-display'
  if (length <= 170) return 'text-display-sm'
  return 'text-2xl'
})

/**
 * Le fond animé est mis en sourdine ET mis en pause AVEC LE CARROUSEL. Une
 * vidéo qui continue de tourner derrière un défilement arrêté n'a pas arrêté
 * grand-chose, et c'est bien le mouvement qu'on nous demande de suspendre.
 *
 * `muted` est posé par le DOM et pas seulement par l'attribut : sans lui, la
 * lecture automatique est refusée par tous les navigateurs et le fond resterait
 * figé sur son affiche.
 *
 * IL N'Y A PAS D'ATTRIBUT `autoplay`, ET C'EST LE POINT DÉLICAT. La lecture
 * automatique du navigateur démarre quand le média est prêt, c'est-à-dire APRÈS
 * `onMounted` : une diapositive atteinte alors que le défilement est déjà en
 * pause se mettait donc à jouer toute seule, notre `pause()` ayant été appelé
 * avant que le navigateur ne décide de lire. La lecture est ici commandée par
 * le composant, au montage et à `loadeddata`, jamais par l'attribut.
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
// `flush: 'post'` : la balise `<video>` naît et meurt avec le type de fond ;
// sans cela, la synchronisation viserait l'élément du rendu précédent.
watch(() => [props.paused, background.value.kind], syncVideo, { flush: 'post' })
</script>

<template>
  <article
    class="relative isolate flex h-full w-full flex-col justify-end overflow-hidden bg-surface-inverse text-text-on-inverse"
    :aria-roledescription="t('home.showcase.slideRole')"
    :aria-label="title"
  >
    <!-- LE FOND. Un seul des quatre cas est rendu : la décision est prise dans
         `utils/showcase.ts`, pas dans une cascade de `v-if` ici. -->
    <div
      class="absolute inset-0 -z-10"
      :style="
        background.kind === 'color' ? { backgroundColor: background.color } : undefined
      "
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

    <!-- Voile allégé sur grand écran le 03/10 : le texte, en bas de cadre, y
         tient par le fondu. Sur téléphone il monte jusqu'au milieu de l'image. -->
    <div class="absolute inset-0 -z-10 bg-scrim/45 lg:bg-scrim/20" aria-hidden="true" />
    <div class="scrim-fade-bottom absolute inset-x-0 bottom-0 -z-10 h-4/5" aria-hidden="true" />

    <!-- BAS DE CADRE, arbitré le 03/10 : le texte se range au-dessus du rail et
         laisse le haut de la photographie libre. -->
    <div class="w-full" :class="props.contentClass" data-showcase-text>
      <div
        class="mx-auto flex w-full max-w-[1280px]"
        :class="
          props.compact
            ? 'flex-col gap-3 px-4 py-5 sm:px-6'
            : 'flex-col gap-5 px-4 py-8 sm:px-6 sm:py-10 lg:flex-row lg:items-end lg:gap-12'
        "
      >
        <div class="min-w-0 flex-1" :style="{ maxWidth: '52rem' }">
          <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
            <HomeNatureBadge
              :label="props.slide.nature_label"
              :color="props.slide.nature_color"
              tone="inverse"
              :size="props.compact ? 'sm' : 'md'"
            />
            <span
              v-if="hasQuote"
              class="font-medium text-text-on-inverse-muted"
              :class="props.compact ? 'text-sm' : 'text-base'"
            >
              {{ title }}
            </span>
          </div>
          <!-- `line-clamp` est un garde-fou : le bandeau fait un écran et rogne ce qui dépasse. -->
          <div class="relative" :class="[props.compact ? 'mt-2' : 'mt-4', { 'ps-8 sm:ps-11': hasQuote }]">
            <span
              v-if="hasQuote"
              class="absolute -top-2 start-0 font-display text-5xl leading-none font-bold text-accent-on-inverse sm:-top-3 sm:text-7xl"
              aria-hidden="true"
            >
              {{ t('home.showcase.quoteMark') }}
            </span>
            <p
              class="font-display font-bold text-balance text-text-on-inverse"
              :class="props.compact ? 'line-clamp-4 text-display-sm' : `line-clamp-5 ${headlineSize}`"
              :style="{ lineHeight: 'var(--leading-tight)', letterSpacing: 'var(--tracking-title)' }"
            >
              {{ headline }}
            </p>
          </div>
        </div>

        <div
          v-if="hasAside"
          class="shrink-0 border-glass-border-strong"
          :class="
            props.compact
              ? 'border-t pt-3'
              : 'border-t pt-4 lg:w-72 lg:border-t-0 lg:border-s-2 lg:ps-6 lg:pt-0'
          "
        >
          <!-- Un témoignage sans son auteur n'engage personne. -->
          <p v-if="props.slide.author_name || affiliation" :class="props.compact ? 'text-sm' : 'text-base'">
            <span v-if="props.slide.author_name" class="block font-bold">{{ props.slide.author_name }}</span>
            <span
              v-if="props.slide.author_title || affiliation"
              class="block text-sm text-text-on-inverse-muted"
            >
              {{ [tr(props.slide.author_title), affiliation].filter(Boolean).join(' · ') }}
            </span>
          </p>
          <p
            v-if="props.slide.event_title || props.slide.session_title"
            class="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-text-on-inverse-muted"
          >
            <span v-if="props.slide.event_title" class="font-medium">
              {{ tr(props.slide.event_title) }}
            </span>
            <span v-if="props.slide.event_title && props.slide.session_title" aria-hidden="true">·</span>
            <span v-if="props.slide.session_title">{{ tr(props.slide.session_title) }}</span>
            <UiZonedTime
              v-if="props.slide.session_starts_at && props.slide.session_timezone"
              :start="props.slide.session_starts_at"
              :end="props.slide.session_ends_at"
              :timezone="props.slide.session_timezone"
              format="short"
            />
          </p>
          <!-- Un lien et non un bouton : « En savoir plus » déplace, il n'engage pas (règle n° 1). -->
          <a
            v-if="props.slide.link_url"
            :href="props.slide.link_url"
            target="_blank"
            rel="noopener noreferrer"
            class="mt-2 inline-flex min-h-(--target-min) items-center gap-2 border-b-(length:--border-medium) border-text-on-inverse font-medium text-text-on-inverse no-underline transition-colors hover:border-accent-solid"
            :class="props.compact ? 'text-sm' : 'text-base'"
          >
            {{ linkLabel }}
            <UiIcon name="arrow-up-right" :size="props.compact ? '0.9rem' : '1.05rem'" />
            <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
          </a>
        </div>
      </div>
    </div>
  </article>
</template>

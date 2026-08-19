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
 * `--color-scrim` existe précisément pour ce cas. La direction artistique
 * interdit les dégradés ; le voile est donc uniforme, et son opacité suit celle
 * du site institutionnel de l'IFDD sur ses bandeaux d'ouverture.
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
    class="relative isolate flex h-full w-full flex-col justify-center overflow-hidden bg-surface-inverse text-text-on-inverse"
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

    <!-- LE VOILE. Jeton `--color-scrim`, jamais `bg-black/50`, jamais de
         dégradé : c'est ce que la direction artistique proscrit et ce que ce
         rôle existe pour remplacer. -->
    <!-- ALLÉGÉ À 38 % LE 19/08. Le voile était à 65 % quand le texte se posait à
         nu sur la photographie : il portait alors seul tout le contraste, et
         noyait l'image. Depuis que la citation vit dans un panneau de verre qui
         apporte son propre fond, il n'a plus qu'un rôle — empêcher qu'une
         photographie très claire ne troue la page et n'affaiblisse les
         commandes du rail. La photographie redevient lisible en tant que
         photographie, ce que la plateforme de référence obtenait sans voile
         général du tout. -->
    <div class="absolute inset-0 -z-10 bg-scrim/38" aria-hidden="true" />

    <!-- TROIS BOÎTES, ET CHACUNE NE FAIT QU'UNE CHOSE : celle du dehors reçoit
         les classes de l'appelant, celle du milieu centre et espace, celle du
         dedans borne la ligne de lecture. Empiler ces rôles sur un seul élément
         obligerait l'appelant à deviner comment ses classes s'arbitrent avec
         celles du composant — et Tailwind arbitre par ordre de génération, pas
         par ordre d'écriture. -->
    <div class="w-full" :class="props.contentClass">
      <div
        class="mx-auto w-full max-w-[1280px]"
        :class="props.compact ? 'px-4 py-5 sm:px-6 sm:py-6' : 'px-4 py-8 sm:px-6 sm:py-10'"
      >
        <!-- L'ENCART DE CITATION EST UN PANNEAU DE VERRE, posé sur la
             photographie — le rendu de la plateforme de référence, arbitré par
             le commanditaire le 19/08.

             LA MATIÈRE VIENT DES JETONS, PAS D'UN `bg-white/20` ÉCRIT ICI :
             `--color-glass-accent` pour la teinte institutionnelle,
             `--color-glass-border` pour le trait, `--blur-glass-strong` pour le
             flou — c'est le seul endroit qui mérite le flou fort, parce que
             c'est le seul qui porte un long texte.

             LE VOILE RESTE INDISPENSABLE en dessous : le verre sépare, il ne
             contraste pas. Sur une photographie claire et sans voile, ce panneau
             deviendrait illisible malgré son flou. -->
        <div
          class="rounded-lg border border-glass-border bg-glass-accent shadow-glass backdrop-blur-glass-strong"
          :class="props.compact ? 'p-4' : 'p-5 sm:p-7'"
          :style="{ maxWidth: 'var(--measure)' }"
        >
          <HomeNatureBadge
            :label="props.slide.nature_label"
            :color="props.slide.nature_color"
            tone="inverse"
            :size="props.compact ? 'sm' : 'md'"
          />

          <!-- Le titre passe en surtitre quand la citation prend le devant : il
               reste lisible, sans disputer la place au texte choisi par
               l'éditeur. -->
          <p
            v-if="hasQuote"
            class="mt-4 font-medium text-text-on-inverse-muted"
            :class="props.compact ? 'text-sm' : 'text-base'"
          >
            {{ title }}
          </p>

          <!-- `line-clamp` EST UN GARDE-FOU, PAS UNE MISE EN PAGE. Le bandeau a
               une hauteur bornée (`min(84vh, 720px)`) et rogne ce qui dépasse :
               une citation de dix lignes s'y trouverait coupée EN HAUT ET EN
               BAS, c'est-à-dire illisible. Cinq lignes tiennent dans tous les
               cas, et l'éditeur voit le même rendu dans son aperçu. -->
          <p
            class="font-display font-bold text-balance text-text-on-inverse"
            :class="[
              hasQuote ? 'mt-2' : 'mt-4',
              props.compact ? 'line-clamp-4 text-display-sm' : `line-clamp-6 ${headlineSize}`,
            ]"
            :style="{ lineHeight: 'var(--leading-tight)', letterSpacing: 'var(--tracking-title)' }"
          >
            {{ headline }}
          </p>

          <!-- L'ATTRIBUTION. Un témoignage sans son auteur n'engage personne ;
               c'est elle qui distingue une citation d'une accroche. -->
          <p
            v-if="props.slide.author_name || affiliation"
            class="mt-5 text-text-on-inverse"
            :class="props.compact ? 'text-sm' : 'text-base'"
          >
            <span v-if="props.slide.author_name" class="font-bold">
              — {{ props.slide.author_name }}
            </span>
            <span v-if="props.slide.author_title" class="text-text-on-inverse-muted">
              <template v-if="props.slide.author_name">, </template>
              {{ tr(props.slide.author_title) }}
            </span>
            <span v-if="affiliation" class="block text-text-on-inverse-muted">
              {{ affiliation }}
            </span>
          </p>

          <!-- LE RATTACHEMENT — édition, et séance quand il y en a une. Toute
               heure porte son fuseau, celui de la séance et non celui du
               visiteur. -->
          <p
            v-if="props.slide.event_title || props.slide.session_title"
            class="mt-4 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-text-on-inverse-muted"
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

          <!-- PAS DE PASTILLES THÉMATIQUES ICI, et ce n'est pas un oubli.
               `UiThemeTag` peint son fond avec la couleur de la base à 12 % et
               garde `--color-text` pour le libellé : sur une surface claire
               c'est exactement ce qu'il faut, sur une photographie voilée le
               texte devient illisible. Les refaire en version claire dupliquerait
               la règle des trois, qui appartient à `UiThemeTagList` — et la
               maquette ne les demande pas au bandeau. Elles restent où elles
               informent : sur les cartes d'édition, en fond de page. -->
          <!-- UN LIEN, PAS UN BOUTON. Règle d'usage n° 1 du guide de style :
               « un bouton engage, un lien déplace », et « En savoir plus » est
               toujours un lien. Le bandeau ne demande rien à la personne, il
               l'envoie voir ailleurs — le plus souvent hors de la plateforme,
               d'où la mention de nouvel onglet, la même que porte l'épingle du
               panneau latéral. Le dessin reprend celui de `AsidePin`, en plus
               grand : deux liens issus de la même donnée (`link_url`) ne
               peuvent pas se comporter différemment sur le même écran. -->
          <div v-if="props.slide.link_url" class="mt-6">
            <a
              :href="props.slide.link_url"
              target="_blank"
              rel="noopener noreferrer"
              class="inline-flex min-h-(--target-min) items-center gap-2 border-b-(length:--border-medium) border-text-on-inverse font-medium text-text-on-inverse no-underline transition-colors hover:border-accent-solid"
              :class="props.compact ? 'text-sm' : 'text-base'"
            >
              {{ linkLabel }}
              <UiIcon name="arrow-up-right" :size="props.compact ? '0.9rem' : '1.05rem'" />
              <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
            </a>
          </div>
        </div>
      </div>
    </div>
  </article>
</template>

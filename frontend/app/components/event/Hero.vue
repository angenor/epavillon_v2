<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type { AttachedImage, EditionImageRole } from '~/types/media'

/**
 * EN-TÊTE DE LA PAGE PUBLIQUE D'UNE ÉDITION — la première question de la page :
 * DE QUOI S'AGIT-IL ? Et, depuis le 19/08, la seule qui compte autant :
 * QUE PUIS-JE FAIRE ?
 *
 * ── CE QUI EST DANS LE BANDEAU, ET CE QUI N'Y EST PLUS ──────────────────────
 *
 * Y RESTE, parce que cela IDENTIFIE l'édition : la série, l'état, le titre, et
 * les quatre faits — dates, lieu, mode, pavillon.
 *
 * Y ENTRE, parce que c'est la raison de la visite : l'action de dépôt, avec le
 * rebours et l'échéance (`EventHeroCall`). Elle vivait à mi-page, dans l'encart
 * d'appel : il fallait défiler pour la trouver, et sur un portable le premier
 * écran ne montrait que le titre de la conférence.
 *
 * N'Y EST PLUS : la présentation de l'édition, sortie le 19/08. Un paragraphe de
 * soixante-cinq caractères de large sur une photographie reste pénible à lire
 * même voilé, et posé sous le bandeau il repoussait les échéances de deux cents
 * pixels. Elle vit dans `EventPresentation`.
 *
 * ── LE VISUEL PASSE DERRIÈRE LE TEXTE ───────────────────────────────────────
 *
 * L'objection tenait — un titre sur photographie perd son contraste dès que
 * l'image change, et elle change à chaque édition. Elle est TRAITÉE, par le même
 * voile en deux temps que l'affiche de l'accueil :
 *   · un APLAT `bg-scrim/45` sur toute la surface, pour le surtitre et la
 *     pastille d'état, qui se posent haut et qu'aucun fondu n'atteint ;
 *   · le FONDU `.scrim-fade-bottom` sur les quatre cinquièmes bas, pour le titre,
 *     les tuiles et le panneau d'action.
 * Le contraste ne dépend donc plus de la photographie, ce qui était tout
 * l'argument. Le verre, lui, ne contraste pas : il SÉPARE — c'est la règle du
 * § « Le verre » du guide de style.
 *
 * ── LE 32:9, ET NON LE 16:9 ─────────────────────────────────────────────────
 *
 * Une édition porte trois recadrages téléversés à la main. Ce bandeau occupe
 * toute la largeur de la fenêtre : c'est la forme du rôle `banner`. Le 16:9 sert
 * de repli — recadré, il vaut mieux qu'un en-tête nu — mais `thumbnail` n'entre
 * PAS dans ce repli : un carré étiré sur 1 500 px ne montre plus rien.
 *
 * ── SANS VISUEL, L'EN-TÊTE SOBRE ────────────────────────────────────────────
 *
 * Même composition, autre matière : pas de verre, pas de voile, des surfaces de
 * page. Poser le même bandeau sur un aplat serait de l'ornement — le voile et le
 * fondu ne séparent rien d'une surface unie. C'est le cas de la COP29 dans les
 * données simulées, et il est gardé exprès.
 *
 * ── LE BANDEAU NE PREND PAS L'ÉCRAN ─────────────────────────────────────────
 *
 * Sa hauteur est bornée par le contenu, jamais par le ratio de l'image : un 32:9
 * pleine fenêtre mesurerait 420 px et pousserait tout le reste hors de vue.
 */

interface Props {
  edition: EventEdition
  /** Série de rattachement, quand l'édition en a une : « COP Climat (CCNUCC) ». */
  series?: EventSeries | null
  /** Les trois déclinaisons, telles que `v_public_editions` les rend. */
  images?: Partial<Record<EditionImageRole, AttachedImage | null>> | null
  /** Nom du pays hôte, déjà résolu depuis `reference.countries`. */
  country?: string | null
  /**
   * Le bandeau réserve-t-il sa colonne de droite à l'action ? L'appelant seul
   * sait s'il y a un appel : sans lui, le titre reprend toute la largeur plutôt
   * que de laisser une colonne vide.
   */
  hasAction?: boolean
}

const props = defineProps<Props>()

/**
 * Le bandeau ne connaît pas l'appel à propositions : il expose l'emplacement et
 * la MATIÈRE qui convient à son fond — du verre sur la photographie, une surface
 * de page sur l'en-tête sobre. C'est l'appelant qui décide de ce qu'il y pose.
 */
defineSlots<{
  action?: (props: { tone: 'glass' | 'surface' }) => unknown
}>()

const { t } = useI18n()
const { tr } = useI18nText()

/** Le 32:9 d'abord, le 16:9 à défaut. Jamais le carré — cf. l'en-tête. */
const poster = computed(() => props.images?.banner ?? props.images?.cover ?? null)

/**
 * L'état de l'édition, dit avec les couleurs des états et non celles des
 * humeurs : le cyan informe, le jaune demande attention (« en cours »), le gris
 * clôt. Une édition annulée est un échec de calendrier, donc rouge.
 */
const STATUS_INTENT: Record<EventEdition['status'], 'info' | 'warning' | 'neutral' | 'danger'> = {
  draft: 'neutral',
  announced: 'info',
  ongoing: 'warning',
  completed: 'neutral',
  cancelled: 'danger',
  suspended: 'danger',
}

/** Colonne du titre : sept douzièmes quand l'action l'accompagne, sinon tout. */
const titleColumnClass = computed(() =>
  props.hasAction ? 'lg:col-span-7 xl:col-span-8' : 'lg:col-span-12',
)
const actionColumnClass = 'lg:col-span-5 xl:col-span-4'
</script>

<template>
  <header>
    <!-- AVEC VISUEL : le bandeau, débordant la gouttière de page.
         `-mt-8 sm:-mt-10` annule le rembourrage haut du `<main>` : un bandeau
         qui commence à quatre-vingts pixels du menu n'est pas un bandeau. -->
    <div
      v-if="poster"
      class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse text-text-on-inverse sm:-mt-10"
    >
      <!-- Chargement IMMÉDIAT : l'image est au-dessus de la ligne de flottaison,
           la différer ferait sauter la mise en page à l'ouverture. -->
      <UiImage
        :image="poster"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0"
        loading="eager"
        sizes="100vw"
      />
      <div class="pointer-events-none absolute inset-0 bg-scrim/45" aria-hidden="true" />
      <div
        class="scrim-fade-bottom pointer-events-none absolute inset-x-0 bottom-0 h-4/5"
        aria-hidden="true"
      />

      <!-- Le texte reprend la gouttière du reste de la page : le bandeau déborde,
           son contenu s'aligne. -->
      <div
        class="relative mx-auto w-full max-w-[1280px] px-4 pt-16 pb-10 sm:px-6 sm:pt-24 sm:pb-14 lg:pt-28"
      >
        <div class="grid items-end gap-8 lg:grid-cols-12 lg:gap-10">
          <div :class="titleColumnClass">
            <p class="flex flex-wrap items-center gap-2 text-sm text-text-on-inverse">
              <span
                v-if="props.series"
                class="uppercase"
                :style="{ letterSpacing: 'var(--tracking-caps)' }"
              >
                {{ tr(props.series.name) }}
              </span>
              <!-- LA PASTILLE GARDE SON DESSIN CLAIR : son fond est OPAQUE, donc
                   lisible sur n'importe quelle photographie, et son code de
                   couleur est le même sur toute la plateforme. -->
              <UiBadge :intent="STATUS_INTENT[props.edition.status]" size="sm">
                {{ t(`event.public.hero.status.${props.edition.status}`) }}
              </UiBadge>
            </p>

            <h1
              class="mt-3 font-display text-3xl leading-tight text-balance text-text-on-inverse sm:text-4xl lg:text-display"
            >
              {{ tr(props.edition.title) }}
            </h1>

            <!-- LES TUILES SORTENT DU BANDEAU SOUS `sm`, et c'est une mesure :
                 à 375 px, un titre de COP tient cinq lignes et le bandeau
                 dépassait la hauteur de l'écran — l'action passait sous la ligne
                 de flottaison, alors qu'elle est ce qu'on vient chercher. Le
                 voile porte donc le titre et l'action seuls, et les faits
                 reprennent sur la surface de page, où ils se lisent mieux. -->
            <EventHeroFacts
              class="mt-6 hidden max-w-3xl sm:grid"
              :edition="props.edition"
              :country="props.country"
              tone="glass"
            />
          </div>

          <div v-if="props.hasAction" :class="actionColumnClass">
            <slot name="action" tone="glass" />
          </div>
        </div>
      </div>
    </div>

    <!-- SANS VISUEL : l'en-tête sobre, sur la surface de page. -->
    <div v-else class="grid items-end gap-8 lg:grid-cols-12 lg:gap-10">
      <div class="min-w-0" :class="titleColumnClass">
        <p class="flex flex-wrap items-center gap-2 text-sm text-text-subtle">
          <span
            v-if="props.series"
            class="uppercase"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ tr(props.series.name) }}
          </span>
          <UiBadge :intent="STATUS_INTENT[props.edition.status]" size="sm">
            {{ t(`event.public.hero.status.${props.edition.status}`) }}
          </UiBadge>
        </p>

        <h1 class="mt-3 font-display text-3xl leading-tight text-balance sm:text-4xl lg:text-display">
          {{ tr(props.edition.title) }}
        </h1>

        <EventHeroFacts
          class="mt-6 max-w-3xl"
          :edition="props.edition"
          :country="props.country"
        />
      </div>

      <div v-if="props.hasAction" :class="actionColumnClass">
        <slot name="action" tone="surface" />
      </div>
    </div>

    <!-- Les mêmes tuiles, sur la surface de page, quand le bandeau les a laissées
         partir. `sm:hidden` : jamais les deux à la fois. -->
    <EventHeroFacts
      v-if="poster"
      class="mt-8 max-w-3xl sm:hidden"
      :edition="props.edition"
      :country="props.country"
    />
  </header>
</template>

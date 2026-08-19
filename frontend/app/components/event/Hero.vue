<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type { AttachedImage, EditionImageRole } from '~/types/media'

/**
 * EN-TÊTE DE LA PAGE PUBLIQUE D'UNE ÉDITION — la première des quatre questions
 * auxquelles cette page répond : DE QUOI S'AGIT-IL ?
 *
 * Titre, dates, lieu, mode de participation, visuel. Rien d'autre : les
 * échéances viennent juste après, et le programme plus bas. Un en-tête qui
 * répond à tout ne répond à rien.
 *
 * ── LE VISUEL PASSE DERRIÈRE LE TEXTE, ET C'EST UN REVIREMENT (19/08) ───────
 *
 * Ce composant posait l'image À CÔTÉ du titre, pour une raison qui tenait : un
 * titre sur photographie perd son contraste dès que l'image change, et elle
 * change à chaque édition. Le commanditaire a tranché autrement.
 *
 * L'objection était juste, elle n'est pas ignorée — elle est TRAITÉE, par le
 * même voile en deux temps que l'affiche de l'accueil et le bandeau du
 * back-office :
 *   · un APLAT `bg-scrim/40` sur toute la surface, pour le surtitre et la
 *     pastille d'état, qui se posent haut et qu'aucun fondu n'atteint ;
 *   · le FONDU `.scrim-fade-bottom` sur les trois quarts bas, pour le titre et
 *     les faits.
 * Le contraste ne dépend donc plus de la photographie, ce qui était tout
 * l'argument.
 *
 * ── LE 32:9, ET NON LE 16:9 ─────────────────────────────────────────────────
 *
 * Une édition porte trois recadrages téléversés à la main. Ce bandeau occupe
 * toute la largeur de la fenêtre : c'est la forme du rôle `banner`. Le 16:9
 * sert de repli — recadré, il vaut mieux qu'un en-tête nu — mais `thumbnail`
 * n'entre PAS dans ce repli : un carré étiré sur 1 500 px ne montre plus rien,
 * et l'en-tête sobre lui est alors préférable.
 *
 * ── LA PRÉSENTATION A QUITTÉ CET EN-TÊTE ────────────────────────────────────
 *
 * Elle n'est pas seulement sortie du bandeau — un paragraphe de soixante-cinq
 * caractères de large sur une photographie reste pénible à lire même voilé —
 * elle a quitté le composant. Posée sous le bandeau, elle repoussait la frise
 * des échéances de deux cents pixels, alors que c'est la frise qui répond à la
 * question pour laquelle on vient. Elle vit désormais dans
 * `EventPresentation`, après l'encart d'appel.
 *
 * L'en-tête ne porte donc plus que ce qui IDENTIFIE l'édition : série, état,
 * titre, et les quatre faits.
 *
 * ── SANS VISUEL, L'EN-TÊTE SOBRE ────────────────────────────────────────────
 *
 * Et surtout PAS le même bandeau sur un aplat : le voile et le fondu ne
 * séparent rien d'une surface unie, ils ne seraient plus que de l'ornement.
 * C'est le cas de la COP29 dans les données simulées, et il est gardé exprès.
 *
 * ── LE BANDEAU NE PREND PAS L'ÉCRAN ─────────────────────────────────────────
 *
 * Sa hauteur est bornée par le contenu, jamais par le ratio de l'image : un
 * 32:9 pleine fenêtre mesurerait 420 px et pousserait la frise des échéances
 * hors de vue. Or une organisation vient ici pour savoir si elle peut encore
 * déposer — ce qui retarde cette réponse la fait partir.
 *
 * ── TOUTE DATE PORTE SON FUSEAU ─────────────────────────────────────────────
 *
 * Celui de l'édition, jamais celui du visiteur. Les faits eux-mêmes vivent dans
 * `EventHeroFacts`, partagé par les deux rendus : écrits deux fois, ils
 * auraient divergé là où la divergence se voit le moins — un seul des deux
 * rendus est à l'écran à la fois.
 */

interface Props {
  edition: EventEdition
  /** Série de rattachement, quand l'édition en a une : « COP Climat (CCNUCC) ». */
  series?: EventSeries | null
  /** Les trois déclinaisons, telles que `v_public_editions` les rend. */
  images?: Partial<Record<EditionImageRole, AttachedImage | null>> | null
  /** Nom du pays hôte, déjà résolu depuis `reference.countries`. */
  country?: string | null
}

const props = defineProps<Props>()

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
      <div class="pointer-events-none absolute inset-0 bg-scrim/40" aria-hidden="true" />
      <div
        class="scrim-fade-bottom pointer-events-none absolute inset-x-0 bottom-0 h-3/4"
        aria-hidden="true"
      />

      <!-- Le texte reprend la gouttière du reste de la page : le bandeau déborde,
           son contenu s'aligne. -->
      <div
        class="relative mx-auto flex min-h-64 w-full max-w-[1280px] flex-col justify-end px-4 pt-14 pb-8 sm:min-h-96 sm:px-6 sm:pt-20 sm:pb-10 lg:min-h-[26rem]"
      >
        <p class="flex flex-wrap items-center gap-2 text-sm text-text-on-inverse">
          <span
            v-if="props.series"
            class="uppercase"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ tr(props.series.name) }}
          </span>
          <!-- LA PASTILLE GARDE SON DESSIN CLAIR : son fond est OPAQUE, donc
               lisible sur n'importe quelle photographie, et son code de couleur
               est le même sur toute la plateforme. -->
          <UiBadge :intent="STATUS_INTENT[props.edition.status]" size="sm">
            {{ t(`event.public.hero.status.${props.edition.status}`) }}
          </UiBadge>
        </p>

        <h1 class="mt-3 font-display text-2xl leading-tight text-text-on-inverse text-balance sm:text-4xl lg:text-5xl">
          {{ tr(props.edition.title) }}
        </h1>

        <!-- LES FAITS SORTENT DU BANDEAU SOUS `sm`, et c'est une mesure, pas
             une préférence : à 375 px, un titre de COP tient cinq lignes et le
             bandeau atteignait 673 px sur un écran de 812 — la frise des
             échéances passait sous la ligne de flottaison, alors que c'est
             exactement ce qu'une organisation vient chercher. Le voile porte
             donc le titre seul, et les faits reprennent sur la surface de page,
             où ils se lisent mieux de toute façon. -->
        <EventHeroFacts
          class="mt-6 hidden max-w-3xl sm:grid"
          :edition="props.edition"
          :country="props.country"
          tone="inverse"
        />
      </div>
    </div>

    <!-- SANS VISUEL : l'en-tête sobre, sur la surface de page. -->
    <div v-else class="min-w-0">
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

      <h1 class="mt-3 font-display text-3xl leading-tight text-balance sm:text-4xl">
        {{ tr(props.edition.title) }}
      </h1>

      <EventHeroFacts class="mt-6 max-w-3xl" :edition="props.edition" :country="props.country" />
    </div>

    <!-- Les mêmes faits, sur la surface de page, quand le bandeau les a laissés
         partir. `sm:hidden` : jamais les deux à la fois. -->
    <EventHeroFacts
      v-if="poster"
      class="mt-8 max-w-3xl sm:hidden"
      :edition="props.edition"
      :country="props.country"
    />

  </header>
</template>

<script setup lang="ts">
import type { ApexOptions } from 'apexcharts'
import type { ChartSeries } from '~/composables/useChartTheme'

/**
 * ENVELOPPE DE GRAPHIQUE — le seul endroit de la plateforme qui parle à
 * ApexCharts.
 *
 * TROIS RESPONSABILITÉS, et chacune corrige un défaut autrement inévitable :
 *
 *  1. LE RENDU EST CÔTÉ NAVIGATEUR. Un graphique mesure son conteneur : il n'y a
 *     rien à en tirer au rendu serveur. `<ClientOnly>` affiche donc un squelette
 *     DE LA MÊME HAUTEUR pendant l'hydratation — une hauteur différente ferait
 *     sauter la page au moment où le tracé apparaît, ce que les squelettes
 *     existent précisément pour éviter.
 *  2. IL ATTEND LA PALETTE. Les couleurs sont lues dans les jetons de design au
 *     moment où le DOM les porte (voir `useChartTheme`) : dessiner avant, c'est
 *     dessiner en transparent.
 *  3. IL PORTE SON TEXTE DE REMPLACEMENT. Un graphique est une image : sans
 *     résumé, il ne dit rien à qui ne le voit pas. Le SVG lui-même est masqué aux
 *     technologies d'assistance — laissé visible, il leur fait énumérer les
 *     libellés d'axes dans le désordre.
 *
 * UN GRAPHIQUE DÉCORATIF NE PORTE PAS DE RÉSUMÉ, et c'est délibéré : une
 * étincelle posée sous un chiffre déjà écrit en toutes lettres, avec sa variation,
 * n'apprend rien de plus à un lecteur d'écran — elle ne fait que répéter la carte.
 */

interface Props {
  type: 'bar' | 'line' | 'area' | 'donut' | 'radialBar' | 'treemap'
  series: ChartSeries[] | number[]
  options: ApexOptions
  /** Hauteur en pixels — la même pour le squelette et pour le tracé. */
  height: number
  /**
   * Ce qu'un œil retire du graphique, en une phrase. Lu à sa place.
   * OBLIGATOIRE, sauf pour un tracé `decorative`.
   */
  summary?: string
  /** Étincelle ou fond : le sens est déjà écrit à côté, en texte. */
  decorative?: boolean
}

const props = defineProps<Props>()

const { palette, isDark } = useChartTheme()
const { locale } = useI18n()

/**
 * LE TRACÉ EST REMONTÉ QUAND LE THÈME OU LA LANGUE CHANGE, il n'est pas mis à
 * jour.
 *
 * ApexCharts fusionne les options qu'on lui repasse, et cette fusion PERD LES
 * FONCTIONS : au premier changement de thème, les formats d'axe disparaissaient —
 * « 0, 1, 2, 3 » devenait « 0.00, 1.00, 2.00, 3.00 » — et la couleur de la
 * seconde série restait celle du thème précédent, donc blanche sur fond blanc.
 * Mesuré en basculant le thème sans recharger la page.
 *
 * Une clé qui change force un composant neuf, avec ses options complètes. Elle ne
 * porte QUE le thème et la langue : les données, elles, se mettent à jour
 * proprement, et remonter le graphique à chaque nouvelle série coûterait une
 * animation à chaque rafraîchissement.
 */
const redrawKey = computed(() => `${isDark.value ? 'dark' : 'light'}-${locale.value}`)

</script>

<template>
  <!-- LA HAUTEUR EST RÉSERVÉE PAR LE CONTENEUR, et pas seulement par le squelette :
       la bibliothèque arrive par un paquet séparé, chargé au premier graphique de
       la session. Entre le squelette et le tracé, il y a donc un instant où le
       conteneur est vide — sans hauteur réservée, la page sauterait là. -->
  <div
    class="min-w-0"
    :style="{ minHeight: `${props.height}px` }"
    :role="props.decorative ? undefined : 'img'"
    :aria-label="props.decorative ? undefined : props.summary"
    :aria-hidden="props.decorative ? true : undefined"
  >
    <ClientOnly>
      <apexchart
        v-if="palette"
        :key="redrawKey"
        :type="props.type"
        :series="props.series"
        :options="props.options"
        :height="props.height"
        width="100%"
      />
      <UiSkeletonLoader v-else :height="`${props.height}px`" rounded="var(--radius-md)" />

      <template #fallback>
        <UiSkeletonLoader :height="`${props.height}px`" rounded="var(--radius-md)" />
      </template>
    </ClientOnly>
  </div>
</template>

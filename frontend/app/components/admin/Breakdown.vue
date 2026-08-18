<script setup lang="ts">
import type { ApexFormatterOpts, ApexOptions } from 'apexcharts'
import type { BreakdownSlice } from '~/types/admin-dashboard'
import type { ChartSeries, ChartTone } from '~/composables/useChartTheme'
import { contrastRatio } from '~/utils/contrast'

/**
 * RÉPARTITION — pays d'origine, thématiques.
 *
 * DES BARRES HORIZONTALES, ET NON UN CAMEMBERT. Comparer deux angles est une
 * tâche de perception coûteuse ; comparer deux longueurs alignées sur un même
 * bord ne l'est pas. Sur dix parts dont les trois premières écrasent le reste, un
 * camembert devient une roue de fines lamelles sans libellé lisible. La bascule
 * vers une bibliothèque de graphiques n'a rien changé à ce choix — elle a
 * seulement donné à ces barres un axe, des infobulles et un dessin cohérent avec
 * les courbes voisines.
 *
 * LA COULEUR VIENT DE LA BASE, quand il y en a une. Les thématiques portent leur
 * `color_hex` dans `reference.taxonomy_terms`, modifiable au back-office : c'est
 * elle qui s'affiche, et rien d'autre. Les figer dans la feuille de style est le
 * défaut n° 1 de la v1. Les répartitions sans couleur propre — les pays —
 * prennent l'accent : une seule teinte pour une seule série.
 *
 * D'OÙ LE CALCUL DE CONTRASTE. Le chiffre est écrit DANS la barre, et la teinte
 * de la barre est une donnée : on ne sait pas, en écrivant ce composant, si elle
 * sera claire ou sombre. L'encre est donc choisie par mesure — le meilleur des
 * deux jetons de texte face à cette teinte-là — plutôt que fixée à blanc, qui
 * disparaîtrait le jour où un administrateur choisit un jaune pâle.
 *
 * LA PART SE LIT SUR LE TOTAL DES DOSSIERS, pas sur le total des rattachements.
 * Un dossier porte plusieurs thématiques : la somme des parts dépasse 100 %, et
 * c'est exact. « 40 % des dossiers touchent à l'adaptation » se comprend ;
 * « l'adaptation représente 18 % des rattachements » ne veut rien dire.
 */

interface Props {
  title: string
  slices: BreakdownSlice[]
  /** Précision affichée sous le titre — « la somme dépasse le nombre de dossiers ». */
  note?: string
  /** Les parts sans couleur propre prennent celle-ci. */
  fallbackTone?: 'accent' | 'postponed'
}

const props = withDefaults(defineProps<Props>(), { fallbackTone: 'accent' })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { palette, baseOptions, toneColor, fontFamily } = useChartTheme()

/** Une barre par part, sans jamais descendre sous une hauteur lisible. */
const ROW_HEIGHT = 30
const CHART_PADDING = 16

const height = computed(() => Math.max(props.slices.length * ROW_HEIGHT + CHART_PADDING, 96))

const fallback = computed<ChartTone>(() => (props.fallbackTone === 'accent' ? 'accent' : 'postponed'))

const colors = computed(() =>
  props.slices.map((slice) => slice.color ?? toneColor(fallback.value)),
)

function share(slice: BreakdownSlice): string {
  return t('common.formats.percent', {
    value: new Intl.NumberFormat(locale.value).format(Math.round(slice.share * 100)),
  })
}

const series = computed<ChartSeries[]>(() => [
  {
    name: props.title,
    data: props.slices.map((slice) => ({ x: tr(slice.label), y: slice.count })),
  },
])

/** Le meilleur des deux jetons d'encre face à cette teinte — voir l'en-tête. */
function inkOn(background: string): string {
  const ink = palette.value?.text ?? ''
  const inverse = palette.value?.textInverse ?? ''
  const onInk = contrastRatio(ink, background) ?? 0
  const onInverse = contrastRatio(inverse, background) ?? 0
  return onInk >= onInverse ? ink : inverse
}

const maxCount = computed(() => Math.max(...props.slices.map((slice) => slice.count), 1))

const options = computed<ApexOptions>(() => {
  const base = baseOptions()

  return {
    ...base,
    chart: { ...base.chart, type: 'bar' },
    colors: colors.value,
    plotOptions: {
      bar: {
        horizontal: true,
        distributed: true,
        barHeight: '68%',
        borderRadius: 3,
        borderRadiusApplication: 'end',
        dataLabels: { position: 'center' },
      },
    },
    /*
     * LE CHIFFRE EST ÉCRIT DANS LA BARRE, et la part avec lui QUAND LA BARRE EST
     * ASSEZ LONGUE. Sur une barre courte, « 1 · 3 % » déborde et se superpose à la
     * suivante : seul le décompte y tient, et la part reste dans l'infobulle.
     */
    dataLabels: {
      enabled: true,
      style: { fontSize: '11px', fontWeight: 600, fontFamily: fontFamily.value, colors: colors.value.map(inkOn) },
      formatter: (_value: string | number | number[], opts?: ApexFormatterOpts) => {
        const slice = props.slices[opts?.dataPointIndex ?? -1]
        if (!slice) return ''
        const wide = slice.count / maxCount.value > 0.22
        return wide ? `${slice.count} · ${share(slice)}` : String(slice.count)
      },
    },
    /*
     * MARGE À GAUCHE — 12 px, et ce n'est pas décoratif. La bibliothèque réserve
     * pour les libellés d'axe la largeur du texte TRONQUÉ, puis lui ajoute ses
     * points de suspension au moment de l'écrire : le libellé le plus long
     * dépassait du cadre par la gauche, où il se faisait couper. La marge rend ce
     * dépassement inoffensif.
     */
    grid: { ...base.grid, show: false, padding: { top: -8, right: 0, bottom: -8, left: 12 } },
    xaxis: {
      labels: { show: false },
      axisBorder: { show: false },
      axisTicks: { show: false },
    },
    yaxis: {
      labels: {
        /*
         * LARGEUR MAXIMALE VOLONTAIREMENT BASSE — 120 px et non 150.
         *
         * La bibliothèque mesure la largeur d'un libellé avant de connaître notre
         * police, puis rend le texte dans la nôtre, un peu plus large : elle
         * réservait 133 px pour un libellé qui en occupe 140, et « Agriculture et
         * alimentation » se faisait couper au COMMENCEMENT par le bord du cadre —
         * « griculture et alimentation ». Sous le plafond, elle tronque
         * elle-même, PAR LA FIN et avec ses points de suspension, et le libellé
         * entier reste dans l'infobulle. Mesuré sur la répartition par thématique.
         */
        trim: true,
        maxWidth: 120,
        style: { fontSize: '12px', fontFamily: fontFamily.value, colors: palette.value?.textMuted },
      },
    },
    tooltip: {
      ...base.tooltip,
      y: {
        formatter: (value: number, opts?: ApexFormatterOpts) => {
          const slice = props.slices[opts?.dataPointIndex ?? -1]
          return slice ? `${value} · ${share(slice)}` : String(value)
        },
      },
    },
  }
})

/** Ce qu'un œil retire de la répartition : les trois premières parts. */
const summary = computed(() =>
  t('admin.dashboard.breakdown.summary', {
    title: props.title,
    count: props.slices.length,
    top: props.slices
      .slice(0, 3)
      .map((slice) => `${tr(slice.label)} ${share(slice)}`)
      .join(', '),
  }),
)
</script>

<template>
  <section class="min-w-0">
    <h3 class="text-base font-semibold text-text">{{ props.title }}</h3>
    <p v-if="props.note" class="mt-0.5 text-xs text-text-subtle">{{ props.note }}</p>

    <p
      v-if="props.slices.length === 0"
      class="mt-3 rounded-md border border-border-subtle bg-surface-sunken px-4 py-6 text-center text-sm text-text-muted"
    >
      {{ t('admin.dashboard.charts.noData') }}
    </p>

    <UiChart
      v-else
      class="mt-2"
      type="bar"
      :series="series"
      :options="options"
      :height="height"
      :summary="summary"
    />
  </section>
</template>

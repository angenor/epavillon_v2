<script setup lang="ts">
import type { ApexFormatterOpts, ApexOptions } from 'apexcharts'
import type { TrendPoint } from '~/types/admin-dashboard'
import type { IsoDateTime } from '~/types/shared'
import type { ChartSeries, ChartTone } from '~/composables/useChartTheme'

/**
 * COURBE QUOTIDIENNE — des bâtons pour les jours, une ligne pour la tendance.
 *
 * DES BÂTONS, PAS UNE COURBE, ET C'EST UN CHOIX DE VÉRITÉ. Une valeur QUOTIDIENNE
 * est un compte, pas une grandeur continue : relier « 3 dépôts mardi » à
 * « 0 mercredi » par un segment dessine une pente qui n'a jamais existé, et sur
 * une série creuse — le cas ordinaire d'un appel à propositions — le tracé
 * devient un peigne dont on ne lit plus rien. Un bâton par jour ne prétend rien
 * entre deux jours.
 *
 * LA TENDANCE EST UNE SECONDE SÉRIE, ET ELLE VIENT DE LA BASE. Les deux
 * projections quotidiennes portent `moyenne_mobile_7j` : c'est elle qui répond à
 * « est-ce que ça accélère », question à laquelle des bâtons seuls ne répondent
 * pas. Elle n'est PAS recalculée ici — une seconde moyenne, calculée autrement,
 * finirait par contredire celle qu'un export SQL rendrait. Elle est nulle sur les
 * premiers jours, où la fenêtre de sept jours n'est pas pleine, et la ligne ne
 * commence donc pas au bord du cadre : c'est exact, et c'est voulu.
 *
 * DEUX SÉRIES, DONC UNE LÉGENDE — mais écrite à la main, au-dessus du tracé, avec
 * les jetons de la charte. Celle de la bibliothèque impose ses pastilles, ses
 * tailles et ses gris ; deux formes aussi différentes qu'un bâton et une ligne se
 * nomment en deux mots, sans boîte.
 *
 * LES REPÈRES VERTICAUX ÉTENDENT L'AXE. « L'échéance marquée » n'est pas un trait
 * de plus : sans elle, l'effet de dernière minute — 60 % des dépôts sur les 48
 * dernières heures, mesuré en v1 — se lit comme un pic devant rien. Et comme
 * l'échéance est le plus souvent DEVANT la fin de la série, le cadre doit aller la
 * chercher : voir `domain`.
 *
 * LES JOURS SONT FORMATÉS EN UTC, PAS DANS LE FUSEAU DE L'ÉDITION, et c'est une
 * correction. Les projections découpent leurs journées en UTC, explicitement (un
 * agrégat ne change pas de valeur selon le fuseau qui le calcule) : `2026-06-30`
 * y est une DATE CIVILE, pas un instant. La rendre dans le fuseau de Belém la
 * reculerait d'un jour — le graphique dirait le 29 là où la base compte le 30.
 * C'est la seule vue de la plateforme où le fuseau de l'édition n'a pas cours.
 */

interface Marker {
  at: IsoDateTime
  label: string
  /** `deadline` : trait plein appuyé. `default` : trait léger, un jalon. */
  kind?: 'deadline' | 'default'
}

interface Props {
  title: string
  points: TrendPoint[]
  /** Nom de la série de bâtons, écrit dans la légende. */
  seriesLabel: string
  /** Repères verticaux datés — ouverture de l'appel, échéance. */
  markers?: Marker[]
  /** Une couleur par série. `accent` pour la première, `postponed` pour la seconde. */
  tone?: 'accent' | 'postponed'
  /** Total affiché en tête — le cumul du dernier point, déjà calculé en base. */
  totalLabel?: string
}

const props = withDefaults(defineProps<Props>(), { tone: 'accent', markers: () => [] })

const { t } = useI18n()
const { intlLocale } = useDateTime()
const { palette, baseOptions, toneColor, fontFamily } = useChartTheme()

const HEIGHT = 232
const DAY = 86_400_000

const chartTone = computed<ChartTone>(() =>
  props.tone === 'accent' ? 'accent' : 'postponed',
)

/** Minuit UTC du jour civil — les séries du modèle sont découpées en UTC. */
function dayTime(day: string): number {
  return Date.parse(`${day.slice(0, 10)}T00:00:00Z`)
}

/**
 * L'AXE DES ABSCISSES EST UN AXE DE TEMPS, PAS UN RANG DE POINTS, et les repères
 * l'étendent.
 *
 * C'est ce qui permet de marquer une échéance ENCORE À VENIR : la série des
 * dépôts s'arrête aujourd'hui — une courbe ne se projette pas dans le futur —,
 * mais l'échéance, elle, est devant. Laisser la bibliothèque cadrer sur les seules
 * données rejetterait le repère hors du tracé, où elle ne le dessinerait pas.
 */
const domain = computed(() => {
  const times = [
    ...props.points.map((point) => dayTime(point.jour)),
    ...props.markers.map((marker) => dayTime(marker.at)),
  ].filter((time) => Number.isFinite(time))

  if (times.length === 0) return { min: 0, max: DAY }
  const min = Math.min(...times)
  const max = Math.max(...times)
  // Une série d'un seul jour, sans repère : le cadre serait de largeur nulle.
  return { min: min - DAY / 2, max: Math.max(max, min + DAY) + DAY / 2 }
})

const series = computed<ChartSeries[]>(() => [
  {
    name: props.seriesLabel,
    type: 'bar',
    data: props.points.map((point) => [dayTime(point.jour), point.valeur] as [number, number]),
  },
  {
    name: t('admin.dashboard.charts.movingAverage'),
    type: 'line',
    data: props.points.map(
      (point) => [dayTime(point.jour), point.moyenne_7j] as [number, number | null],
    ),
  },
])

const dayLabel = computed(
  () =>
    new Intl.DateTimeFormat(intlLocale.value, {
      day: 'numeric',
      month: 'short',
      timeZone: 'UTC',
    }),
)

const fullDayLabel = computed(
  () => new Intl.DateTimeFormat(intlLocale.value, { dateStyle: 'long', timeZone: 'UTC' }),
)

const integer = computed(() => new Intl.NumberFormat(intlLocale.value, { maximumFractionDigits: 0 }))

/**
 * L'ÉCHELLE VERTICALE EST ENTIÈRE, ET C'EST À NOUS DE L'IMPOSER.
 *
 * On compte des dépôts : « 1,5 dépôt » n'existe pas. Laissée à
 * `forceNiceScale` avec quatre graduations, la bibliothèque découpe un maximum de
 * 3 en 0,75 — 1,5 — 2,25, que notre format arrondit à « 1, 2, 2 » : deux
 * graduations portent le même nombre et la plus haute manque. Le pas est donc
 * calculé ici pour que chaque graduation tombe sur un entier.
 *
 * Une série courte reçoit autant de graduations que d'unités (0, 1, 2, 3) ; au
 * delà de cinq, quatre graduations d'un pas entier suffisent.
 */
const scale = computed(() => {
  const peakValue = Math.max(
    1,
    ...props.points.map((point) => point.valeur),
    ...props.points.map((point) => point.moyenne_7j ?? 0),
  )
  const rounded = Math.ceil(peakValue)
  // Une graduation de respiration au-dessus du pic : cadré sur son maximum
  // exact, le bâton le plus haut touche le bord du cadre et vient buter dans le
  // libellé du repère d'échéance.
  if (rounded <= 5) return { max: rounded + 1, ticks: rounded + 1 }
  const step = Math.ceil(rounded / 4)
  return { max: step * 4, ticks: 4 }
})
const oneDecimal = computed(
  () => new Intl.NumberFormat(intlLocale.value, { maximumFractionDigits: 1 }),
)

const options = computed<ApexOptions>(() => {
  const base = baseOptions()
  const span = domain.value.max - domain.value.min

  return {
    ...base,
    chart: { ...base.chart, type: 'line', stacked: false },
    colors: [toneColor(chartTone.value), palette.value?.text ?? 'transparent'],
    // Un bâton par jour, quelle que soit la largeur du cadre : c'est la journée
    // qui donne la largeur, pas le nombre de points.
    plotOptions: { bar: { columnWidth: '62%', borderRadius: 2, borderRadiusApplication: 'end' } },
    stroke: { width: [0, 2.5], curve: 'smooth', lineCap: 'round' },
    markers: { size: 0, hover: { size: 0 } },
    xaxis: {
      type: 'datetime',
      min: domain.value.min,
      max: domain.value.max,
      tickAmount: span > 60 * DAY ? 6 : 4,
      axisBorder: { show: true, color: palette.value?.border },
      axisTicks: { show: false },
      crosshairs: { show: false },
      labels: {
        rotate: 0,
        hideOverlappingLabels: true,
        style: { fontSize: '11px', fontFamily: fontFamily.value },
        formatter: (_value: string, timestamp?: number) =>
          dayLabel.value.format(new Date(timestamp ?? Number(_value))),
      },
    },
    yaxis: {
      min: 0,
      max: scale.value.max,
      tickAmount: scale.value.ticks,
      labels: {
        style: { fontSize: '11px', fontFamily: fontFamily.value },
        formatter: (value: number) => integer.value.format(value),
      },
    },
    tooltip: {
      ...base.tooltip,
      shared: true,
      intersect: false,
      x: { formatter: (value: number) => fullDayLabel.value.format(new Date(value)) },
      y: {
        formatter: (value: number | null, opts?: ApexFormatterOpts) =>
          value === null
            ? ''
            : opts?.seriesIndex === 1
              ? oneDecimal.value.format(value)
              : integer.value.format(value),
      },
    },
    /*
     * LES REPÈRES, TRACÉS PAR LA BIBLIOTHÈQUE ET NON POSÉS PAR-DESSUS. C'est le
     * gain le plus net de la bascule : le libellé se place, se mesure et
     * s'esquive tout seul, là où deux plans superposés — un SVG étiré et du HTML
     * absolu — demandaient de recalculer un alignement à chaque largeur d'écran.
     */
    annotations: {
      xaxis: props.markers.map((marker) => {
        const deadline = marker.kind === 'deadline'
        const ratio = (dayTime(marker.at) - domain.value.min) / span
        return {
          x: dayTime(marker.at),
          borderColor: deadline
            ? (palette.value?.danger ?? 'transparent')
            : (palette.value?.border ?? 'transparent'),
          strokeDashArray: deadline ? 0 : 4,
          label: {
            text: marker.label,
            position: 'top' as const,
            orientation: 'horizontal' as const,
            offsetY: -6,
            // Un libellé centré sur un repère collé au bord déborderait du cadre.
            textAnchor: ratio > 0.88 ? ('end' as const) : ratio < 0.12 ? ('start' as const) : ('middle' as const),
            borderColor: 'transparent',
            style: {
              background: 'transparent',
              color: deadline ? palette.value?.danger : palette.value?.textSubtle,
              fontSize: '11px',
              fontWeight: 600,
              fontFamily: fontFamily.value,
            },
          },
        }
      }),
    },
  }
})

const lastPoint = computed(() => props.points.at(-1) ?? null)
const peak = computed(() =>
  props.points.reduce<TrendPoint | null>(
    (best, point) => (best === null || point.valeur > best.valeur ? point : best),
    null,
  ),
)

/**
 * Résumé lu par les lecteurs d'écran. Un graphique est une image : sans ce texte,
 * il ne dit rien. On y met ce qu'un œil en retire — la période, le total, le jour
 * le plus haut.
 */
const summary = computed(() => {
  const first = props.points[0]
  const last = props.points.at(-1)
  return t('admin.dashboard.charts.summary', {
    series: props.seriesLabel,
    from: first ? fullDayLabel.value.format(new Date(dayTime(first.jour))) : '',
    to: last ? fullDayLabel.value.format(new Date(dayTime(last.jour))) : '',
    total: lastPoint.value?.cumul ?? 0,
    peakValue: peak.value?.valeur ?? 0,
    peakDay: peak.value ? fullDayLabel.value.format(new Date(dayTime(peak.value.jour))) : '',
  })
})
</script>

<template>
  <figure class="min-w-0">
    <figcaption class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h3 class="text-base font-semibold text-text">{{ props.title }}</h3>
      <p v-if="props.totalLabel" class="text-sm tabular-nums text-text-secondary">
        {{ props.totalLabel }}
      </p>
    </figcaption>

    <div
      v-if="props.points.length === 0"
      class="mt-3 rounded-md border border-border-subtle bg-surface-sunken px-4 py-8 text-center text-sm text-text-muted"
    >
      {{ t('admin.dashboard.charts.noData') }}
    </div>

    <template v-else>
      <!-- LÉGENDE ÉCRITE À LA MAIN : deux formes, deux mots, aucun cadre. -->
      <ul class="mt-2 mb-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-text-muted">
        <li class="flex items-center gap-1.5">
          <span
            class="h-2.5 w-2 rounded-sm"
            :class="props.tone === 'accent' ? 'bg-accent-solid' : 'bg-postponed'"
          />
          {{ props.seriesLabel }}
        </li>
        <li class="flex items-center gap-1.5">
          <span class="h-0.5 w-4 rounded-full bg-text" />
          {{ t('admin.dashboard.charts.movingAverage') }}
        </li>
      </ul>

      <UiChart type="line" :series="series" :options="options" :height="HEIGHT" :summary="summary" />
    </template>
  </figure>
</template>

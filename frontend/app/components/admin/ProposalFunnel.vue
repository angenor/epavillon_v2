<script setup lang="ts">
import type { ApexOptions } from 'apexcharts'
import type { ProposalFunnelRow } from '~/types/analytics'
import type { ChartSeries } from '~/composables/useChartTheme'
import { contrastRatio } from '~/utils/contrast'

/**
 * L'ENTONNOIR DES PROPOSITIONS — `analytics.mv_proposal_funnel`.
 *
 * CINQ ÉTAGES, ET ILS SE DÉDUISENT LES UNS DES AUTRES : ouverts → déposés → en
 * évaluation → décidés → retenus. La largeur d'un étage est la part du TOTAL,
 * jamais de l'étage précédent : rapportée au précédent, chaque étage remplirait la
 * largeur et l'entonnoir ne s'amincirait jamais — ce qui est précisément ce qu'un
 * entonnoir doit montrer.
 *
 * IL NE PREND LA FORME D'UN ENTONNOIR QUE S'IL EN A LE DROIT. Les étages ne sont
 * pas monotones par construction : « en évaluation » compte ce qui attend, « décidés »
 * ce qui est tranché, et un comité qui a bien avancé rend le second plus grand que
 * le premier. Dessiné en trapèzes, cet entonnoir s'ÉLARGIRAIT au milieu — une
 * forme qui affirme le contraire des chiffres qu'elle porte. On vérifie donc que
 * la série décroît avant de demander la forme ; sinon ce sont des barres, qui
 * n'affirment rien de la suite. C'est mesuré, pas supposé : voir `monotone`.
 *
 * UNE COULEUR POUR LA SÉRIE, UNE POUR L'ISSUE. Les quatre premiers étages
 * partagent l'accent : c'est un même flux qui se rétrécit, pas quatre choses
 * différentes. Seul le dernier prend une couleur d'état — vert pour ce qui est
 * retenu —, parce que c'en est un, et que la couleur distingue des états ; elle ne
 * décore pas.
 *
 * LES SORTIES SONT DITES, PAS DEVINÉES. Brouillons jamais déposés, dossiers
 * retirés, dossiers écartés : ce sont eux qui expliquent l'écart entre deux étages.
 * Un entonnoir qui ne montre que ce qui avance laisse croire que le reste s'est
 * évaporé.
 *
 * DEUX TAUX, ET ILS NE DISENT PAS LA MÊME CHOSE — le modèle les sépare
 * délibérément : la sélectivité du comité se calcule sur les dossiers tranchés, le
 * rendement de l'appel sur tout ce qui a été déposé, retraits compris.
 */

interface Props {
  funnel: ProposalFunnelRow
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { palette, baseOptions, toneColor, fontFamily } = useChartTheme()

const STAGE_KEYS = ['opened', 'submitted', 'inReview', 'decided', 'accepted'] as const

const stages = computed(() => {
  const f = props.funnel
  const counts: Record<(typeof STAGE_KEYS)[number], number> = {
    opened: f.total,
    submitted: f.deposees,
    inReview: f.en_attente_affectation + f.en_revue + f.modifications_demandees,
    decided: f.decidees,
    accepted: f.acceptees,
  }
  return STAGE_KEYS.map((key) => ({
    key,
    label: t(`admin.dashboard.funnel.stage.${key}`),
    count: counts[key],
  }))
})

/** La forme d'entonnoir n'est légitime que sur une série décroissante — voir l'en-tête. */
const monotone = computed(() =>
  stages.value.every((stage, index, all) => index === 0 || stage.count <= (all[index - 1]?.count ?? 0)),
)

const colors = computed(() => {
  const flow = toneColor('accent')
  return [flow, flow, flow, flow, toneColor('success')]
})

function inkOn(background: string): string {
  const ink = palette.value?.text ?? ''
  const inverse = palette.value?.textInverse ?? ''
  return (contrastRatio(ink, background) ?? 0) >= (contrastRatio(inverse, background) ?? 0)
    ? ink
    : inverse
}

const series = computed<ChartSeries[]>(() => [
  {
    name: t('admin.dashboard.funnel.title'),
    data: stages.value.map((stage) => ({ x: stage.label, y: stage.count })),
  },
])

const HEIGHT = 236

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
        isFunnel: monotone.value,
        barHeight: '78%',
        borderRadius: monotone.value ? 0 : 3,
        borderRadiusApplication: 'end',
      },
    },
    dataLabels: {
      enabled: true,
      style: {
        fontSize: '12px',
        fontWeight: 700,
        fontFamily: fontFamily.value,
        colors: colors.value.map(inkOn),
      },
      // La part du total, à côté du décompte : c'est elle qui dit de combien
      // l'étage s'est rétréci, et elle est la raison d'être de la forme.
      formatter: (value: number) => {
        const total = Math.max(1, props.funnel.total)
        return `${new Intl.NumberFormat(locale.value).format(value)} · ${t('common.formats.percent', {
          value: new Intl.NumberFormat(locale.value).format(Math.round((value / total) * 100)),
        })}`
      },
    },
    grid: { ...base.grid, show: false, padding: { top: -12, right: 0, bottom: -12, left: 12 } },
    xaxis: { labels: { show: false }, axisBorder: { show: false }, axisTicks: { show: false } },
    yaxis: {
      labels: {
        trim: true,
        maxWidth: 110,
        style: { fontSize: '12px', fontFamily: fontFamily.value, colors: palette.value?.textMuted },
      },
    },
    tooltip: { ...base.tooltip, y: { formatter: (value: number) => String(value) } },
  }
})

/**
 * « 62 % » — les taux du modèle sont des RATIOS entre 0 et 1.
 *
 * Un taux nul n'est pas un zéro : `taux_acceptation` vaut `null` quand aucun
 * dossier n'a été tranché, et afficher « 0 % » ferait passer un comité qui n'a pas
 * commencé pour un comité qui a tout refusé.
 */
function percent(ratio: number | null): string {
  if (ratio === null) return t('common.labels.none')
  return t('common.formats.percent', {
    value: new Intl.NumberFormat(locale.value).format(Math.round(ratio * 100)),
  })
}

const exits = computed(() => [
  { key: 'drafts', count: props.funnel.brouillons },
  { key: 'withdrawn', count: props.funnel.retirees },
  { key: 'rejected', count: props.funnel.rejetees },
])

const summary = computed(() =>
  t('admin.dashboard.funnel.summary', {
    stages: stages.value.map((stage) => `${stage.label} ${stage.count}`).join(', '),
  }),
)
</script>

<template>
  <section aria-labelledby="admin-funnel-title">
    <h3 id="admin-funnel-title" class="text-base font-semibold text-text">
      {{ t('admin.dashboard.funnel.title') }}
    </h3>

    <UiChart
      class="mt-2"
      type="bar"
      :series="series"
      :options="options"
      :height="HEIGHT"
      :summary="summary"
    />

    <!-- CE QUI SORT DE L'ENTONNOIR, à part : c'est l'explication des écarts. -->
    <dl class="mt-4 grid grid-cols-3 gap-3 border-t border-border-subtle pt-4">
      <div v-for="exit in exits" :key="exit.key">
        <dt class="text-xs text-text-subtle">{{ t(`admin.dashboard.funnel.exit.${exit.key}`) }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">{{ exit.count }}</dd>
      </div>
    </dl>

    <dl class="mt-4 grid grid-cols-2 gap-x-4 gap-y-3 border-t border-border-subtle pt-4 sm:grid-cols-3">
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.selectivity') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ percent(props.funnel.taux_acceptation) }}
        </dd>
      </div>
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.yield') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ percent(props.funnel.taux_acceptation_sur_depots) }}
        </dd>
      </div>
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.organizations') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ props.funnel.organisations_distinctes }}
        </dd>
      </div>
    </dl>
  </section>
</template>

<script setup lang="ts">
import type { ApexOptions } from 'apexcharts'
import type { DashboardKpiTone } from '~/types/admin-dashboard'
import type { ChartTone } from '~/composables/useChartTheme'

/**
 * UN CHIFFRE DE TÊTE — libellé, valeur, ce qui la précise, et son mouvement.
 *
 * QUATRE NIVEAUX DE LECTURE, ET LEUR ORDRE EST LE SUJET. Le libellé dit de quoi
 * on parle, la valeur répond, l'unité l'ancre, la précision explique. On lit la
 * carte en un coup d'œil et on s'arrête au niveau qui suffit — c'est ce qui
 * distingue un indicateur d'une ligne de tableau.
 *
 * UN SEUL VISUEL PAR CARTE, jamais deux. Une étincelle quand le chiffre est une
 * série (des dépôts, des inscriptions : « ça monte » est l'information), une
 * jauge quand c'est un rapport (18 revues sur 24 : « il en reste 6 » est
 * l'information), rien quand c'est ni l'un ni l'autre. Empiler les deux remplit
 * la carte sans rien ajouter.
 *
 * LA VARIATION N'EST NI VERTE NI ROUGE, et c'est un choix de fond. Une hausse
 * des dépôts est une bonne nouvelle, une hausse des retards non : la même
 * couleur pour les deux apprend à l'équipe que la flèche vers le haut est
 * toujours bonne, ce qui est faux. La direction se lit sur la flèche ; la couleur
 * reste réservée aux états, comme partout ailleurs dans la charte.
 *
 * LA VALEUR ARRIVE DÉJÀ FORMATÉE. Le formatage dépend de la locale et de la
 * nature du chiffre — un compte, un pourcentage, un décompte de jours : c'est
 * l'écran qui sait, pas la carte.
 */

interface Props {
  label: string
  /** Déjà formatée par l'écran, locale comprise. */
  value: string
  /** Ce que la valeur compte : « jours restants », « sur 24 ». */
  unit?: string
  /** Précision d'une ligne : le dénominateur, la date, la définition retenue. */
  hint?: string
  /** Mouvement sur sept jours. Le libellé est écrit, la flèche le double. */
  delta?: { label: string; direction: 'up' | 'down' | 'flat' } | null
  /** Série courte de l'étincelle. Vide : pas d'étincelle. */
  spark?: number[]
  /** Jauge entre 0 et 1 pour un rapport. `null` : pas de jauge. */
  progress?: number | null
  tone?: DashboardKpiTone
  icon: string
}

const props = withDefaults(defineProps<Props>(), {
  tone: 'neutral',
  spark: () => [],
  progress: null,
})

const { baseOptions, toneColor } = useChartTheme()

/** La teinte de l'indicateur, dans le vocabulaire des graphiques. */
const chartTone = computed<ChartTone>(() => (props.tone === 'neutral' ? 'accent' : props.tone))

const TONE_ICON: Record<DashboardKpiTone, string> = {
  neutral: 'bg-surface-sunken text-text-muted',
  accent: 'bg-accent-surface text-accent',
  success: 'bg-success-surface text-success',
  warning: 'bg-warning-surface text-warning',
  danger: 'bg-danger-surface text-danger',
}

/**
 * LA JAUGE N'EST PAS COLORÉE PAR L'ÉTAT DE LA CARTE, et c'est une correction.
 *
 * Une jauge mesure une AVANCÉE. Peinte en rouge parce que la carte signale des
 * retards, « 31 revues rendues sur 39 » se lit comme « 80 % d'échec » — l'inverse
 * de ce que le chiffre dit. Elle reste donc à l'accent, et passe au vert quand
 * elle est pleine, parce qu'un travail achevé est un état. L'alerte, elle, se
 * porte sur la pastille d'icône et dans la précision écrite.
 */
const barClass = computed(() =>
  props.progress !== null && props.progress >= 1 ? 'bg-success-solid' : 'bg-accent-solid',
)

const hasSpark = computed(() => props.spark.length > 1)

const sparkSeries = computed(() => [{ name: props.label, data: props.spark }])

/**
 * UNE ÉTINCELLE N'A NI AXE, NI GRILLE, NI INFOBULLE. Elle donne une forme, pas
 * une valeur : la valeur est écrite au-dessus, en grand. Une infobulle sur un
 * tracé de 48 px de haut se survole par accident et masque le chiffre qu'on
 * venait lire.
 *
 * DES BÂTONS, PAS UNE AIRE LISSÉE, pour la même raison que la courbe voisine :
 * ces séries comptent des dépôts et des inscriptions, jour par jour. Lissée, une
 * série de 0, 1 et 2 produit des vagues régulières qui dépassent les valeurs
 * réelles et ne racontent plus rien — un ornement, pas une mesure.
 */
const sparkOptions = computed<ApexOptions>(() => {
  const base = baseOptions()
  return {
    ...base,
    chart: {
      ...base.chart,
      type: 'bar',
      sparkline: { enabled: true },
      animations: { enabled: false },
    },
    colors: [toneColor(chartTone.value)],
    plotOptions: { bar: { columnWidth: '64%', borderRadius: 1, borderRadiusApplication: 'end' } },
    stroke: { width: 0 },
    fill: { type: 'solid', opacity: 0.85 },
    tooltip: { enabled: false },
    grid: { ...base.grid, show: false, padding: { top: 2, right: 0, bottom: 0, left: 0 } },
  }
})
</script>

<template>
  <article class="flex min-w-0 flex-col rounded-lg border border-border bg-surface-raised shadow-xs">
    <div class="flex items-start gap-3 px-4 pt-4">
      <span
        class="flex size-9 shrink-0 items-center justify-center rounded-md"
        :class="TONE_ICON[props.tone]"
      >
        <UiIcon :name="props.icon" size="1.125rem" :stroke-width="1.8" />
      </span>

      <div class="min-w-0 flex-1">
        <h3 class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
          {{ props.label }}
        </h3>

        <p class="mt-1 flex flex-wrap items-baseline gap-x-1.5 gap-y-0">
          <span class="font-mono text-3xl leading-none font-bold tabular-nums text-text">
            {{ props.value }}
          </span>
          <span v-if="props.unit" class="text-sm text-text-muted">{{ props.unit }}</span>
        </p>
      </div>
    </div>

    <div class="min-w-0 px-4 pt-2">
      <p v-if="props.hint" class="truncate text-xs text-text-muted" :title="props.hint">
        {{ props.hint }}
      </p>

      <!-- LE MOUVEMENT, ÉCRIT ET FLÉCHÉ, jamais coloré : voir l'en-tête. -->
      <p v-if="props.delta" class="mt-1 flex items-center gap-1 text-xs text-text-secondary">
        <UiIcon
          v-if="props.delta.direction !== 'flat'"
          :name="props.delta.direction === 'up' ? 'sort-asc' : 'sort-desc'"
          size="0.875rem"
          :stroke-width="2"
        />
        <UiIcon v-else name="minus" size="0.875rem" :stroke-width="2" />
        {{ props.delta.label }}
      </p>
    </div>

    <!-- LE VISUEL, TOUT EN BAS ET BORD À BORD : il ferme la carte sans concurrencer
         le chiffre. Un seul des deux paraît — voir l'en-tête. -->
    <div class="mt-auto pt-3">
      <UiChart
        v-if="hasSpark"
        type="bar"
        decorative
        :series="sparkSeries"
        :options="sparkOptions"
        :height="48"
      />
      <div v-else-if="props.progress !== null" class="px-4 pb-4">
        <div class="h-1.5 overflow-hidden rounded-full bg-surface-sunken">
          <div
            class="h-full rounded-full transition-[width]"
            :class="barClass"
            :style="{ width: `${Math.min(Math.max(props.progress, 0), 1) * 100}%` }"
          />
        </div>
      </div>
      <div v-else class="pb-4" />
    </div>
  </article>
</template>

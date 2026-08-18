<script setup lang="ts">
import type { DashboardKpi, DashboardKpiKey } from '~/types/admin-dashboard'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES SIX CHIFFRES DE TÊTE, mis en forme.
 *
 * CE COMPOSANT NE CALCULE AUCUN INDICATEUR : il les reçoit tels que les
 * projections les rendent (voir `DashboardKpi`) et ne fait que les habiller —
 * libellé, format, unité, jauge. Un chiffre recalculé à l'écran finirait par ne
 * plus correspondre au graphique posé juste en dessous, et c'est exactement le
 * genre d'écart qu'on ne découvre qu'en réunion.
 *
 * LE FORMAT DÉPEND DE LA NATURE DU CHIFFRE, et c'est ici qu'il se décide : un
 * compte se sépare par milliers selon la locale, un taux s'écrit en pourcentage,
 * une échéance se compte en jours. La donnée, elle, reste un nombre — envoyer une
 * chaîne déjà formatée depuis l'API interdirait de la reformater en anglais.
 *
 * `null` N'EST PAS ZÉRO. Un taux d'acceptation sans dossier tranché affiche un
 * tiret et sa raison, jamais « 0 % » — qui ferait passer un comité qui n'a pas
 * commencé pour un comité qui a tout refusé.
 */

interface Props {
  kpis: DashboardKpi[]
  /** Fuseau de l'édition : l'échéance affichée le porte. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { date } = useDateTime()

/** Tiret cadratin : une absence de valeur, pas un libellé — rien à traduire. */
const NO_VALUE = '—'

const ICONS: Record<DashboardKpiKey, string> = {
  submissions: 'inbox',
  deadline: 'clock',
  review_progress: 'check-circle',
  acceptance_rate: 'chart',
  scheduled: 'calendar',
  registrations: 'users',
}

function count(value: number): string {
  return new Intl.NumberFormat(locale.value).format(value)
}

/** « 62 % » — les taux du modèle sont des ratios entre 0 et 1. */
function percent(ratio: number): string {
  return t('common.formats.percent', {
    value: new Intl.NumberFormat(locale.value).format(Math.round(ratio * 100)),
  })
}

interface Card {
  key: DashboardKpiKey
  label: string
  value: string
  unit: string
  hint: string
  delta: { label: string; direction: 'up' | 'down' | 'flat' } | null
  spark: number[]
  progress: number | null
  tone: DashboardKpi['tone']
  icon: string
}

/** La variation, en toutes lettres. La flèche double le texte, elle ne le remplace pas. */
function deltaOf(kpi: DashboardKpi): Card['delta'] {
  if (kpi.delta === null) return null
  if (kpi.delta === 0) return { label: t('admin.dashboard.kpi.delta.flat'), direction: 'flat' }
  const direction = kpi.delta > 0 ? 'up' : 'down'
  return {
    label: t(`admin.dashboard.kpi.delta.${direction}`, { value: count(Math.abs(kpi.delta)) }),
    direction,
  }
}

/** Le rapport d'un indicateur à deux membres, borné : une jauge ne dépasse pas. */
function ratio(kpi: DashboardKpi): number | null {
  if (kpi.value === null || kpi.out_of === null || kpi.out_of === 0) return null
  return kpi.value / kpi.out_of
}

function build(kpi: DashboardKpi): Card {
  const common = {
    key: kpi.key,
    label: t(`admin.dashboard.kpi.${kpi.key}.label`),
    delta: deltaOf(kpi),
    spark: kpi.spark,
    progress: null as number | null,
    tone: kpi.tone,
    // UNE CARTE EN ALERTE PORTE UNE ICÔNE D'ALERTE. Le rond rouge marqué d'une
    // coche de l'avancement du comité se lisait à contresens : la coche dit
    // « c'est fait », la couleur dit « il y a un retard ».
    icon: kpi.tone === 'danger' ? 'warning' : ICONS[kpi.key],
    unit: '',
    hint: '',
    value: NO_VALUE,
  }

  switch (kpi.key) {
    case 'submissions':
    case 'registrations':
      return {
        ...common,
        value: kpi.value === null ? NO_VALUE : count(kpi.value),
        hint: t(`admin.dashboard.kpi.${kpi.key}.hint`),
      }

    /*
     * L'ÉCHÉANCE SE LIT DANS LES DEUX SENS. Devant, on compte les jours qui
     * restent ; derrière, ceux qui se sont écoulés — et pas « 0 jour », qui se
     * lirait « aujourd'hui ». Un appel clos reste un fait utile : c'est lui qui
     * explique qu'aucun dépôt n'arrive plus.
     */
    case 'deadline': {
      if (kpi.value === null) {
        return { ...common, hint: t('admin.dashboard.kpi.deadline.noCall') }
      }
      const days = Math.abs(kpi.value)
      return {
        ...common,
        value: count(days),
        unit:
          kpi.value > 0
            ? t('admin.dashboard.kpi.deadline.remaining', days)
            : t('admin.dashboard.kpi.deadline.elapsed', days),
        hint: kpi.at
          ? t('admin.dashboard.kpi.deadline.hint', { date: date(kpi.at, props.timezone) })
          : '',
        /*
         * LA JAUGE MESURE LA FENÊTRE DE DÉPÔT CONSOMMÉE, pas les jours restants :
         * c'est ce qui donne l'échelle. « 44 jours » ne veut pas dire la même
         * chose sur une fenêtre de deux mois et sur une fenêtre de six. Pleine
         * quand l'appel est clos.
         */
        progress:
          kpi.out_of === null
            ? null
            : kpi.value <= 0
              ? 1
              : (kpi.out_of - kpi.value) / kpi.out_of,
      }
    }

    case 'review_progress':
      if (kpi.value === null || kpi.out_of === null) {
        return { ...common, hint: t('admin.dashboard.kpi.review_progress.none') }
      }
      return {
        ...common,
        value: count(kpi.value),
        unit: t('admin.dashboard.kpi.review_progress.unit', { count: count(kpi.out_of) }),
        hint: t('admin.dashboard.kpi.review_progress.hint'),
        progress: ratio(kpi),
      }

    case 'acceptance_rate':
      if (kpi.value === null) {
        return { ...common, hint: t('admin.dashboard.kpi.acceptance_rate.none') }
      }
      return {
        ...common,
        value: percent(kpi.value),
        hint: t('admin.dashboard.kpi.acceptance_rate.hint', kpi.out_of ?? 0),
        progress: kpi.value,
      }

    /*
     * PAS DE RAPPORT ICI. Les séances créées ne se comparent pas aux dossiers
     * retenus : la projection compte les séances issues de TOUT dossier de
     * l'appel, retenu ou non, et les deux ensembles ne sont pas emboîtés (voir
     * `buildKpis` dans les données simulées). « 18 sur 16 » affirmerait un
     * rapport qui n'existe pas.
     */
    case 'scheduled':
      if (kpi.value === null) {
        return { ...common, hint: t('admin.dashboard.kpi.scheduled.none') }
      }
      return {
        ...common,
        value: count(kpi.value),
        hint: t('admin.dashboard.kpi.scheduled.hint'),
      }
  }
}

const cards = computed<Card[]>(() => props.kpis.map(build))
</script>

<template>
  <ul class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
    <li v-for="card in cards" :key="card.key" class="min-w-0">
      <AdminStatCard
        class="h-full"
        :label="card.label"
        :value="card.value"
        :unit="card.unit"
        :hint="card.hint"
        :delta="card.delta"
        :spark="card.spark"
        :progress="card.progress"
        :tone="card.tone"
        :icon="card.icon"
      />
    </li>
  </ul>
</template>

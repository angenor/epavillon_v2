<script setup lang="ts">
import type { TrendPoint } from '~/types/admin-dashboard'
import type { IsoDateTime, TimeZoneName } from '~/types/shared'

/**
 * COURBE SOBRE — une série, une couleur, sa légende posée sur la courbe.
 *
 * DES BÂTONS, PAS UNE LIGNE, ET C'EST UN CHOIX DE VÉRITÉ. Une valeur QUOTIDIENNE
 * est un compte, pas une grandeur continue : relier « 3 dépôts mardi » à
 * « 0 mercredi » par un segment dessine une pente qui n'a jamais existé, et sur
 * une série creuse — le cas ordinaire d'un appel à propositions — le tracé
 * devient un peigne dont on ne lit plus rien. Un bâton par jour ne prétend rien
 * entre deux jours, et l'effet de dernière minute s'y voit d'un coup d'œil.
 *
 * CE QU'ELLE NE FAIT PAS, et c'est ce qui la rend lisible : pas de dégradé, pas
 * d'ombre portée, pas de troisième dimension, pas de boîte de légende posée à
 * côté. Le nom de la série est écrit AU BOUT DE LA SÉRIE, là où l'œil arrive :
 * une légende séparée oblige à faire l'aller-retour entre une pastille de
 * couleur et un tracé, ce qui n'a de sens qu'à partir de trois séries — et à
 * partir de trois séries, ce n'est plus un tableau de bord, c'est un rapport.
 *
 * ELLE NE REBOUCHE AUCUN TROU. Les séries du modèle sont CONTINUES par
 * construction (`generate_series` dans `mv_daily_submissions` et
 * `mv_daily_registrations`), jours à zéro compris. Si un jour manque, c'est la
 * requête qui est en cause, et l'inventer ici masquerait le défaut.
 *
 * LES REPÈRES VERTICAUX PORTENT LEUR LIBELLÉ, et ils ÉTENDENT L'AXE. « L'échéance
 * marquée » du prompt n'est pas un trait de plus : sans elle, l'effet de dernière
 * minute — 60 % des dépôts sur les 48 dernières heures, mesuré en v1 — se lit
 * comme un pic devant rien. Et comme l'échéance est le plus souvent DEVANT, le
 * cadre doit aller la chercher : voir `domain`.
 *
 * DEUX PLANS SUPERPOSÉS, et c'est délibéré : le tracé est un SVG étiré
 * (`preserveAspectRatio="none"`), les textes sont du HTML posé par-dessus. Un
 * texte placé DANS un SVG étiré se déforme avec lui — lettres écrasées à 375 px,
 * étirées à 1400. Les traits, eux, gardent leur épaisseur par
 * `vector-effect: non-scaling-stroke`.
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
  /** Nom de la série, écrit au bout de la courbe. */
  seriesLabel: string
  /** Repères verticaux datés — ouverture de l'appel, échéance. */
  markers?: Marker[]
  /** Fuseau d'affichage des dates d'axe : celui de l'édition. */
  timezone: TimeZoneName
  /** Une couleur par série. `accent` pour la première, `postponed` pour la seconde. */
  tone?: 'accent' | 'postponed'
  /** Total affiché en tête — le cumul du dernier point, déjà calculé en base. */
  totalLabel?: string
}

const props = withDefaults(defineProps<Props>(), { tone: 'accent', markers: () => [] })

const { t } = useI18n()
const { date } = useDateTime()

/** Repère de dessin : les coordonnées SVG, étirées à la largeur disponible. */
const W = 1000
const H = 260
const PAD_TOP = 12
const PAD_BOTTOM = 18

const days = computed(() => props.points.map((point) => point.jour))

/** Le maximum sert d'échelle ; un plancher à 1 évite la division par zéro. */
const maxValue = computed(() => Math.max(1, ...props.points.map((point) => point.valeur)))

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
 * mais l'échéance, elle, est devant. Positionner les points par leur rang
 * placerait l'échéance au bord du cadre, c'est-à-dire aujourd'hui, ce qui est
 * faux. Le domaine couvre donc la série ET ses repères ; la courbe occupe la
 * part gauche du cadre, l'échéance se tient à sa place, et l'écart entre les
 * deux est précisément ce qu'on vient lire.
 */
const domain = computed(() => {
  const times = [
    ...days.value.map(dayTime),
    ...props.markers.map((marker) => dayTime(marker.at)),
  ].filter((t) => Number.isFinite(t))
  const start = Math.min(...times)
  const end = Math.max(...times)
  // Une série d'un seul jour, sans repère : le domaine serait de largeur nulle.
  return { start, span: Math.max(end - start, 86_400_000) }
})

function ratioOf(day: string): number {
  return (dayTime(day) - domain.value.start) / domain.value.span
}

function x(day: string): number {
  return ratioOf(day) * W
}

function y(value: number): number {
  const usable = H - PAD_TOP - PAD_BOTTOM
  return H - PAD_BOTTOM - (value / maxValue.value) * usable
}

/**
 * Largeur d'un bâton : la journée, ramenée au repère de dessin, moins un filet
 * de respiration. Plancher à un point et demi — sous cette largeur, un jour à
 * une seule unité disparaît, et c'est précisément le jour qu'on cherche.
 */
const barWidth = computed(() => Math.max((86_400_000 / domain.value.span) * W * 0.8, 1.5))

/** Les bâtons, un par jour de la série. Aucun n'est dessiné pour un jour à zéro. */
const bars = computed(() =>
  props.points
    .filter((point) => point.valeur > 0)
    .map((point) => ({
      key: point.jour,
      x: x(point.jour) - barWidth.value / 2,
      y: y(point.valeur),
      height: H - PAD_BOTTOM - y(point.valeur),
    })),
)

const placedMarkers = computed(() =>
  props.markers.map((marker) => ({
    ...marker,
    ratio: ratioOf(marker.at),
    // Un libellé centré sur un repère collé au bord déborderait du cadre : les
    // deux extrémités s'alignent donc sur l'intérieur.
    align: ratioOf(marker.at) < 0.12 ? 'start' : ratioOf(marker.at) > 0.88 ? 'end' : 'center',
  })),
)

/** Fin du domaine, qui n'est pas la fin de la série quand un repère la dépasse. */
const domainEndDay = computed(() =>
  new Date(domain.value.start + domain.value.span).toISOString().slice(0, 10),
)

/** Position de la légende : AU BOUT DE LA COURBE, pas au bord du cadre. */
const seriesEndRatio = computed(() => ratioOf(days.value.at(-1) ?? ''))

const fillClass = computed(() => (props.tone === 'accent' ? 'fill-accent' : 'fill-postponed'))
const textClass = computed(() => (props.tone === 'accent' ? 'text-accent' : 'text-postponed'))

const lastPoint = computed(() => props.points.at(-1) ?? null)
const peak = computed(() =>
  props.points.reduce<TrendPoint | null>(
    (best, point) => (best === null || point.valeur > best.valeur ? point : best),
    null,
  ),
)

/**
 * Résumé lu par les lecteurs d'écran. Une courbe est une image : sans ce texte,
 * elle ne dit rien. On y met ce qu'un œil en retire — la période, le total, le
 * jour le plus haut.
 */
const summary = computed(() =>
  t('admin.dashboard.charts.summary', {
    series: props.seriesLabel,
    from: date(days.value[0] ?? '', props.timezone),
    to: date(days.value.at(-1) ?? '', props.timezone),
    total: lastPoint.value?.cumul ?? 0,
    peakValue: peak.value?.valeur ?? 0,
    peakDay: date(peak.value?.jour ?? '', props.timezone),
  }),
)
</script>

<template>
  <figure class="min-w-0">
    <figcaption class="mb-3 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h3 class="text-base font-semibold text-text">{{ props.title }}</h3>
      <p v-if="props.totalLabel" class="text-sm tabular-nums text-text-secondary">
        {{ props.totalLabel }}
      </p>
    </figcaption>

    <div v-if="props.points.length === 0" class="rounded-md border border-border-subtle bg-surface-sunken px-4 py-8 text-center text-sm text-text-muted">
      {{ t('admin.dashboard.charts.noData') }}
    </div>

    <template v-else>
      <div class="relative">
        <svg
          :viewBox="`0 0 ${W} ${H}`"
          preserveAspectRatio="none"
          class="h-48 w-full sm:h-56"
          role="img"
          :aria-label="summary"
        >
          <!-- Ligne de base seule : pas de grille. Sur une série quotidienne
               dense, un quadrillage ajoute plus de traits que de lecture. -->
          <line
            :x1="0"
            :y1="H - PAD_BOTTOM"
            :x2="W"
            :y2="H - PAD_BOTTOM"
            class="stroke-border"
            stroke-width="1"
            vector-effect="non-scaling-stroke"
          />

          <line
            v-for="marker in placedMarkers"
            :key="marker.label"
            :x1="marker.ratio * W"
            :y1="0"
            :x2="marker.ratio * W"
            :y2="H - PAD_BOTTOM"
            :class="marker.kind === 'deadline' ? 'stroke-danger' : 'stroke-border-strong'"
            :stroke-dasharray="marker.kind === 'deadline' ? '0' : '4 4'"
            stroke-width="1.5"
            vector-effect="non-scaling-stroke"
          />

          <rect
            v-for="bar in bars"
            :key="bar.key"
            :x="bar.x"
            :y="bar.y"
            :width="barWidth"
            :height="bar.height"
            :class="fillClass"
          />
        </svg>

        <!-- Textes posés PAR-DESSUS le tracé étiré : ils gardent leur dessin. -->
        <p
          v-for="marker in placedMarkers"
          :key="marker.label"
          class="pointer-events-none absolute top-0 max-w-24 text-[0.6875rem] leading-tight font-semibold"
          :class="[
            marker.kind === 'deadline' ? 'text-danger' : 'text-text-subtle',
            marker.align === 'center' ? '-translate-x-1/2 text-center' : '',
            marker.align === 'end' ? '-translate-x-full text-right' : '',
          ]"
          :style="{ left: `${marker.ratio * 100}%` }"
        >
          {{ marker.label }}
        </p>

        <!-- LA LÉGENDE, POSÉE SUR LA COURBE, au bout, là où l'œil arrive. -->
        <p
          class="pointer-events-none absolute -top-1 text-xs font-semibold"
          :class="[textClass, seriesEndRatio > 0.85 ? '-translate-x-full' : '']"
          :style="{ left: `${Math.min(seriesEndRatio * 100, 100)}%` }"
        >
          {{ props.seriesLabel }}
        </p>
      </div>

      <!-- Axe des dates : les deux bornes, et rien entre elles. Une graduation
           complète sur cent jours produit un peigne illisible. -->
      <div class="mt-1 flex justify-between text-xs tabular-nums text-text-subtle">
        <span>{{ date(days[0] ?? '', props.timezone) }}</span>
        <!-- La borne droite est celle du CADRE, pas celle de la série : quand
             l'échéance est devant, c'est elle qui ferme l'axe. -->
        <span>{{ date(domainEndDay, props.timezone) }}</span>
      </div>
    </template>
  </figure>
</template>

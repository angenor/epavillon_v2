<script setup lang="ts">
import type { BreakdownSlice } from '~/types/admin-dashboard'

/**
 * RÉPARTITION — pays d'origine, thématiques.
 *
 * DES BARRES HORIZONTALES, ET NON UN CAMEMBERT. Comparer deux angles est une
 * tâche de perception coûteuse ; comparer deux longueurs alignées sur un même
 * bord ne l'est pas. Sur dix parts dont les trois premières écrasent le reste,
 * un camembert devient une roue de fines lamelles sans libellé lisible.
 *
 * LA COULEUR VIENT DE LA BASE, quand il y en a une. Les thématiques portent leur
 * `color_hex` dans `reference.taxonomy_terms`, modifiable au back-office : c'est
 * elle qui s'affiche, et rien d'autre. Les figer dans la feuille de style est le
 * défaut n° 1 de la v1. Les répartitions sans couleur propre — les pays —
 * prennent l'accent : une seule teinte pour une seule série.
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

const fallbackClass = computed(() =>
  props.fallbackTone === 'accent' ? 'bg-accent-solid' : 'bg-postponed',
)

function share(slice: BreakdownSlice): string {
  return t('common.formats.percent', {
    value: new Intl.NumberFormat(locale.value).format(Math.round(slice.share * 100)),
  })
}

/** La barre la plus longue occupe toute la largeur : c'est une comparaison, pas une échelle absolue. */
const maxShare = computed(() => Math.max(...props.slices.map((slice) => slice.share), 0.0001))
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

    <ul v-else class="mt-3 flex flex-col gap-2">
      <li v-for="slice in props.slices" :key="slice.key" class="min-w-0">
        <div class="flex items-baseline justify-between gap-3">
          <span class="min-w-0 truncate text-sm text-text-secondary">{{ tr(slice.label) }}</span>
          <span class="shrink-0 font-mono text-sm tabular-nums text-text">
            {{ slice.count }}
            <span class="text-text-subtle">· {{ share(slice) }}</span>
          </span>
        </div>
        <div class="mt-1 h-2 rounded-sm bg-surface-sunken">
          <div
            class="h-full rounded-sm"
            :class="slice.color ? '' : fallbackClass"
            :style="{
              width: `${(slice.share / maxShare) * 100}%`,
              backgroundColor: slice.color ?? undefined,
            }"
          />
        </div>
      </li>
    </ul>
  </section>
</template>

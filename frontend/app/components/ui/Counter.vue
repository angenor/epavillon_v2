<script setup lang="ts">
/**
 * Compteur — le nombre d'éléments d'un onglet, d'une entrée de navigation, d'une
 * file d'attente.
 *
 * C'EST UNE INFORMATION, PAS UNE ALERTE. Le ton neutre est le défaut : « 12
 * propositions » ne demande rien à personne. Le ton accentué est réservé à ce
 * qui attend une action — des évaluations à rendre, des demandes de correction
 * ouvertes. Peindre tous les compteurs en accent, c'est n'en distinguer aucun.
 *
 * PLAFONNÉ À 99+. Au-delà, le chiffre exact n'aide plus à décider et fait sauter
 * la largeur de l'entrée qui le porte. Le compte réel reste énoncé pour les
 * lecteurs d'écran.
 *
 * Chiffres en chasse fixe et tabulaires : dans une liste de compteurs empilés,
 * les colonnes s'alignent.
 */

interface Props {
  value: number
  /** `accent` : ce qui attend une action. `neutral` : le décompte ordinaire. */
  tone?: 'neutral' | 'accent'
  /** Valeur au-delà de laquelle on affiche « N+ ». */
  max?: number
}

const props = withDefaults(defineProps<Props>(), { tone: 'neutral', max: 99 })

const display = computed(() => (props.value > props.max ? `${props.max}+` : String(props.value)))
const isTruncated = computed(() => props.value > props.max)
</script>

<template>
  <span
    class="relative inline-flex h-[22px] min-w-[22px] items-center justify-center rounded-full px-1.5 font-mono text-xs font-bold tabular-nums"
    :class="
      props.tone === 'accent'
        ? 'bg-accent-solid text-accent-contrast'
        : 'bg-neutral-surface text-text-secondary'
    "
  >
    <span aria-hidden="true">{{ display }}</span>
    <!-- Le compte exact reste énoncé : « 99+ » ne dit pas s'il y en a cent ou mille. -->
    <span class="sr-only">{{ isTruncated ? props.value : display }}</span>
  </span>
</template>

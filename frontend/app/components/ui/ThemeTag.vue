<script setup lang="ts">
import type { ThemeBadge } from '~/types/ui'

/**
 * PASTILLE THÉMATIQUE — un terme de `reference.taxonomy_terms`.
 *
 * LA COULEUR EST UNE DONNÉE, PAS UN JETON. Le guide de style de référence fige
 * huit thématiques dans autant de triplets de jetons CSS. On ne l'a pas suivi
 * sur ce point, et c'est la seule divergence assumée avec lui : le modèle compte
 * dix-sept thématiques, chacune portant sa `color_hex` en base, modifiable au
 * back-office. Les figer dans la feuille de style reproduirait mot pour mot le
 * défaut n° 1 de la v1 — des libellés et des couleurs de thématiques recopiés
 * dans le front, désynchronisés de la base dès la première retouche.
 *
 * CE QU'ON GARDE DU GUIDE : le dessin. Pilule, fond teinté, bordure de la même
 * famille, libellé complet en gras — jamais d'abréviation ni de code.
 *
 * CE QU'ON EN CHANGE : le TEXTE reste en `--color-text`, il ne prend pas la
 * teinte. Le guide peut se le permettre, ses huit couleurs sont choisies et
 * vérifiées ; les nôtres sont saisies par un administrateur et rien ne garantit
 * leur contraste, ni en clair ni en sombre. Le fond à 12 % et la bordure à 40 %
 * portent la teinte sans jamais mettre la lisibilité en jeu — `color-mix`
 * travaille sur la couleur reçue, quelle qu'elle soit.
 *
 * LA RÈGLE DES TROIS. Le guide plafonne à trois pastilles par carte, les
 * suivantes se repliant en « +2 ». C'est `UiThemeTagList` qui l'applique : une
 * pastille seule ne peut pas savoir combien de voisines elle a.
 */

interface Props {
  theme: ThemeBadge
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const { tr } = useI18nText()

const label = computed(() =>
  typeof props.theme.label === 'string' ? props.theme.label : tr(props.theme.label),
)

/**
 * Sans couleur en base, la pastille retombe sur les jetons neutres — un terme
 * sans `color_hex` est un cas parfaitement normal du modèle, pas une anomalie.
 */
const style = computed(() => {
  const color = props.theme.color
  if (!color) return undefined
  return {
    backgroundColor: `color-mix(in srgb, ${color} 12%, transparent)`,
    borderColor: `color-mix(in srgb, ${color} 40%, transparent)`,
  }
})
</script>

<template>
  <span
    class="inline-flex items-center gap-2 rounded-full border border-border bg-surface-sunken font-bold whitespace-nowrap text-text"
    :class="props.size === 'sm' ? 'px-2.5 py-0.5 text-[0.6875rem]' : 'px-3 py-1 text-xs'"
    :style="style"
  >
    <span
      v-if="props.theme.color"
      class="size-2 shrink-0 rounded-full"
      :style="{ backgroundColor: props.theme.color }"
      aria-hidden="true"
    />
    {{ label }}
  </span>
</template>

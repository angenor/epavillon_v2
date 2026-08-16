<script setup lang="ts">
import type { ThemeBadge } from '~/types/ui'

/**
 * Liste de pastilles thématiques, PLAFONNÉE.
 *
 * LA RÈGLE DES TROIS vient du guide de style : trois pastilles au plus sur une
 * carte, les suivantes se replient en « +2 ». Ce n'est pas une coquetterie de
 * mise en page. Une activité peut porter cinq ou six thématiques — le modèle ne
 * les limite pas — et six pastilles sur une carte de programmation prennent plus
 * de place que le titre. Passé trois, elles cessent d'informer : personne ne lit
 * six étiquettes, on retient « cette activité parle de tout ».
 *
 * LE REPLI RESTE ACCESSIBLE : les thématiques masquées sont énoncées dans le
 * `title` et pour les lecteurs d'écran. Un « +3 » muet cacherait l'information à
 * ceux qui ne peuvent pas survoler.
 *
 * Ce composant existe SÉPARÉMENT de `UiThemeTag` parce qu'une pastille seule ne
 * peut pas savoir combien de voisines elle a — la règle porte sur la liste.
 */

interface Props {
  themes: ThemeBadge[]
  /** Nombre de pastilles visibles avant repli. */
  max?: number
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { max: 3, size: 'md' })

const { t } = useI18n()
const { tr } = useI18nText()

const labelOf = (theme: ThemeBadge): string =>
  typeof theme.label === 'string' ? theme.label : tr(theme.label)

const visible = computed(() => props.themes.slice(0, props.max))
const hidden = computed(() => props.themes.slice(props.max))
const hiddenLabels = computed(() => hidden.value.map(labelOf).join(', '))
</script>

<template>
  <div v-if="props.themes.length" class="flex flex-wrap items-center gap-2">
    <UiThemeTag v-for="theme in visible" :key="theme.code" :theme="theme" :size="props.size" />

    <span
      v-if="hidden.length"
      class="inline-flex items-center rounded-full border border-border bg-surface-sunken font-bold whitespace-nowrap text-text-secondary"
      :class="props.size === 'sm' ? 'px-2.5 py-0.5 text-[0.6875rem]' : 'px-3 py-1 text-xs'"
      :title="hiddenLabels"
    >
      +{{ hidden.length }}
      <!-- Les thématiques repliées restent énoncées : un « +3 » muet cacherait
           l'information à qui ne peut pas survoler. -->
      <span class="sr-only"> — {{ t('session-card.themes.more', { themes: hiddenLabels }) }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import type { ScheduleThemeBadge } from '~/types/views'

/**
 * LES THÉMATIQUES D'UNE SÉANCE, SUR LE PANNEAU SOMBRE.
 *
 * Pourquoi pas `UiThemeTagList` : celui-ci ne connaît que la surface claire —
 * teinte à 12 %, texte en `--color-text` — et disparaît sur le verre du panneau.
 * Même arbitrage que `HomeNatureBadge`, et pour la même raison : un composant
 * d'interface partagé n'a pas à porter la contrainte d'un seul bandeau.
 *
 * DEUX PASTILLES, PAS TROIS. Le guide plafonne à trois par carte ; dans une
 * colonne de 340 px, la troisième passe à la ligne et pousse la carte suivante
 * sous le pli. Le repli reste énoncé pour les lecteurs d'écran — un « +2 » muet
 * cacherait l'information à qui ne peut pas survoler.
 *
 * LA COULEUR VIENT DE LA BASE (`reference.taxonomy_terms.color_hex`) et ne porte
 * que le point : rien ne garantit son contraste sur une photographie voilée.
 */

interface Props {
  themes: ScheduleThemeBadge[]
  /** Nombre de pastilles visibles avant repli. */
  max?: number
}

const props = withDefaults(defineProps<Props>(), { max: 2 })

const { t } = useI18n()
const { tr } = useI18nText()

const visible = computed(() => props.themes.slice(0, props.max))
const hidden = computed(() => props.themes.slice(props.max))
const hiddenLabels = computed(() => hidden.value.map((theme) => tr(theme.label)).join(', '))
</script>

<template>
  <div v-if="props.themes.length" class="flex flex-wrap items-center gap-1.5">
    <span
      v-for="theme in visible"
      :key="theme.code"
      class="inline-flex items-center gap-1.5 rounded-full border border-glass-border bg-glass-raised px-2.5 py-0.5 text-[0.6875rem] font-bold whitespace-nowrap text-text-on-inverse"
    >
      <span
        v-if="theme.color"
        class="size-2 shrink-0 rounded-full ring-1 ring-ring-contrast/40"
        :style="{ backgroundColor: theme.color }"
        aria-hidden="true"
      />
      {{ tr(theme.label) }}
    </span>

    <span
      v-if="hidden.length"
      class="inline-flex items-center rounded-full border border-glass-border bg-glass-raised px-2.5 py-0.5 text-[0.6875rem] font-bold whitespace-nowrap text-text-on-inverse-muted"
      :title="hiddenLabels"
    >
      +{{ hidden.length }}
      <span class="sr-only"> — {{ t('home.aside.programme.moreThemes', { themes: hiddenLabels }) }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import type { ModeDeLecture } from '~/utils/guide-nego/appareil-lecture'

/**
 * « Pages · Texte » : le choix du mode de lecture, dans la barre dépliée — à la place de
 * « Marquer » (écarts 42 et 43) — et dans la feuille des réglages. Deux boutons à
 * `aria-pressed` : chacun dit ce qu'il montre, et celui qu'on lit est enfoncé.
 */
const props = withDefaults(defineProps<{ variante?: 'barre' | 'feuille' }>(), { variante: 'feuille' })
const mode = defineModel<ModeDeLecture>({ required: true })

const { t } = useI18n()

const MODES: ModeDeLecture[] = ['pages', 'texte']
</script>

<template>
  <div class="gn-choix-mode" :class="`gn-choix-mode--${props.variante}`" role="group" :aria-label="t('gn-choix-mode.groupe')">
    <button
      v-for="choix in MODES"
      :key="choix"
      type="button"
      class="gn-choix-mode__segment"
      :class="{ 'gn-choix-mode__segment--actif': mode === choix }"
      :aria-pressed="mode === choix"
      @click="mode = choix"
    >
      {{ t(`gn-choix-mode.${choix}`) }}
    </button>
  </div>
</template>

<style>
/* Deux pilules de jour, comme GnSegmente : le mode lu est à l'accent, l'autre bordé. */
[data-app="guide-nego"] .gn-choix-mode {
  display: flex;
  gap: 6px;
}

/* Dans la barre, le choix tient l'emplacement d'une action : un quart de 320 px. */
[data-app="guide-nego"] .gn-choix-mode--barre {
  flex: 1 1 0;
  min-width: calc(2 * var(--gn-onglet-min));
  align-self: center;
  gap: var(--gn-espace-4);
  margin-inline: var(--gn-onglet-air);
}

[data-app="guide-nego"] .gn-choix-mode__segment {
  flex: 1;
  min-width: 0;
  height: var(--gn-pilule-jour);
  padding-inline: var(--gn-espace-16);
  display: flex;
  align-items: center;
  justify-content: center;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  background: transparent;
  color: var(--gn-texte-2);
  font: inherit;
  font-size: var(--gn-taille-15);
  line-height: 1;
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-choix-mode--barre .gn-choix-mode__segment {
  padding-inline: var(--gn-espace-4);
  font-size: var(--gn-taille-13);
}

[data-app="guide-nego"] .gn-choix-mode__segment:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-choix-mode__segment--actif {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-choix-mode__segment--actif:active {
  background: var(--gn-accent);
  filter: brightness(0.9);
}
</style>

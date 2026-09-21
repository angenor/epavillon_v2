<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * Un choix unique parmi trois ou quatre, tous visibles : taille de lecture, thème.
 *
 * Au clavier, le groupe entier est UNE étape de tabulation et les flèches passent d'un
 * segment à l'autre, comme un groupe de boutons radio — c'est ce qu'attend un lecteur
 * d'écran, et ce qui évite quatre arrêts pour un seul réglage.
 */
export interface SegmentDeChoix {
  valeur: string
  libelle: string
  picto?: NomDePicto
}

const props = defineProps<{ segments: SegmentDeChoix[]; libelle: string }>()
const choix = defineModel<string>({ required: true })

const groupe = useTemplateRef<HTMLElement>('groupe')

function deplacer(pas: number) {
  const index = props.segments.findIndex((segment) => segment.valeur === choix.value)
  const suivant = props.segments[(index + pas + props.segments.length) % props.segments.length]
  if (!suivant) return
  choix.value = suivant.valeur
  nextTick(() => groupe.value?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus())
}
</script>

<template>
  <div ref="groupe" class="gn-segmente" role="radiogroup" :aria-label="libelle">
    <button
      v-for="segment in segments"
      :key="segment.valeur"
      type="button"
      role="radio"
      class="gn-segmente__segment"
      :class="{ 'gn-segmente__segment--actif': segment.valeur === choix }"
      :aria-checked="segment.valeur === choix"
      :tabindex="segment.valeur === choix ? 0 : -1"
      @click="choix = segment.valeur"
      @keydown.left.prevent="deplacer(-1)"
      @keydown.up.prevent="deplacer(-1)"
      @keydown.right.prevent="deplacer(1)"
      @keydown.down.prevent="deplacer(1)"
    >
      <GnPicto v-if="segment.picto" :nom="segment.picto" :taille="20" />
      {{ segment.libelle }}
    </button>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-segmente {
  display: flex;
  gap: var(--gn-filet-2);
  padding: var(--gn-filet-2);
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  border-radius: var(--gn-rayon-4);
}

[data-app="guide-nego"] .gn-segmente__segment {
  flex: 1;
  min-width: 0;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: none;
  background: none;
  color: var(--gn-texte);
  font-size: var(--gn-taille-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-segmente__segment:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-segmente__segment--actif {
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"][data-theme="sombre"] .gn-segmente__segment--actif {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-segmente__segment:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}
</style>

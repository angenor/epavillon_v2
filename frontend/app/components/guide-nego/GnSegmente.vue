<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * Un choix unique parmi trois ou quatre, tous visibles : taille de lecture.
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
/* Une rangée de pilules de jour : le choix est à l'accent, les autres sont bordés. */
[data-app="guide-nego"] .gn-segmente {
  display: flex;
  gap: 6px;
}

[data-app="guide-nego"] .gn-segmente__segment {
  flex: 1;
  min-width: 0;
  height: var(--gn-pilule-jour);
  padding-inline: var(--gn-espace-16);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  background: transparent;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: 1;
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
  gap: 6px;
}

[data-app="guide-nego"] .gn-segmente__segment:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-segmente__segment--actif {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}
</style>

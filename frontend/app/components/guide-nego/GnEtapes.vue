<script setup lang="ts">
/**
 * Où l'on en est dans le parcours d'entrée : compte, code, thématiques (FR-007).
 *
 * **Les segments sont décoratifs, le libellé porte le sens.** Une barre remplie
 * ne se lit pas à voix haute et ne se voit pas en fort contraste ; « Étape 2 sur
 * 3 — Code » se lit dans les deux cas. C'est pourquoi les segments sont
 * `aria-hidden` et le texte ne l'est pas.
 *
 * Trois écrans l'emploient — la création de compte, la saisie du code, et le
 * choix des thématiques à venir. Recopier ses huit lignes dans chacun donnerait
 * trois indicateurs à tenir accordés, dont deux se périmeraient.
 */
const props = withDefaults(
  defineProps<{
    /** 1, 2 ou 3. Les segments avant celle-ci comptent pour franchis. */
    courante: number
    total?: number
    /** « Étape 2 sur 3 — Code », composé par l'écran depuis son i18n. */
    libelle: string
  }>(),
  { total: 3 },
)

const segments = computed(() =>
  Array.from({ length: props.total }, (_, index) => index < props.courante),
)
</script>

<template>
  <p class="gn-etapes">
    <span class="gn-etapes__segments" aria-hidden="true">
      <span
        v-for="(franchi, index) in segments"
        :key="index"
        class="gn-etapes__segment"
        :class="{ 'gn-etapes__segment--franchi': franchi }"
      />
    </span>
    <span class="gn-etapes__libelle">{{ libelle }}</span>
  </p>
</template>

<style>
[data-app="guide-nego"] .gn-etapes {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-etapes__segments {
  display: flex;
  flex: 1;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-etapes__segment {
  flex: 1;
  height: var(--gn-segment-etape);
  background: var(--gn-filet);
}

[data-app="guide-nego"] .gn-etapes__segment--franchi {
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-etapes__libelle {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-titre);
}
</style>

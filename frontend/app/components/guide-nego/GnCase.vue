<script setup lang="ts">
/**
 * Une ligne à cocher : toute la ligne de 56 px est la cible, jamais le seul carré de 24.
 * Un pouce en salle de négociation ne vise pas 24 px.
 *
 * La case est une vraie case native, masquée sous son libellé. C'est ce qui donne
 * gratuitement la bascule à la barre d'espace, l'envoi dans un formulaire et
 * l'annonce « case à cocher, cochée » : trois choses qu'un `role="checkbox"`
 * obligerait à réécrire, et dont la réécriture se démode.
 */
withDefaults(
  defineProps<{
    libelle: string
    detail?: string
    desactive?: boolean
    /** La dernière d'une liste n'a pas de filet : le bloc s'achève de lui-même. */
    derniere?: boolean
  }>(),
  { detail: undefined, desactive: false, derniere: false },
)

const coche = defineModel<boolean>({ default: false })
</script>

<template>
  <label
    class="gn-case"
    :class="{ 'gn-case--derniere': derniere, 'gn-case--desactive': desactive }"
  >
    <input
      v-model="coche"
      type="checkbox"
      class="gn-hors-ecran gn-case__natif"
      :disabled="desactive"
    >
    <span class="gn-case__carre" aria-hidden="true">
      <GnPicto v-if="coche" nom="check" :taille="16" />
    </span>
    <span class="gn-case__texte">
      <span class="gn-case__libelle">{{ libelle }}</span>
      <span v-if="detail" class="gn-case__detail">{{ detail }}</span>
    </span>
  </label>
</template>

<style>
[data-app="guide-nego"] .gn-case {
  position: relative;
  min-height: var(--gn-ligne-reglage);
  padding-block: var(--gn-espace-8);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  cursor: pointer;
}

[data-app="guide-nego"] .gn-case--derniere {
  border-bottom: none;
}

/* L'anneau se voit sur la ligne entière, pas sur le carré : c'est la ligne qu'on active. */
[data-app="guide-nego"] .gn-case:has(.gn-case__natif:focus-visible) {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-case:not(.gn-case--desactive):active {
  background: var(--gn-presse);
  /* La pression déborde les marges de l'écran, comme une ligne pleine largeur. */
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-case__carre {
  inline-size: var(--gn-case);
  block-size: var(--gn-case);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  border-radius: var(--gn-rayon-4);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-case__natif:checked + .gn-case__carre {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-case__texte {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-case__libelle {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-case__detail {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-case--desactive {
  color: var(--gn-desactive-texte);
  cursor: not-allowed;
}

[data-app="guide-nego"] .gn-case--desactive .gn-case__detail {
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-case--desactive .gn-case__carre {
  border-color: var(--gn-desactive-texte);
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-case--desactive .gn-case__natif:checked + .gn-case__carre {
  background: var(--gn-desactive-fond);
}
</style>

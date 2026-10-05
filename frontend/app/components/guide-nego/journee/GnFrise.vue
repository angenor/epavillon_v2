<script setup lang="ts">
/**
 * Une ligne de frise (Nuit 02), commune aux sessions, aux réunions, au Pavillon et à
 * mon agenda : l'heure, l'axe et son nœud coloré par l'état, la carte. Le lien est dans
 * la carte (`.gn-frise__lien`), son pseudo-élément la couvre : une étiquette qui mène
 * ailleurs peut y vivre sans faire un lien dans un lien.
 */
export type NoeudDeFrise = 'neutre' | 'attention' | 'danger' | 'reseau' | 'information' | 'mienne' | 'eteint'

withDefaults(
  defineProps<{
    heure: string
    noeud?: NoeudDeFrise
    /** Ma session : la carte est bordée d'accent. */
    bordee?: boolean
    /** Terminée ou annulée : l'heure passe au gris. */
    eteinte?: boolean
  }>(),
  { noeud: 'neutre', bordee: false, eteinte: false },
)
</script>

<template>
  <div class="gn-frise">
    <span class="gn-frise__heure" :class="{ 'gn-frise__heure--eteinte': eteinte }" aria-hidden="true">{{ heure }}</span>
    <span class="gn-frise__axe" aria-hidden="true">
      <span class="gn-frise__trait gn-frise__trait--haut" />
      <span class="gn-frise__noeud" :class="`gn-frise__noeud--${noeud}`" />
      <span class="gn-frise__trait" />
    </span>
    <div class="gn-frise__carte" :class="{ 'gn-frise__carte--bordee': bordee }">
      <slot />
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-frise {
  display: flex;
  gap: 14px;
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-frise__heure {
  flex: none;
  width: 46px;
  padding-top: var(--gn-espace-16);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__heure--eteinte {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-frise__axe {
  flex: none;
  width: 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

[data-app="guide-nego"] .gn-frise__trait {
  flex: 1;
  width: 2px;
  background: var(--gn-filet);
}

[data-app="guide-nego"] .gn-frise__trait--haut {
  flex: none;
  height: 20px;
}

[data-app="guide-nego"] .gn-frise__noeud {
  flex: none;
  width: 12px;
  height: 12px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-neutre-marque);
}

[data-app="guide-nego"] .gn-frise__noeud--attention {
  background: var(--gn-attention);
}

[data-app="guide-nego"] .gn-frise__noeud--danger {
  background: var(--gn-danger);
}

[data-app="guide-nego"] .gn-frise__noeud--reseau {
  background: var(--gn-reseau);
}

[data-app="guide-nego"] .gn-frise__noeud--information {
  background: var(--gn-information);
}

[data-app="guide-nego"] .gn-frise__noeud--mienne {
  background: transparent;
  border: 3px solid var(--gn-accent);
}

[data-app="guide-nego"] .gn-frise__noeud--eteint {
  background: var(--gn-filet);
}

[data-app="guide-nego"] .gn-frise__carte {
  position: relative;
  flex: 1;
  min-width: 0;
  margin: 6px 0 10px;
  padding: 14px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  background: var(--gn-fond-2);
  border: var(--gn-filet-1) solid transparent;
  border-radius: var(--gn-rayon-20);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-frise__carte--bordee {
  border-color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-frise__carte:has(.gn-frise__lien:active) {
  background: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-frise__lien {
  color: inherit;
  text-decoration: none;
}

[data-app="guide-nego"] .gn-frise__lien::after {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: var(--gn-rayon-20);
}

[data-app="guide-nego"] .gn-frise__lien:focus-visible {
  outline: none;
}

[data-app="guide-nego"] .gn-frise__lien:focus-visible::after {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: 2px;
}

/* Ce qui mène ailleurs dans la carte passe au-dessus du lien principal. */
[data-app="guide-nego"] .gn-frise__carte :where(a, button):not(.gn-frise__lien) {
  position: relative;
  z-index: 1;
}

[data-app="guide-nego"] .gn-frise__titre {
  font-size: var(--gn-taille-16);
  line-height: 1.3;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__titre--barre {
  color: var(--gn-texte-2);
  text-decoration: line-through;
}

[data-app="guide-nego"] .gn-frise__surtitre {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-12);
  line-height: var(--gn-interligne-12);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-frise__pilule {
  padding: 3px 9px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-frise__ton--attention {
  color: var(--gn-attention);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__ton--danger {
  color: var(--gn-danger);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__ton--reseau {
  color: var(--gn-reseau);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__ton--accent {
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
}
</style>

<script setup lang="ts">
/**
 * Interrupteur d'un réglage. Actif, il porte une coche dans son curseur : en plein
 * soleil, ou pour qui distingue mal le vert, la couleur seule ne dit rien.
 *
 * Par défaut il occupe la ligne entière de 56 px — libellé à gauche, piste à droite —
 * parce que c'est la ligne qui est la cible, pas la piste de 52 px. Posé dans le slot
 * de `GnLigneReglage`, qui porte déjà le libellé et le filet, passer `dans-ligne` :
 * le libellé ne s'affiche plus mais reste le nom accessible de la commande.
 *
 * Rien n'est repris de l'interrupteur du site : celui-là est moulé, celui-ci est plat.
 */
withDefaults(
  defineProps<{
    libelle: string
    detail?: string
    desactive?: boolean
    /** Posé dans une ligne qui affiche déjà le libellé. */
    dansLigne?: boolean
    /** La dernière d'une liste n'a pas de filet : le bloc s'achève de lui-même. */
    derniere?: boolean
  }>(),
  { detail: undefined, desactive: false, dansLigne: false, derniere: false },
)

const actif = defineModel<boolean>({ default: false })
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="actif"
    :aria-label="dansLigne ? libelle : undefined"
    :disabled="desactive || undefined"
    class="gn-interrupteur"
    :class="{
      'gn-interrupteur--dans-ligne': dansLigne,
      'gn-interrupteur--derniere': derniere,
    }"
    @click="actif = !actif"
  >
    <span v-if="!dansLigne" class="gn-interrupteur__texte">
      <span class="gn-interrupteur__libelle">{{ libelle }}</span>
      <span v-if="detail" class="gn-interrupteur__detail">{{ detail }}</span>
    </span>
    <span class="gn-interrupteur__piste" aria-hidden="true">
      <span class="gn-interrupteur__curseur">
        <GnPicto v-if="actif" nom="check" :taille="16" />
      </span>
    </span>
  </button>
</template>

<style>
[data-app="guide-nego"] .gn-interrupteur {
  width: 100%;
  min-height: var(--gn-ligne-reglage);
  padding: var(--gn-espace-8) 0;
  display: flex;
  align-items: center;
  gap: 14px;
  border: none;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
  background: none;
  color: var(--gn-texte);
  text-align: start;
}

/* Dans une ligne qui porte déjà libellé et filet : la piste seule, mais une cible
   qui reste haute de 56 px et déborde de 8 px de chaque côté. */
[data-app="guide-nego"] .gn-interrupteur--dans-ligne {
  width: auto;
  margin-inline: calc(-1 * var(--gn-espace-8));
  padding-inline: var(--gn-espace-8);
  border-bottom: none;
}

[data-app="guide-nego"] .gn-interrupteur--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-interrupteur:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-interrupteur:not(:disabled):active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-interrupteur__texte {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-interrupteur__libelle {
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-interrupteur__detail {
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  color: var(--gn-texte-2);
}

/* Piste pleine, sans bord : la course du curseur vaut largeur − hauteur. */
[data-app="guide-nego"] .gn-interrupteur__piste {
  inline-size: var(--gn-interrupteur-largeur);
  block-size: var(--gn-interrupteur-hauteur);
  flex: none;
  display: flex;
  align-items: center;
  padding: calc((var(--gn-interrupteur-hauteur) - var(--gn-interrupteur-curseur)) / 2);
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-bloc-releve);
  transition: background var(--gn-duree-bref) var(--gn-courbe-standard);
}

[data-app="guide-nego"] .gn-interrupteur__curseur {
  inline-size: var(--gn-interrupteur-curseur);
  block-size: var(--gn-interrupteur-curseur);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-texte-2);
  color: var(--gn-accent);
  translate: 0;
  transition: translate var(--gn-duree-bref) var(--gn-courbe-standard),
              background var(--gn-duree-bref) var(--gn-courbe-standard);
}

[data-app="guide-nego"] .gn-interrupteur[aria-checked="true"] .gn-interrupteur__piste {
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-interrupteur[aria-checked="true"] .gn-interrupteur__curseur {
  background: var(--gn-fond);
  translate: calc(var(--gn-interrupteur-largeur) - var(--gn-interrupteur-hauteur));
}

[data-app="guide-nego"] .gn-interrupteur:disabled {
  color: var(--gn-desactive-texte);
  cursor: not-allowed;
}

[data-app="guide-nego"] .gn-interrupteur:disabled .gn-interrupteur__detail {
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-interrupteur:disabled .gn-interrupteur__curseur {
  background: var(--gn-neutre-marque);
  color: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-interrupteur:disabled[aria-checked="true"] .gn-interrupteur__piste {
  background: var(--gn-neutre-marque);
}

[data-app="guide-nego"] .gn-interrupteur:disabled[aria-checked="true"] .gn-interrupteur__curseur {
  background: var(--gn-bloc-releve);
  color: var(--gn-texte-2);
}
</style>

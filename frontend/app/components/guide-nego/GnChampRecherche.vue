<script setup lang="ts">
/**
 * Champ de recherche : loupe, saisie, puis croix pour effacer — ou l'arc quand la
 * réponse se fait attendre. Repos · focus avec saisie · chargement.
 *
 * La croix est un vrai bouton de 44 px, débordant sur la marge intérieure du champ :
 * une croix dessinée à la taille du pictogramme se rate un doigt sur trois. Après
 * l'effacement le focus revient dans la saisie, sinon il retombe dans le vide avec
 * le bouton qui disparaît.
 */
defineOptions({ inheritAttrs: false })

withDefaults(
  defineProps<{ libelle: string; indication?: string; chargement?: boolean }>(),
  { indication: undefined, chargement: false },
)

const texte = defineModel<string>({ default: '' })

const { t } = useI18n()
const saisie = useTemplateRef<HTMLInputElement>('saisie')

function effacer() {
  texte.value = ''
  saisie.value?.focus()
}
</script>

<template>
  <div class="gn-champ-recherche" role="search">
    <GnPicto nom="search" :taille="20" class="gn-champ-recherche__loupe" />

    <input
      ref="saisie"
      v-model="texte"
      class="gn-champ-recherche__saisie"
      type="search"
      enterkeyhint="search"
      autocomplete="off"
      :aria-label="libelle"
      :placeholder="indication"
      :aria-busy="chargement ? 'true' : undefined"
      v-bind="$attrs"
    >

    <GnChargement v-if="chargement" class="gn-champ-recherche__arc" />

    <button
      v-else-if="texte"
      type="button"
      class="gn-champ-recherche__effacer"
      :aria-label="t('gn-champ-recherche.effacer')"
      @click="effacer"
    >
      <GnPicto nom="close" :taille="20" />
    </button>
  </div>
</template>

<style>
/* Le champ de la maquette : 56 de haut, bloc bordé, rayon 18, loupe en texte secondaire. */
[data-app="guide-nego"] .gn-champ-recherche {
  min-height: var(--gn-champ);
  padding-inline: var(--gn-espace-16);
  display: flex;
  align-items: center;
  gap: 10px;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-18);
  background: var(--gn-fond-2);
  color: var(--gn-texte-2);
}

/* L'anneau entoure le champ entier : le bord s'allume à l'accent. */
[data-app="guide-nego"] .gn-champ-recherche:has(.gn-champ-recherche__saisie:focus-visible) {
  border-color: var(--gn-focus);
}

[data-app="guide-nego"] .gn-champ-recherche__loupe {
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-champ-recherche__saisie {
  flex: 1;
  min-width: 0;
  border: none;
  background: none;
  color: var(--gn-texte);
  font-family: var(--gn-police);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-champ-recherche__saisie::placeholder {
  color: var(--gn-texte-2);
  opacity: 1;
}

[data-app="guide-nego"] .gn-champ-recherche__saisie:focus-visible {
  outline: none;
}

/* La croix native de WebKit doublerait le bouton, sans nom accessible ni vraie cible. */
[data-app="guide-nego"] .gn-champ-recherche__saisie::-webkit-search-cancel-button {
  appearance: none;
}

/* Le bouton mord la marge intérieure : 44 de cible, pictogramme à 12 px du bord. */
[data-app="guide-nego"] .gn-champ-recherche__effacer {
  flex: none;
  width: var(--gn-bouton-rond);
  height: var(--gn-bouton-rond);
  margin-inline-end: calc(-1 * var(--gn-espace-12));
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--gn-rayon-22);
  background: none;
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-champ-recherche__effacer:active {
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-champ-recherche__effacer:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-champ-recherche__arc {
  flex: none;
}
</style>

<script setup lang="ts">
/**
 * L'en-tête d'écran. Le bouton « Aa » ouvre le lexique depuis n'importe où : un terme
 * anglais se cherche en salle, sans revenir en arrière.
 *
 * Ni notifications ni avatar à cette étape — ils viennent avec leurs écrans (0b, 3b).
 */
withDefaults(
  defineProps<{
    titre: string
    sousTitre?: string
    /** Écran secondaire : un retour remplace le titre d'onglet. */
    retour?: string
    /** Vrai sur le lexique lui-même : le bouton dit où l'on est. */
    lexiqueOuvert?: boolean
  }>(),
  { sousTitre: undefined, retour: undefined, lexiqueOuvert: false },
)

const { t } = useI18n()
</script>

<template>
  <header class="gn-entete">
    <div class="gn-entete__barre">
      <NuxtLink v-if="retour" :to="retour" class="gn-entete__bouton" :aria-label="t('gn-entete.retour')">
        <GnPicto nom="back" />
      </NuxtLink>
      <!-- Emplacement de la ligne de connexion : « Synchronisé à » ou « Hors connexion ». -->
      <div class="gn-entete__connexion"><slot name="connexion" /></div>
      <NuxtLink
        to="/guide-nego/lexique"
        class="gn-entete__bouton gn-entete__aa"
        :class="{ 'gn-entete__aa--ouvert': lexiqueOuvert }"
        :aria-label="lexiqueOuvert ? t('gn-entete.lexique-ouvert') : t('gn-entete.lexique')"
        :aria-current="lexiqueOuvert ? 'page' : undefined"
      >
        Aa
      </NuxtLink>
    </div>
    <h1 class="gn-entete__titre">{{ titre }}</h1>
    <p v-if="sousTitre" class="gn-entete__sous-titre">{{ sousTitre }}</p>
  </header>
</template>

<style>
/* Les marges de l'écran viennent de `.gn-ecran` (mesures.css) : l'en-tête n'en pose pas. */
[data-app="guide-nego"] .gn-entete {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-bottom: 10px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-entete__barre {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-en-tete-lecture);
}

[data-app="guide-nego"] .gn-entete__connexion {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-entete__bouton {
  flex: none;
  width: var(--gn-bouton-aa);
  height: var(--gn-bouton-aa);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--gn-titre);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-entete__bouton:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-entete__aa {
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  /* « Aa » est un glyphe de bouton, hors échelle de texte (écarts 8 et 9). */
  font-size: 18px;
  font-weight: var(--gn-graisse-gras);
  line-height: 1;
}

[data-app="guide-nego"] .gn-entete__aa--ouvert {
  background: var(--gn-titre);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"][data-theme="sombre"] .gn-entete__aa--ouvert {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-entete__titre {
  font-size: var(--gn-en-tete-titre);
  line-height: var(--gn-interligne-28);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-entete__sous-titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-accent);
}
</style>

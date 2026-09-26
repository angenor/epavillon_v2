<script setup lang="ts">
/**
 * L'en-tête d'écran. Le bouton « Aa » ouvre le lexique depuis n'importe où : un terme
 * anglais se cherche en salle, sans revenir en arrière.
 *
 * **L'avatar et le retour s'excluent** : un écran d'onglet porte l'avatar, qui ouvre
 * le profil ; un écran secondaire porte le retour. La cloche passe par l'emplacement `action`.
 */
export interface AvatarDEntete {
  prenom: string | null
  nom: string | null
  image?: string | null
}

withDefaults(
  defineProps<{
    titre: string
    sousTitre?: string
    /** Écran secondaire : un retour remplace le titre d'onglet. */
    retour?: string
    /** Écran d'onglet, personne connectée. Ignoré quand un retour est donné. */
    avatar?: AvatarDEntete
    /** Le profil : l'avatar en grand, à côté du titre qui porte le nom. */
    avatarDuTitre?: AvatarDEntete
    /** Vrai sur le lexique lui-même : le bouton dit où l'on est. */
    lexiqueOuvert?: boolean
    /** Le lecteur : une seule ligne, le titre en petit entre le retour et « Aa » (04 · 01). */
    compact?: boolean
    /** Un retour qui referme ce qui est posé sur l'écran — le sommaire du lecteur — au lieu d'y naviguer. */
    retourBouton?: boolean
  }>(),
  {
    sousTitre: undefined,
    retour: undefined,
    avatar: undefined,
    avatarDuTitre: undefined,
    lexiqueOuvert: false,
    compact: false,
    retourBouton: false,
  },
)

defineEmits<{ retour: [] }>()

const { t } = useI18n()
</script>

<template>
  <header class="gn-entete" :class="{ 'gn-entete--compact': compact }">
    <div class="gn-entete__barre">
      <button v-if="retourBouton" type="button" class="gn-entete__bouton" :aria-label="t('gn-entete.retour')" @click="$emit('retour')">
        <GnPicto nom="back" />
      </button>
      <NuxtLink v-else-if="retour" :to="retour" class="gn-entete__bouton" :aria-label="t('gn-entete.retour')">
        <GnPicto nom="back" />
      </NuxtLink>
      <GnAvatar
        v-else-if="avatar"
        :prenom="avatar.prenom"
        :nom="avatar.nom"
        :image="avatar.image"
        vers="/guide-nego/ressources/reglages"
      />
      <h1 v-if="compact" class="gn-entete__titre-compact">{{ titre }}</h1>
      <!-- Emplacement de la ligne de connexion : « Synchronisé à » ou « Hors connexion ». -->
      <div v-else class="gn-entete__connexion"><slot name="connexion" /></div>
      <!-- Un accès propre à l'écran, à côté de « Aa » : « Mon agenda » sur les sessions. -->
      <slot name="action" />
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
    <div v-if="!compact && avatarDuTitre" class="gn-entete__identite">
      <GnAvatar grand :prenom="avatarDuTitre.prenom" :nom="avatarDuTitre.nom" :image="avatarDuTitre.image" />
      <h1 class="gn-entete__titre">{{ titre }}</h1>
    </div>
    <h1 v-else-if="!compact" class="gn-entete__titre">{{ titre }}</h1>
    <div v-if="sousTitre && !compact && $slots['sous-titre-action']" class="gn-entete__ligne">
      <p class="gn-entete__sous-titre">{{ sousTitre }}</p>
      <!-- Une commande qui porte sur tout l'écran : « Tout marquer comme lu » (02 · 10). -->
      <slot name="sous-titre-action" />
    </div>
    <p v-else-if="sousTitre && !compact" class="gn-entete__sous-titre">{{ sousTitre }}</p>
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
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
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
  color: var(--gn-sur-titre);
}

[data-app="guide-nego"][data-theme="sombre"] .gn-entete__aa--ouvert {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-entete__identite {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-entete__titre {
  font-size: var(--gn-en-tete-titre);
  line-height: var(--gn-interligne-28);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-entete--compact {
  padding-bottom: var(--gn-espace-8);
  border-bottom-width: var(--gn-filet-1);
  border-bottom-color: var(--gn-filet);
}

[data-app="guide-nego"] .gn-entete__titre-compact {
  flex: 1;
  min-width: 0;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
  text-align: center;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-entete__ligne {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  column-gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-entete__sous-titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-accent);
}
</style>

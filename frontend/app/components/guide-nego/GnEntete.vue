<script setup lang="ts">
/**
 * L'en-tête d'écran de la maquette Nuit. Le bouton « Aa » ouvre le lexique depuis
 * n'importe où : un terme anglais se cherche en salle, sans revenir en arrière.
 *
 * Écran d'onglet : grand titre Sora 28 à gauche, boutons ronds à droite (« Sessions »).
 * Écran secondaire : retour rond, titre Sora 22 sur la même ligne (« Lexique »).
 * **L'avatar et le retour s'excluent** ; la cloche passe par l'emplacement `action`.
 */
export interface AvatarDEntete {
  prenom: string | null
  nom: string | null
  image?: string | null
}

const props = withDefaults(
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
    /** Le retour est une croix : l'écran se referme sur celui d'où l'on venait (le lexique). */
    fermer?: boolean
    /** Au-dessus du titre, en petit gris : la famille d'un terme du lexique. */
    surtitre?: string
    /** Le titre est un terme anglais : en italique, sa traduction en sous-titre plus grand. */
    terme?: boolean
    /** Une question de FAQ : le titre descend à 24, une phrase entière y tient. */
    titreLong?: boolean
    /** Écran racine d'un onglet : la loupe de la recherche globale, à gauche de « Aa » (R13). */
    loupe?: boolean
  }>(),
  {
    sousTitre: undefined,
    retour: undefined,
    avatar: undefined,
    avatarDuTitre: undefined,
    lexiqueOuvert: false,
    compact: false,
    retourBouton: false,
    fermer: false,
    surtitre: undefined,
    terme: false,
    titreLong: false,
    loupe: false,
  },
)

defineEmits<{ retour: [] }>()

const { t } = useI18n()

/** Un retour, une croix ou un retour-bouton : le titre monte sur la ligne, en 22. */
const secondaire = computed(() => Boolean(props.retour) || props.retourBouton)
/**
 * Le titre se pose sur la ligne des boutons, comme dans la maquette. Gardent leur ligne
 * à eux, sous la barre : le titre long (une question de FAQ), le terme, le surtitre, et
 * l'écran d'onglet qui porte l'avatar à gauche.
 */
const enLigne = computed(
  () =>
    !props.titreLong &&
    !props.terme &&
    !props.surtitre &&
    !props.avatarDuTitre &&
    (secondaire.value || !props.avatar),
)
const classesDuTitre = computed(() => ({
  'gn-entete__titre--terme': props.terme,
  'gn-entete__titre--long': props.titreLong,
}))
</script>

<template>
  <header class="gn-entete" :class="{ 'gn-entete--compact': compact, 'gn-entete--secondaire': secondaire }">
    <div class="gn-entete__barre">
      <GnBoutonRond v-if="retourBouton" picto="back" :libelle="t('gn-entete.retour')" @clic="$emit('retour')" />
      <GnBoutonRond
        v-else-if="retour"
        :vers="retour"
        :picto="fermer ? 'close' : 'back'"
        :libelle="t(fermer ? 'gn-entete.fermer' : 'gn-entete.retour')"
      />
      <GnAvatar
        v-else-if="avatar"
        :prenom="avatar.prenom"
        :nom="avatar.nom"
        :image="avatar.image"
        vers="/guide-nego/ressources/reglages"
      />
      <h1 v-if="compact" class="gn-entete__titre-compact">{{ titre }}</h1>
      <h1 v-else-if="enLigne" class="gn-entete__titre" :class="classesDuTitre" :lang="terme ? 'en' : undefined">{{ titre }}</h1>
      <span v-else class="gn-entete__vide" />
      <div class="gn-entete__actions">
        <!-- Un accès propre à l'écran, à côté de « Aa » : « Mon agenda » sur les sessions. -->
        <slot name="action" />
        <GnLoupe v-if="loupe" />
        <GnBoutonRond
          vers="/guide-nego/lexique"
          :actif="lexiqueOuvert"
          :libelle="lexiqueOuvert ? t('gn-entete.lexique-ouvert') : t('gn-entete.lexique')"
        >
          Aa
        </GnBoutonRond>
      </div>
    </div>
    <div v-if="!compact && avatarDuTitre" class="gn-entete__identite">
      <GnAvatar grand :prenom="avatarDuTitre.prenom" :nom="avatarDuTitre.nom" :image="avatarDuTitre.image" />
      <h1 class="gn-entete__titre">{{ titre }}</h1>
    </div>
    <p v-if="surtitre && !compact" class="gn-entete__surtitre">{{ surtitre }}</p>
    <h1 v-if="!compact && !avatarDuTitre && !enLigne" class="gn-entete__titre" :class="classesDuTitre" :lang="terme ? 'en' : undefined">
      {{ titre }}
    </h1>
    <div v-if="sousTitre && !compact && $slots['sous-titre-action']" class="gn-entete__ligne">
      <p class="gn-entete__sous-titre">{{ sousTitre }}</p>
      <!-- Une commande qui porte sur tout l'écran : « Tout marquer comme lu ». -->
      <slot name="sous-titre-action" />
    </div>
    <p v-else-if="sousTitre && !compact" class="gn-entete__sous-titre" :class="{ 'gn-entete__sous-titre--terme': terme }">{{ sousTitre }}</p>
    <!-- La ligne de connexion : « Synchronisé à » ou « Hors connexion ». -->
    <div v-if="!compact && $slots.connexion" class="gn-entete__connexion"><slot name="connexion" /></div>
    <slot name="pied" />
  </header>
</template>

<style>
/* Les marges de l'écran viennent de `.gn-ecran` (mesures.css) : l'en-tête n'en pose pas. */
[data-app="guide-nego"] .gn-entete {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-bottom: var(--gn-espace-20);
}

[data-app="guide-nego"] .gn-entete__barre {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  min-height: var(--gn-bouton-rond);
}

[data-app="guide-nego"] .gn-entete__vide {
  flex: 1;
}

[data-app="guide-nego"] .gn-entete__actions {
  flex: none;
  display: flex;
  gap: var(--gn-espace-8);
  margin-inline-start: auto;
}

[data-app="guide-nego"] .gn-entete__identite {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
}

/* Titre d'écran : Sora 28/700, approche −0,02em. */
[data-app="guide-nego"] .gn-entete__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
  letter-spacing: var(--gn-approche-28);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
  overflow-wrap: anywhere;
}

/* Écran d'onglet : le titre en bas de la barre, les boutons alignés sur sa dernière ligne. */
[data-app="guide-nego"] .gn-entete:not(.gn-entete--secondaire) .gn-entete__barre {
  align-items: flex-end;
}

[data-app="guide-nego"] .gn-entete__barre .gn-entete__titre {
  flex: 1;
  min-width: 0;
}

/* Titre d'écran secondaire, sur la ligne du retour : Sora 22/700. */
[data-app="guide-nego"] .gn-entete--secondaire .gn-entete__barre .gn-entete__titre {
  font-size: var(--gn-taille-22);
  line-height: var(--gn-interligne-22);
  letter-spacing: normal;
}

[data-app="guide-nego"] .gn-entete__titre--long {
  font-size: var(--gn-taille-22);
  line-height: var(--gn-interligne-22);
  letter-spacing: normal;
}

[data-app="guide-nego"] .gn-entete__titre--terme {
  font-style: italic;
}

[data-app="guide-nego"] .gn-entete--compact {
  padding-bottom: var(--gn-espace-12);
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

[data-app="guide-nego"] .gn-entete__surtitre {
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-entete__ligne {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  column-gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-entete__sous-titre {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-entete__sous-titre.gn-entete__sous-titre--terme {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-entete__connexion {
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  color: var(--gn-texte-2);
}
</style>

<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
import { adresseDeLaSortie, moduleOuvert, sortieDuVerrou } from '~/utils/guide-nego/verrou'

/**
 * Le verrou d'un module réservé — écran « 12 · 1k Sans code » de la maquette.
 *
 * **LA SORTIE SUIT LE MODE D'ADMISSION, PAS L'ÉCRAN QUI POSE LE VERROU.** Un
 * composant qui enverrait toujours vers la saisie du code proposerait, en mode
 * « approbation seule », un champ qui n'ouvre rien (FR-022) ; et une personne
 * qui attend déjà une réponse serait renvoyée saisir un code qu'elle n'a pas.
 * La décision vit dans `utils/guide-nego/verrou.ts`, pure et éprouvée.
 *
 * **L'ÉTAT VIENT DE `me/access`, JAMAIS D'UN ÉTAT RETENU CÔTÉ CLIENT** (FR-032).
 * `moduleOuvert` ne regarde que `state`, dérivé du RBAC par l'API : un `granted`
 * bricolé dans la garde locale n'ouvre rien.
 *
 * **DEUX SORTIES DÈS QU'UN COMPTE EXISTE, TROIS SANS COMPTE** (écart 39). La
 * maquette en dessine deux et suppose un compte ; sans compte, « Saisir mon code
 * d'invitation » mènerait à un écran qui redemande d'abord de s'en créer un
 * (FR-031). Le verrou enchaîne donc lui-même.
 *
 * **LA PHRASE ET LA LISTE SONT DONNÉES PAR L'ÉCRAN**, parce que la maquette en
 * porte quatre versions selon le module verrouillé — les Échanges annoncent des
 * canaux, une fiche de document annonce un téléchargement. Seuls le titre et
 * les sorties sont communs, et ils viennent de l'i18n.
 */

const props = withDefaults(
  defineProps<{
    /** La phrase qui dit ce que le code ouvre, propre à ce module. */
    propos: string
    /** Ce qui est fermé, une ligne par entrée. Vide : pas de liste bordée. */
    contenus?: string[]
    /** Ce qui reste ouvert sans code. */
    resteOuvert: string
    /** Le titre du verrou. Défaut : la formule imposée par le lexique. */
    titre?: string
    /** La sortie discrète. Défaut : « Continuer en visiteur ». */
    retour?: string
    retourVers?: string
  }>(),
  {
    contenus: () => [],
    titre: undefined,
    retour: undefined,
    retourVers: '/guide-nego',
  },
)

const { t } = useI18n()
const acces = useGnAcces()
const session = useGnSession()

onMounted(async () => {
  acces.relireAuRetourAuPremierPlan()
  await session.assurer()
  await acces.assurer()
})

/** Le module s'ouvre-t-il ? L'écran hôte s'en sert pour ne pas poser le verrou. */
const ouvert = computed(() => moduleOuvert(acces.acces.value))

const sortie = computed(() => sortieDuVerrou(acces.acces.value, session.connectee.value))
const sortieVers = computed(() => adresseDeLaSortie(sortie.value))
const libelleDeLaSortie = computed(() => t(`gn-verrou.sortie.${sortie.value}`))

/** Sans compte, la connexion prend la place de la sortie discrète (écart 39). */
const sansCompte = computed(() => sortie.value === 'compte')

const titreLu = computed(() => props.titre ?? t('gn-verrou.titre'))
const retourLu = computed(() => props.retour ?? t('gn-verrou.visiteur'))

const PICTO_DE_SORTIE: Record<string, NomDePicto> = {
  compte: 'user',
  code: 'lock',
  demande: 'send',
  attente: 'clock',
}
</script>

<template>
  <div class="gn-verrou">
    <span class="gn-verrou__cadenas" aria-hidden="true"><GnPicto nom="lock" :taille="20" /></span>
    <h2 class="gn-verrou__titre">{{ titreLu }}</h2>
    <p class="gn-verrou__propos">{{ propos }}</p>

    <!-- Ce qui est fermé. Absente quand le module n'a rien à énumérer : une
         fiche de document dit ce qu'elle cache dans sa phrase. -->
    <ul v-if="contenus.length > 0" class="gn-verrou__liste">
      <li v-for="contenu in contenus" :key="contenu" class="gn-verrou__contenu">
        <span class="gn-verrou__puce" aria-hidden="true"><GnPicto nom="lock" :taille="18" /></span>
        <span>{{ contenu }}</span>
      </li>
    </ul>

    <p class="gn-verrou__reste">{{ resteOuvert }}</p>

    <div class="gn-verrou__sorties">
      <GnBouton variante="principal" :vers="sortieVers" :picto="PICTO_DE_SORTIE[sortie]">
        {{ libelleDeLaSortie }}
      </GnBouton>

      <!-- Sans compte, l'enchaînement est ici : créer, ou se connecter, PUIS
           saisir le code (FR-031). -->
      <GnBouton v-if="sansCompte" variante="secondaire" vers="/guide-nego/connexion">
        {{ t('gn-verrou.sortie.connexion') }}
      </GnBouton>

      <GnBouton variante="discret" :vers="retourVers">{{ retourLu }}</GnBouton>
    </div>

    <!-- L'accès vient de s'ouvrir pendant qu'on lisait le verrou : rien ne
         doit laisser croire qu'il est encore fermé. -->
    <GnLigneInformation
      v-if="ouvert"
      :texte="t('gn-verrou.ouvert')"
      :sortie="t('gn-verrou.recharger')"
      :vers="retourVers"
    />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-verrou {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-verrou__cadenas {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  margin-block-end: var(--gn-espace-4);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-bloc-releve);
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-verrou__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-verrou__propos {
  max-width: var(--gn-mesure-lecture);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  color: var(--gn-texte-2);
}

/* La liste de la carte d'invitation (maquette 04) : ce que le code ouvrira. */
[data-app="guide-nego"] .gn-verrou__liste {
  width: 100%;
  margin: var(--gn-espace-8) 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  list-style: none;
}

[data-app="guide-nego"] .gn-verrou__contenu {
  display: flex;
  gap: var(--gn-espace-12);
  align-items: center;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-verrou__puce {
  flex: none;
  width: var(--gn-pastille-icone-petite);
  height: var(--gn-pastille-icone-petite);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-12);
  background: var(--gn-bloc-releve);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-verrou__reste {
  margin-block-start: var(--gn-espace-4);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-verrou__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  align-self: stretch;
  padding-top: var(--gn-espace-8);
}
</style>

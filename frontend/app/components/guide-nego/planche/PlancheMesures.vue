<script setup lang="ts">
import { useJetonsLus } from './PlancheSection.vue'

/**
 * Section 3 — les mesures, rendues à leur taille.
 *
 * Une liste de nombres ne dit rien : une cible de 48 px doit se voir comme un carré
 * de 48 px à côté de son nom. Chaque forme prend sa dimension par `var()` ; la valeur
 * écrite à côté est lue à l'exécution.
 */
type Forme = 'barre' | 'carre' | 'rayon' | 'filet'

const GROUPES: readonly { cle: string; forme: Forme; jetons: readonly string[] }[] = [
  {
    cle: 'espacements',
    forme: 'barre',
    jetons: [
      '--gn-grille',
      '--gn-espace-4',
      '--gn-espace-8',
      '--gn-espace-12',
      '--gn-espace-16',
      '--gn-espace-24',
      '--gn-espace-32',
      '--gn-espace-48',
      '--gn-marge-ecran',
      '--gn-entre-blocs',
      '--gn-ligne-air',
      '--gn-entre-paragraphes',
      '--gn-etiquette-air',
    ],
  },
  {
    cle: 'cibles',
    forme: 'carre',
    jetons: [
      '--gn-cible',
      '--gn-ligne-reglage',
      '--gn-onglet-filtre-hauteur',
      '--gn-filtre-hauteur',
      '--gn-reaction-pilule',
      '--gn-entre-cibles',
    ],
  },
  { cle: 'rayons', forme: 'rayon', jetons: ['--gn-rayon-0', '--gn-rayon-4', '--gn-rayon-24'] },
  {
    cle: 'filets',
    forme: 'filet',
    jetons: ['--gn-filet-1', '--gn-filet-2', '--gn-filet-3', '--gn-focus-anneau', '--gn-focus-decalage'],
  },
  {
    cle: 'zones',
    forme: 'barre',
    jetons: [
      '--gn-sur-haut-barre-etat',
      '--gn-sur-haut-avant-contenu',
      '--gn-sur-bas-barre-onglets',
      '--gn-sur-bas-geste',
      '--gn-action-principale-marge-bas',
    ],
  },
  {
    cle: 'cadre',
    forme: 'barre',
    jetons: [
      '--gn-cadre-largeur',
      '--gn-mesure-lecture',
      '--gn-colonne-largeur',
      '--gn-cadre-hauteur',
    ],
  },
  {
    cle: 'composants',
    forme: 'carre',
    jetons: [
      '--gn-colonne-heure',
      '--gn-barre-onglets',
      '--gn-onglet-picto',
      '--gn-onglet-min',
      '--gn-onglet-air',
      '--gn-compteur',
      '--gn-en-tete-titre',
      '--gn-en-tete-lecture',
      '--gn-bouton-aa',
      '--gn-avatar',
      '--gn-case',
      '--gn-cercle-choix',
      '--gn-interrupteur-largeur',
      '--gn-interrupteur-hauteur',
      '--gn-interrupteur-curseur',
      '--gn-carre-non-lu',
      '--gn-squelette-hauteur',
      '--gn-jauge',
      '--gn-segment-etape',
      '--gn-carre-etape',
      '--gn-champ',
      '--gn-zone-texte',
      '--gn-champ-heure',
      '--gn-feuille-poignee-largeur',
      '--gn-feuille-poignee-hauteur',
      '--gn-barre-lecture-repliee',
      '--gn-rail-alpha',
      '--gn-rail-alpha-interligne',
      '--gn-bande-jour-min',
      '--gn-bande-jour-hauteur',
      '--gn-picto-taille',
      '--gn-picto-marque',
      '--gn-picto-18',
      '--gn-picto-16',
      '--gn-picto-40',
    ],
  },
]

const MESURES = GROUPES.flatMap((groupe) => groupe.jetons)

const { t } = useI18n()
const racine = useTemplateRef<HTMLElement>('racine')
const lus = useJetonsLus(racine, [], MESURES)

const valeur = (jeton: string): string => lus.value[jeton] || '…'
const largeur = (jeton: string) => ({ width: `var(${jeton})` })
const cote = (jeton: string) => ({ width: `var(${jeton})`, height: `var(${jeton})` })
const rayon = (jeton: string) => ({ borderRadius: `var(${jeton})` })
/** Une valeur composée — l'air d'une étiquette, par exemple — ne se dessine pas comme une longueur. */
const longueurSeule = (jeton: string): boolean => !valeur(jeton).includes(' ')
const epaisseur = (jeton: string) => ({ borderTopWidth: `var(${jeton})` })
</script>

<template>
  <GnPlancheSection
    numero="3"
    :titre="t('gn-planche-mesures.titre')"
    :propos="t('gn-planche-mesures.propos')"
  >
    <div ref="racine" class="gn-planche-mesures">
      <p class="gn-planche-note">{{ t('gn-planche-section.lecture') }}</p>

      <GnPlancheSection
        v-for="groupe in GROUPES"
        :key="groupe.cle"
        :titre="t(`gn-planche-mesures.groupe.${groupe.cle}`)"
        :propos="t(`gn-planche-mesures.propos-groupe.${groupe.cle}`)"
      >
        <div
          v-if="groupe.forme === 'carre' || groupe.forme === 'rayon'"
          class="gn-planche-grille"
        >
          <div v-for="jeton in groupe.jetons" :key="jeton" class="gn-planche-mesures__carte">
            <div class="gn-planche-mesures__scene">
              <div
                v-if="groupe.forme === 'carre'"
                class="gn-planche-mesures__carre"
                :style="cote(jeton)"
              />
              <div v-else class="gn-planche-mesures__rayon" :style="rayon(jeton)" />
            </div>
            <span class="gn-planche-jeton">{{ jeton }}</span>
            <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
          </div>
        </div>

        <div v-else class="gn-planche-liste">
          <div v-for="jeton in groupe.jetons" :key="jeton" class="gn-planche-mesures__rangee">
            <span class="gn-planche-jeton">{{ jeton }}</span>
            <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
            <div v-if="groupe.forme === 'filet'" class="gn-planche-bande">
              <div class="gn-planche-mesures__filet" :style="epaisseur(jeton)" />
            </div>
            <div v-else-if="longueurSeule(jeton)" class="gn-planche-bande">
              <div class="gn-planche-mesures__barre" :style="largeur(jeton)" />
            </div>
          </div>
        </div>

        <p v-if="groupe.cle === 'cadre'" class="gn-planche-note">
          {{ t('gn-planche-mesures.cadre-note') }}
        </p>
        <p v-if="groupe.cle === 'composants'" class="gn-planche-note">
          {{ t('gn-planche-mesures.composants-note') }}
        </p>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-mesures {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-mesures__carte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  padding: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-mesures__scene {
  display: flex;
  align-items: flex-end;
  min-height: var(--gn-espace-48);
  margin-bottom: var(--gn-espace-4);
  overflow-x: auto;
}

[data-app="guide-nego"] .gn-planche-mesures__carre {
  flex: none;
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-planche-mesures__rayon {
  flex: none;
  width: var(--gn-cible);
  height: var(--gn-cible);
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-planche-mesures__rangee {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  padding-block: var(--gn-ligne-air);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-liste > .gn-planche-mesures__rangee:first-child {
  border-top: none;
}

[data-app="guide-nego"] .gn-planche-mesures__barre {
  height: var(--gn-espace-16);
  min-width: var(--gn-filet-1);
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-planche-mesures__filet {
  width: 100%;
  border-top-style: solid;
  border-top-color: var(--gn-filet-fort);
}
</style>

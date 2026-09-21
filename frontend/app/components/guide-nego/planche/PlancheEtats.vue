<script setup lang="ts">
import { NOMS_D_ETAT, type NomDEtat } from '~/utils/guide-nego/etats'

/**
 * Section 6 — les états de tous les objets, en clair et en sombre côte à côte.
 *
 * Les neuf familles de la maquette ne couvrent pas la table : la dixième est CALCULÉE,
 * et ramasse tout ce qui n'a pas été rangé. Ajouter un état à `etats.ts` le fait donc
 * paraître ici sans toucher à ce fichier — un état sans vitrine est un état que
 * personne ne vérifie.
 *
 * Le sombre se pose sur un conteneur imbriqué : les rôles sont redéfinis par
 * `[data-app="guide-nego"][data-theme="sombre"]`, donc les DEUX attributs doivent être
 * sur le même élément, et le cadre de l'application est neutralisé juste en dessous.
 */
const FAMILLES: readonly { cle: string; etats: readonly NomDEtat[] }[] = [
  { cle: 'session', etats: ['prevue', 'en-cours', 'deplacee', 'annulee', 'terminee', 'non-annoncee'] },
  { cle: 'acces', etats: ['ouverte', 'acces-limite'] },
  { cle: 'signalement', etats: ['envoye', 'valide', 'non-retenu'] },
  { cle: 'document', etats: ['a-jour', 'remplace', 'depasse', 'telecharge', 'reserve', 'lien-externe', 'nouveau'] },
  { cle: 'quiz', etats: ['relu', 'non-relu', 'en-relecture', 'reussi', 'a-refaire'] },
  { cle: 'connexion', etats: ['hors-connexion', 'synchronise', 'a-envoyer'] },
  { cle: 'importee', etats: ['source-officielle', 'traduction', 'lecture-impossible'] },
  { cle: 'faq', etats: ['verifie'] },
  { cle: 'inscription', etats: ['inscrite', 'liste-attente', 'complet', 'rediffusion'] },
]

const rangees = new Set(FAMILLES.flatMap((famille) => famille.etats))
const autres = NOMS_D_ETAT.filter((nom) => !rangees.has(nom))

const GROUPES = autres.length > 0 ? [...FAMILLES, { cle: 'autres', etats: autres }] : FAMILLES

/** Les états qui portent une donnée : le mot seul ne suffit pas à les montrer. */
const AVEC_DONNEE: readonly string[] = [
  'remplace',
  'synchronise',
  'lecture-impossible',
  'verifie',
  'rediffusion',
]

const SENS = [
  { cle: 'succes', jeton: '--gn-succes' },
  { cle: 'information', jeton: '--gn-information' },
  { cle: 'danger', jeton: '--gn-danger' },
  { cle: 'reseau', jeton: '--gn-reseau' },
  { cle: 'attention', jeton: '--gn-attention' },
  { cle: 'terminee', jeton: '--gn-terminee' },
] as const

const THEMES = ['clair', 'sombre'] as const

const { t } = useI18n()

const libelleDe = (etat: NomDEtat): string | undefined =>
  AVEC_DONNEE.includes(etat) ? t(`gn-planche-etats.libelle.${etat}`) : undefined

const aplat = (jeton: string) => ({ background: `var(${jeton})` })
</script>

<template>
  <GnPlancheSection
    numero="6"
    :titre="t('gn-planche-etats.titre')"
    :propos="t('gn-planche-etats.propos')"
  >
    <div class="gn-planche-etats">
      <p class="gn-planche-note">{{ t('gn-planche-etats.regle') }}</p>
      <p class="gn-planche-note">{{ t('gn-planche-etats.compte', { nombre: NOMS_D_ETAT.length }) }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-etats.sens')"
        :propos="t('gn-planche-etats.sens-propos')"
      >
        <ul class="gn-planche-grille gn-planche-grille--large">
          <li v-for="couleur in SENS" :key="couleur.cle" class="gn-planche-echantillon">
            <span class="gn-planche-echantillon__aplat" :style="aplat(couleur.jeton)" />
            <span class="gn-planche-legende">
              <span class="gn-planche-jeton">{{ couleur.jeton }}</span>
              <span class="gn-planche-valeur">{{ t(`gn-planche-etats.sens-de.${couleur.cle}`) }}</span>
            </span>
          </li>
        </ul>
      </GnPlancheSection>

      <GnPlancheSection
        v-for="groupe in GROUPES"
        :key="groupe.cle"
        :titre="t(`gn-planche-etats.famille.${groupe.cle}`)"
      >
        <div class="gn-planche-etats__paire">
          <div
            v-for="theme in THEMES"
            :key="theme"
            class="gn-planche-etats__theme"
            data-app="guide-nego"
            :data-theme="theme"
          >
            <span class="gn-planche-etats__nom">{{ t(`gn-planche-etats.theme.${theme}`) }}</span>
            <ul class="gn-planche-etats__rangee">
              <li v-for="etat in groupe.etats" :key="etat">
                <GnMarqueEtat :etat="etat" :libelle="libelleDe(etat)" />
              </li>
            </ul>
          </div>
        </div>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-etats {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-etats__paire {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: var(--gn-espace-8);
}

/* Le conteneur porte `data-app` pour que le thème sombre s'y applique : il hérite donc
   aussi du cadre de l'application — pleine hauteur, colonne centrée —, que ces quatre
   déclarations reprennent. Sans elles, chaque famille ferait un écran de haut. */
[data-app="guide-nego"] .gn-planche-etats__theme {
  min-height: 0;
  width: auto;
  overflow-x: visible;
  padding: var(--gn-espace-12);
  gap: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-etats__nom {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-planche-etats__rangee {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8) var(--gn-espace-16);
}
</style>

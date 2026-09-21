import type { NomDePicto } from './pictogrammes'

/**
 * Les états de tous les objets : un pictogramme, un mot, une couleur.
 *
 * La couleur ne figure pas dans cette table : le jeton se déduit du nom de l'état
 * (`--gn-etat-<nom>`, tous déclarés dans `theme.css`). Le mot vit dans
 * `i18n/locales/{fr,en}/components/gn-marque-etat.json`, sous la même clé — un
 * appelant ne peut donc pas écrire « Annulée » d'une couleur qui dit autre chose.
 */
export interface DessinDEtat {
  /** Nom de pictogramme, sans le préfixe `gn-`. `null` pour « Nouveau », qui porte le carré non-lu. */
  picto: NomDePicto | null
  taille?: 16 | 18
}

export const DESSINS_D_ETAT = {
  /* Session de négociation */
  prevue: { picto: 'circle' },
  'en-cours': { picto: 'live' },
  deplacee: { picto: 'moved' },
  /* Écart 20 : la croix cerclée dit « annulé » ; le triangle est réservé à l'erreur. */
  annulee: { picto: 'x-circle' },
  terminee: { picto: 'check' },
  'non-annoncee': { picto: 'diamond' },

  /* Accès à une session */
  ouverte: { picto: 'unlock', taille: 18 },
  'acces-limite': { picto: 'lock', taille: 18 },

  /* Signalement */
  envoye: { picto: 'send' },
  valide: { picto: 'check-circle' },
  'non-retenu': { picto: 'x-circle' },

  /* Document */
  'a-jour': { picto: 'check' },
  remplace: { picto: 'refresh' },
  depasse: { picto: 'warn' },
  telecharge: { picto: 'download' },
  reserve: { picto: 'lock', taille: 18 },
  'lien-externe': { picto: 'external' },
  nouveau: { picto: null },

  /* Classement d'un message */
  propose: { picto: 'clock' },
  classe: { picto: 'check' },

  /* Quiz — écart 19 : le bouclier coché dit « validé par un expert » */
  relu: { picto: 'shield-check' },
  'non-relu': { picto: 'shield' },
  'en-relecture': { picto: 'send' },
  reussi: { picto: 'check-circle' },
  'a-refaire': { picto: 'refresh' },

  /* Connexion */
  'hors-connexion': { picto: 'wifi-off', taille: 18 },
  synchronise: { picto: 'sync', taille: 16 },
  'a-envoyer': { picto: 'clock' },

  /* Donnée importée — écart 20 : le triangle rouge dit l'erreur de lecture */
  'source-officielle': { picto: 'check-circle' },
  traduction: { picto: 'translate' },
  'lecture-impossible': { picto: 'warn' },
  verifie: { picto: 'shield-check' },

  /* Inscription */
  inscrite: { picto: 'check-circle' },
  'liste-attente': { picto: 'clock' },
  complet: { picto: 'x-circle' },
  rediffusion: { picto: 'play' },

  /* Question à un expert */
  'en-attente': { picto: 'clock', taille: 18 },
  repondue: { picto: 'check' },
  'ajoutee-faq': { picto: 'quiz' },

  /* Agenda */
  chevauche: { picto: 'warn' },

  /* Personnes et assistant */
  expert: { picto: 'shield-check', taille: 16 },
  'a-confirmer': { picto: 'shield' },
} as const satisfies Record<string, DessinDEtat>

export type NomDEtat = keyof typeof DESSINS_D_ETAT

export const NOMS_D_ETAT = Object.keys(DESSINS_D_ETAT) as NomDEtat[]

/** Le type de retour élargit la table littérale : sans lui, `taille` n'existe que sur certains états. */
export function dessinDEtat(nom: NomDEtat): DessinDEtat {
  return DESSINS_D_ETAT[nom]
}

/** Le rôle porte la couleur ; le nom de l'état suffit à la nommer. */
export function couleurDEtat(nom: NomDEtat): string {
  return `var(--gn-etat-${nom})`
}

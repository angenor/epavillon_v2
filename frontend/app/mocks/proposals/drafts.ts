/**
 * Les cinq dossiers à l'état de BROUILLON.
 *
 * C'est ce que lit l'espace organisation (A5) : des dossiers incomplets, sans
 * `submitted_at`, sans note, que leur auteur peut encore modifier ou supprimer.
 * Les champs facultatifs y sont volontairement laissés vides — c'est la
 * situation réelle, et elle met à l'épreuve les écrans qui supposeraient qu'un
 * résumé ou un public visé est toujours renseigné.
 *
 * `COP31-00040` porte un titre à peine ébauché et rien d'autre : le cas du
 * dossier ouvert dix minutes avant une réunion, et jamais repris.
 */

import type { Proposal } from '~/types/programme/proposal'
import { COUNTRY, ORG, PERSON, PROPOSAL } from '../ids'
import { proposal } from './_shared'

export const draftProposals = [
  proposal({
    id: PROPOSAL.lacTchad,
    ref: 36,
    organization: ORG.anteb,
    submittedBy: PERSON.zinsou,
    title: {
      fr: 'Adapter les zones humides du lac Tchad au recul des eaux',
      en: 'Adapting Lake Chad wetlands to receding waters',
    },
    slug: 'zones-humides-lac-tchad',
    summary: {
      fr: "Retour sur cinq ans de restauration des mares de décrue et de gestion partagée de la ressource entre éleveurs et pêcheurs.",
    },
    objectives: {
      fr: "Présenter les résultats du programme de restauration des mares de décrue et les modalités de gestion partagée retenues avec les communautés riveraines.",
    },
    presentation: {
      fr: "Le lac Tchad a perdu une part considérable de sa surface en cinquante ans. Le programme conduit avec les communes riveraines a restauré une quarantaine de mares de décrue et mis en place des comités locaux réunissant éleveurs, agriculteurs et pêcheurs. La session présentera les résultats hydrologiques mesurés, les conflits d'usage évités et ce qui reste à consolider.",
    },
    format: 'in_person',
    category: 'field_project',
    country: COUNTRY.td,
    duration: 90,
    status: 'draft',
    createdAt: '2026-08-10T14:20:00Z',
    updatedAt: '2026-08-14T09:05:00Z',
  }),
  proposal({
    id: PROPOSAL.batiScolaire,
    ref: 37,
    organization: ORG.fhrc,
    submittedBy: PERSON.josephPierre,
    title: {
      fr: 'Financer des écoles résistantes aux cyclones',
      en: 'Financing cyclone-resistant schools',
    },
    slug: 'financer-ecoles-resistantes-cyclones',
    objectives: {
      fr: "Montrer comment un fonds de proximité finance la mise aux normes du bâti scolaire sans passer par un intermédiaire international.",
    },
    presentation: {
      fr: "Vingt-deux écoles ont été renforcées en trois ans dans le Sud d'Haïti, pour un coût moyen inférieur au tiers des devis internationaux. La session détaillera le montage financier, le recours aux entreprises locales et le suivi de chantier assuré par les comités de parents.",
    },
    format: 'hybrid',
    category: 'field_project',
    country: COUNTRY.ht,
    duration: 60,
    status: 'draft',
    createdAt: '2026-08-12T16:40:00Z',
  }),
  proposal({
    id: PROPOSAL.corridorsEcologiques,
    ref: 38,
    organization: ORG.cofemac,
    submittedBy: PERSON.ngoBassong,
    title: {
      fr: 'Corridors écologiques transfrontaliers du bassin du Congo',
      en: 'Transboundary ecological corridors in the Congo Basin',
    },
    slug: 'corridors-ecologiques-bassin-congo',
    summary: {
      fr: "Trois pays, un corridor, et la question jamais réglée du partage des bénéfices avec les communautés forestières.",
    },
    objectives: {
      fr: "Confronter les modèles de gouvernance des corridors transfrontaliers et la place réelle qu'y tiennent les communautés forestières.",
    },
    presentation: {
      fr: "Le corridor reliant trois aires protégées du bassin du Congo est présenté comme un succès de conservation. La session propose d'en examiner l'envers : les accords de partage des bénéfices, leur mise en œuvre effective, et les arbitrages fonciers qui n'ont pas été tranchés.",
    },
    format: 'in_person',
    category: 'concertation',
    country: COUNTRY.cm,
    duration: 90,
    status: 'draft',
    createdAt: '2026-08-13T11:15:00Z',
  }),
  proposal({
    id: PROPOSAL.journalismeClimatique,
    ref: 39,
    organization: ORG.mvoi,
    submittedBy: PERSON.rakotomalala,
    title: {
      fr: 'Former les rédactions francophones au traitement du climat',
      en: 'Training French-speaking newsrooms on climate coverage',
    },
    slug: 'former-redactions-francophones-climat',
    objectives: {
      fr: "Outiller les journalistes des rédactions francophones du Sud pour couvrir une conférence des Parties sans dépendre des dépêches d'agence.",
    },
    presentation: {
      fr: "Atelier pratique : lire un texte de négociation, identifier ce qui se joue réellement dans une session de contact, vérifier un chiffre d'émissions, et écrire pour un public qui n'a pas suivi les dix dernières COP.",
    },
    format: 'hybrid',
    category: 'capacity_building',
    country: COUNTRY.mg,
    duration: 120,
    status: 'draft',
    createdAt: '2026-08-15T07:50:00Z',
  }),
  proposal({
    // Dossier à peine ouvert : titre provisoire, un objectif d'une ligne, rien
    // d'autre. L'espace organisation doit l'afficher sans se plaindre.
    id: PROPOSAL.comptabiliteCarbone,
    ref: 40,
    organization: ORG.verdeo,
    submittedBy: PERSON.moreau,
    title: { fr: 'Comptabilité carbone des collectivités locales' },
    slug: 'comptabilite-carbone-collectivites',
    objectives: { fr: "À compléter — présenter la méthode et deux cas de collectivités." },
    presentation: { fr: "À rédiger." },
    format: 'online',
    category: null,
    duration: null,
    status: 'draft',
    createdAt: '2026-08-16T08:05:00Z',
  }),
] satisfies Proposal[]

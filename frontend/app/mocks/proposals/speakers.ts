/**
 * Données simulées de `programme.proposal_speakers` — les intervenants annoncés
 * au dépôt du dossier.
 *
 * L'INTERVENANT EST UNE PERSONNE (`identity.people`), pas un bloc de texte
 * recopié : la v1 réécrivait nom, prénom, courriel et photo à chaque activité,
 * et une même personne y existait sous quatre orthographes. Ce qui est recopié
 * ici, volontairement, c'est la FONCTION et l'ORGANISATION AU MOMENT de
 * l'activité (`*_snapshot`) : une personne change d'employeur, l'archive d'une
 * COP passée ne doit pas être réécrite pour autant.
 *
 * Trois états de confirmation coexistent, et les trois doivent s'afficher :
 * confirmé, invitation envoyée sans réponse, et pas encore sollicité.
 *
 * Les intervenants du JOUR — ceux réellement présents — vivent dans
 * `mocks/sessions/speakers.ts` : ils sont recopiés depuis le dossier à la
 * programmation, puis modifiés. Ce ne sont pas toujours les mêmes.
 */

import type { ProposalSpeaker, SpeakerRole } from '~/types/programme/proposal'
import { ORG, PERSON, PROPOSAL, PROPOSAL_SPEAKER } from '../ids'

interface SpeakerFields {
  role?: SpeakerRole
  job: string
  org: string
  organizationId?: string | null
  bio?: { fr: string; en?: string } | null
  /** `null` : invitation envoyée, sans réponse à ce jour. */
  confirmedAt?: string | null
  sentAt?: string | null
  questions?: boolean
}

function speaker(
  n: number,
  proposal_id: string,
  person_id: string,
  sort_order: number,
  fields: SpeakerFields,
): ProposalSpeaker {
  return {
    id: PROPOSAL_SPEAKER(n),
    proposal_id,
    person_id,
    role: fields.role ?? 'speaker',
    job_title_snapshot: fields.job,
    organization_snapshot: fields.org,
    organization_id: fields.organizationId ?? null,
    bio: fields.bio ?? null,
    confirmed_at: fields.confirmedAt ?? null,
    confirmation_sent_at: fields.sentAt ?? null,
    is_available_for_questions: fields.questions ?? true,
    sort_order,
    created_at: '2026-06-20T10:00:00Z',
  }
}

export const proposalSpeakers = [
  // 1. Financer l'adaptation côtière
  speaker(1, PROPOSAL.adaptationCotiere, PERSON.sowFall, 10, {
    role: 'moderator',
    job: 'Directrice exécutive',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    organizationId: ORG.roac,
    bio: {
      fr: "Anime le réseau des associations littorales d'Afrique de l'Ouest et suit les dossiers d'adaptation depuis 2019.",
    },
    confirmedAt: '2026-06-06T09:00:00Z',
    sentAt: '2026-06-03T10:00:00Z',
  }),
  speaker(2, PROPOSAL.adaptationCotiere, PERSON.mbayeNdiaye, 20, {
    job: 'Chargé de projet littoral',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    organizationId: ORG.roac,
    confirmedAt: '2026-06-06T09:30:00Z',
    sentAt: '2026-06-03T10:00:00Z',
  }),
  speaker(3, PROPOSAL.adaptationCotiere, PERSON.zinsou, 30, {
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    organizationId: ORG.anteb,
    confirmedAt: '2026-06-08T11:00:00Z',
    sentAt: '2026-06-03T10:05:00Z',
  }),
  speaker(4, PROPOSAL.adaptationCotiere, PERSON.duchesne, 40, {
    role: 'panelist',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-06-09T08:15:00Z',
    sentAt: '2026-06-03T10:10:00Z',
  }),

  // 2. Alerte précoce
  speaker(10, PROPOSAL.alertePrecoce, PERSON.ouedraogo, 10, {
    role: 'keynote',
    job: 'Directeur',
    org: "Observatoire du Sahel pour l'énergie durable",
    organizationId: ORG.osed,
    confirmedAt: '2026-06-10T07:45:00Z',
    sentAt: '2026-06-08T09:00:00Z',
  }),
  speaker(11, PROPOSAL.alertePrecoce, PERSON.kabore, 20, {
    job: "Cheffe du département accès à l'énergie",
    org: "Observatoire du Sahel pour l'énergie durable",
    organizationId: ORG.osed,
    confirmedAt: '2026-06-10T08:10:00Z',
    sentAt: '2026-06-08T09:00:00Z',
  }),
  speaker(12, PROPOSAL.alertePrecoce, PERSON.compaore, 30, {
    job: 'Chargée de projets',
    org: 'OSED',
    // Rattachée à la FICHE EN DOUBLON : la même personne, la même structure,
    // deux fiches. C'est ce que la fusion (A11) doit réconcilier.
    organizationId: ORG.osedSigle,
    confirmedAt: null,
    sentAt: '2026-06-08T09:05:00Z',
  }),

  // 3. Mini-réseaux solaires
  speaker(20, PROPOSAL.miniReseaux, PERSON.kabore, 10, {
    role: 'moderator',
    job: "Cheffe du département accès à l'énergie",
    org: "Observatoire du Sahel pour l'énergie durable",
    organizationId: ORG.osed,
    confirmedAt: '2026-06-12T10:00:00Z',
    sentAt: '2026-06-10T14:00:00Z',
  }),
  speaker(21, PROPOSAL.miniReseaux, PERSON.moreau, 20, {
    job: 'Associé fondateur',
    org: 'Verdéo Solutions',
    organizationId: ORG.verdeo,
    confirmedAt: '2026-06-13T08:00:00Z',
    sentAt: '2026-06-10T14:05:00Z',
  }),
  speaker(22, PROPOSAL.miniReseaux, PERSON.ilboudo, 30, {
    role: 'panelist',
    job: 'Gestionnaire de mini-réseau, coopérative de Boromo',
    org: 'Coopérative d’électrification de Boromo',
    organizationId: null,
    // Personne sans organisation rattachée : le formulaire doit l'accepter.
    confirmedAt: null,
    sentAt: '2026-06-11T09:00:00Z',
    questions: false,
  }),

  // 4. Agroécologie
  speaker(30, PROPOSAL.agroecologie, PERSON.zinsou, 10, {
    role: 'moderator',
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    organizationId: ORG.anteb,
    confirmedAt: '2026-06-14T09:00:00Z',
    sentAt: '2026-06-12T08:00:00Z',
  }),
  speaker(31, PROPOSAL.agroecologie, PERSON.ngoBassong, 20, {
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: '2026-06-15T11:30:00Z',
    sentAt: '2026-06-12T08:05:00Z',
  }),

  // 5. Pertes et préjudices
  speaker(40, PROPOSAL.pertesPrejudices, PERSON.josephPierre, 10, {
    role: 'moderator',
    job: 'Responsable des programmes',
    org: 'Fonds haïtien pour la résilience communautaire',
    organizationId: ORG.fhrc,
    bio: {
      fr: "Suit depuis trois ans les dossiers de reconstruction post-cyclone financés par des fonds de proximité.",
    },
    confirmedAt: '2026-06-16T10:00:00Z',
    sentAt: '2026-06-13T15:00:00Z',
  }),
  speaker(41, PROPOSAL.pertesPrejudices, PERSON.sowFall, 20, {
    job: 'Directrice exécutive',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    organizationId: ORG.roac,
    confirmedAt: '2026-06-16T14:20:00Z',
    sentAt: '2026-06-13T15:05:00Z',
  }),
  speaker(42, PROPOSAL.pertesPrejudices, PERSON.lemoine, 30, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    organizationId: ORG.cudcm,
    bio: {
      fr: "Travaille sur la qualification juridique des pertes non économiques et sur les régimes de réparation.",
      en: 'Works on the legal qualification of non-economic loss and on reparation regimes.',
    },
    confirmedAt: '2026-06-17T08:40:00Z',
    sentAt: '2026-06-13T15:10:00Z',
  }),
  speaker(43, PROPOSAL.pertesPrejudices, PERSON.ngoBassong, 40, {
    role: 'panelist',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: '2026-06-17T12:00:00Z',
    sentAt: '2026-06-13T15:15:00Z',
  }),

  // 6. Transition juste
  speaker(50, PROPOSAL.transitionJuste, PERSON.ngoBassong, 10, {
    role: 'moderator',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: '2026-06-18T09:00:00Z',
    sentAt: '2026-06-16T10:00:00Z',
  }),
  speaker(51, PROPOSAL.transitionJuste, PERSON.koffi, 20, {
    job: 'Président',
    org: 'Union des jeunes francophones pour le climat',
    organizationId: ORG.ujfc,
    confirmedAt: '2026-06-19T15:30:00Z',
    sentAt: '2026-06-16T10:05:00Z',
  }),

  // 7. Rapports biennaux — atelier
  speaker(60, PROPOSAL.rapportsBiennaux, PERSON.tremblay, 10, {
    role: 'facilitator',
    job: 'Chargé de programme énergie et climat',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-06-20T09:00:00Z',
    sentAt: '2026-06-18T11:00:00Z',
  }),
  speaker(61, PROPOSAL.rapportsBiennaux, PERSON.nkoDiop, 20, {
    role: 'facilitator',
    job: 'Programmatrice',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-06-20T09:10:00Z',
    sentAt: '2026-06-18T11:00:00Z',
  }),

  // 8. Villes du Mékong
  speaker(70, PROPOSAL.villesMekong, PERSON.tranVanMinh, 10, {
    role: 'moderator',
    job: 'Secrétaire général',
    org: 'Consortium des villes durables du Mékong francophone',
    organizationId: ORG.cvdmf,
    confirmedAt: '2026-06-23T03:30:00Z',
    sentAt: '2026-06-20T04:00:00Z',
  }),
  speaker(71, PROPOSAL.villesMekong, PERSON.elFassi, 20, {
    role: 'panelist',
    job: 'Chercheuse principale',
    org: "Institut méditerranéen de recherche sur l'eau",
    organizationId: ORG.imre,
    confirmedAt: null,
    sentAt: '2026-06-20T04:05:00Z',
  }),

  // 9. Contentieux climatique
  speaker(80, PROPOSAL.contentieuxClimatique, PERSON.lemoine, 10, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    organizationId: ORG.cudcm,
    confirmedAt: '2026-06-25T14:00:00Z',
    sentAt: '2026-06-22T09:00:00Z',
  }),
  speaker(81, PROPOSAL.contentieuxClimatique, PERSON.gagnon, 20, {
    role: 'moderator',
    job: 'Coordonnatrice de la chaire',
    org: 'Chaire universitaire de droit climatique de Montréal',
    organizationId: ORG.cudcm,
    confirmedAt: '2026-06-25T14:10:00Z',
    sentAt: '2026-06-22T09:00:00Z',
  }),

  // 10. Relève jeunesse
  speaker(90, PROPOSAL.releveJeunesse, PERSON.koffi, 10, {
    role: 'moderator',
    job: 'Président',
    org: 'Union des jeunes francophones pour le climat',
    organizationId: ORG.ujfc,
    confirmedAt: '2026-06-27T10:00:00Z',
    sentAt: '2026-06-25T12:00:00Z',
  }),
  speaker(91, PROPOSAL.releveJeunesse, PERSON.nkoDiop, 20, {
    role: 'panelist',
    job: 'Programmatrice',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-06-27T11:20:00Z',
    sentAt: '2026-06-25T12:05:00Z',
  }),
  speaker(92, PROPOSAL.releveJeunesse, PERSON.ilboudo, 30, {
    job: 'Jeune délégué, promotion 2026',
    org: 'Union des jeunes francophones pour le climat',
    organizationId: ORG.ujfc,
    confirmedAt: null,
    sentAt: '2026-06-25T12:10:00Z',
  }),

  // 11. Mangroves
  speaker(100, PROPOSAL.mangroves, PERSON.ngoBassong, 10, {
    role: 'moderator',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: '2026-06-28T09:00:00Z',
    sentAt: '2026-06-26T08:00:00Z',
  }),
  speaker(101, PROPOSAL.mangroves, PERSON.mbayeNdiaye, 20, {
    job: 'Chargé de projet littoral',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    organizationId: ORG.roac,
    confirmedAt: '2026-06-29T07:30:00Z',
    sentAt: '2026-06-26T08:05:00Z',
  }),

  // 12. Bassin du Niger
  speaker(110, PROPOSAL.bassinNiger, PERSON.benAmor, 10, {
    role: 'keynote',
    job: 'Directeur de recherche',
    org: "Institut méditerranéen de recherche sur l'eau",
    organizationId: ORG.imre,
    confirmedAt: '2026-06-30T09:40:00Z',
    sentAt: '2026-06-28T10:00:00Z',
  }),
  speaker(111, PROPOSAL.bassinNiger, PERSON.elFassi, 20, {
    role: 'moderator',
    job: 'Chercheuse principale',
    org: "Institut méditerranéen de recherche sur l'eau",
    organizationId: ORG.imre,
    confirmedAt: '2026-06-30T09:45:00Z',
    sentAt: '2026-06-28T10:00:00Z',
  }),

  // 13. Article 6
  speaker(120, PROPOSAL.article6, PERSON.lemoine, 10, {
    role: 'keynote',
    job: 'Professeure de droit international de l’environnement',
    org: 'Chaire universitaire de droit climatique de Montréal',
    organizationId: ORG.cudcm,
    confirmedAt: '2026-07-01T13:00:00Z',
    sentAt: '2026-06-29T15:00:00Z',
  }),
  speaker(121, PROPOSAL.article6, PERSON.duchesne, 20, {
    role: 'panelist',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-07-01T13:30:00Z',
    sentAt: '2026-06-29T15:05:00Z',
  }),
  speaker(122, PROPOSAL.article6, PERSON.gagnon, 30, {
    role: 'moderator',
    job: 'Coordonnatrice de la chaire',
    org: 'Chaire universitaire de droit climatique de Montréal',
    organizationId: ORG.cudcm,
    confirmedAt: '2026-07-01T13:35:00Z',
    sentAt: '2026-06-29T15:05:00Z',
  }),

  // 14. Accès au Fonds vert
  speaker(130, PROPOSAL.accesFondsVert, PERSON.duchesne, 10, {
    role: 'facilitator',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: '2026-07-02T08:00:00Z',
    sentAt: '2026-06-30T09:00:00Z',
  }),
  speaker(131, PROPOSAL.accesFondsVert, PERSON.josephPierre, 20, {
    job: 'Responsable des programmes',
    org: 'Fonds haïtien pour la résilience communautaire',
    organizationId: ORG.fhrc,
    confirmedAt: '2026-07-02T14:15:00Z',
    sentAt: '2026-06-30T09:05:00Z',
  }),
  speaker(132, PROPOSAL.accesFondsVert, PERSON.zinsou, 30, {
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    organizationId: ORG.anteb,
    confirmedAt: '2026-07-03T07:50:00Z',
    sentAt: '2026-06-30T09:10:00Z',
  }),

  // 15. Chaleur et hôpitaux
  speaker(140, PROPOSAL.chaleurHopitaux, PERSON.benAmor, 10, {
    role: 'keynote',
    job: 'Directeur de recherche',
    org: "Institut méditerranéen de recherche sur l'eau",
    organizationId: ORG.imre,
    confirmedAt: '2026-07-04T10:00:00Z',
    sentAt: '2026-07-02T11:00:00Z',
  }),

  // 16. Pastoralisme
  speaker(150, PROPOSAL.pastoralisme, PERSON.sowFall, 10, {
    role: 'moderator',
    job: 'Directrice exécutive',
    org: "Réseau ouest-africain pour l'adaptation côtière",
    organizationId: ORG.roac,
    confirmedAt: '2026-07-05T09:20:00Z',
    sentAt: '2026-07-03T14:00:00Z',
  }),
  speaker(151, PROPOSAL.pastoralisme, PERSON.compaore, 20, {
    job: 'Chargée de projets',
    org: 'OSED',
    organizationId: ORG.osedSigle,
    confirmedAt: null,
    sentAt: '2026-07-03T14:05:00Z',
  }),

  // 20. Budgets sensibles au genre — dossier en cours d'évaluation
  speaker(160, PROPOSAL.budgetsGenre, PERSON.ngoBassong, 10, {
    role: 'moderator',
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: '2026-07-01T08:00:00Z',
    sentAt: '2026-06-28T09:00:00Z',
  }),
  speaker(161, PROPOSAL.budgetsGenre, PERSON.duchesne, 20, {
    role: 'panelist',
    job: 'Expert finance climatique',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: null,
    sentAt: '2026-06-28T09:05:00Z',
  }),

  // 21. Cartographie de Cotonou
  speaker(170, PROPOSAL.cartographieCotonou, PERSON.zinsou, 10, {
    job: 'Directeur de la transition écologique',
    org: "Agence nationale de la transition écologique du Bénin",
    organizationId: ORG.anteb,
    confirmedAt: '2026-07-03T09:00:00Z',
    sentAt: '2026-07-01T10:00:00Z',
  }),

  // 22. Interprétation — aucun intervenant confirmé à ce jour
  speaker(180, PROPOSAL.interpretation, PERSON.tremblay, 10, {
    role: 'moderator',
    job: 'Chargé de programme énergie et climat',
    org: 'Institut de la Francophonie pour le développement durable',
    organizationId: ORG.ifdd,
    confirmedAt: null,
    sentAt: null,
  }),

  // 24. Assurance paramétrique
  speaker(190, PROPOSAL.assuranceParametrique, PERSON.ouedraogo, 10, {
    role: 'keynote',
    job: 'Directeur',
    org: "Observatoire du Sahel pour l'énergie durable",
    organizationId: ORG.osed,
    confirmedAt: '2026-07-14T08:30:00Z',
    sentAt: '2026-07-10T09:00:00Z',
  }),
  speaker(191, PROPOSAL.assuranceParametrique, PERSON.kabore, 20, {
    role: 'moderator',
    job: "Cheffe du département accès à l'énergie",
    org: "Observatoire du Sahel pour l'énergie durable",
    organizationId: ORG.osed,
    confirmedAt: '2026-07-14T08:35:00Z',
    sentAt: '2026-07-10T09:00:00Z',
  }),

  // 28. Transport fluvial — dossier tout juste déposé
  speaker(200, PROPOSAL.transportFluvial, PERSON.ngoBassong, 10, {
    job: 'Coordonnatrice générale',
    org: 'Coalition des femmes pour le climat en Afrique centrale',
    organizationId: ORG.cofemac,
    confirmedAt: null,
    sentAt: '2026-08-04T10:00:00Z',
  }),
] satisfies ProposalSpeaker[]

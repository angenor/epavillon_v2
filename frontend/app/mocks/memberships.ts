/**
 * Données simulées de `org.memberships` — qui appartient à quelle organisation.
 *
 * Fichier séparé de `org.ts` pour une raison de volume : les adhésions
 * croissent avec le nombre de personnes, pas avec le nombre d'organisations.
 *
 * Quatre situations sont représentées, parce que l'espace organisation (A5) et
 * la modération des rattachements (A2) doivent les traiter toutes :
 *   - adhésion ACTIVE, approuvée par un référent ;
 *   - DEMANDE en attente, qu'un référent doit accepter ou refuser ;
 *   - INVITATION en attente, que la personne invitée doit accepter ;
 *   - adhésion RÉVOQUÉE, conservée pour l'historique.
 *
 * LES DEUX ATTENTES SONT L'INVERSE L'UNE DE L'AUTRE et le modèle les distingue
 * depuis le 17/08 : `invited_at` renseigné, l'organisation a invité et attend la
 * personne ; `invited_at` nul, la personne a demandé et attend un référent. Le
 * même statut `pending` recouvrait les deux, et un référent aurait approuvé sa
 * propre invitation — donnant une adhésion active à quelqu'un qui n'a rien
 * accepté.
 *
 * `is_primary` est posé par la base à la première adhésion active : il désigne
 * le rattachement affiché à côté du nom de la personne.
 */

import type { Membership } from '~/types/org'
import { MEMBERSHIP, ORG, PERSON } from './ids'

/** Raccourci d'écriture : une adhésion active et approuvée. */
function membership(
  n: number,
  organization_id: string,
  person_id: string,
  role: Membership['role'],
  job_title: string | null,
  created_at: string,
  options: Partial<
    Pick<
      Membership,
      'status' | 'is_primary' | 'approved_by' | 'approved_at' | 'revoked_at' | 'invited_by' | 'invited_at'
    >
  > = {},
): Membership {
  const status = options.status ?? 'active'
  return {
    id: MEMBERSHIP(n),
    organization_id,
    person_id,
    role,
    status,
    is_primary: options.is_primary ?? status === 'active',
    job_title,
    invited_by: options.invited_by ?? null,
    invited_at: options.invited_at ?? null,
    approved_by: options.approved_by ?? (status === 'pending' ? null : PERSON.adminPivot),
    approved_at: options.approved_at ?? (status === 'pending' ? null : created_at),
    revoked_at: options.revoked_at ?? null,
    created_at,
    updated_at: options.revoked_at ?? created_at,
  }
}

export const memberships = [
  // IFDD
  membership(1, ORG.ifdd, PERSON.adminPivot, 'manager', 'Compte technique', '2026-01-12T09:00:00Z'),
  membership(2, ORG.ifdd, PERSON.bakayoko, 'manager', 'Responsable de la programmation', '2026-01-14T08:40:00Z'),
  membership(3, ORG.ifdd, PERSON.tremblay, 'manager', 'Chargé de programme énergie et climat', '2026-01-14T08:45:00Z'),
  membership(4, ORG.ifdd, PERSON.perretAdmin, 'member', 'Coordonnatrice de la COP31', '2026-02-02T09:05:00Z'),
  membership(5, ORG.ifdd, PERSON.nkoDiop, 'member', 'Programmatrice', '2026-02-05T11:25:00Z'),
  membership(6, ORG.ifdd, PERSON.duchesne, 'member', 'Expert finance climatique', '2026-03-16T14:28:00Z'),

  // ROAC — Sénégal
  membership(10, ORG.roac, PERSON.sowFall, 'manager', 'Directrice exécutive', '2026-02-19T11:05:00Z', {
    approved_by: PERSON.bakayoko,
    approved_at: '2026-03-04T14:20:00Z',
  }),
  membership(11, ORG.roac, PERSON.mbayeNdiaye, 'contributor', 'Chargé de projet littoral', '2026-03-02T10:40:00Z', {
    approved_by: PERSON.sowFall,
  }),
  // INVITATION ÉMISE PAR L'ORGANISATION, sans réponse à ce jour. C'est le cas
  // symétrique de la demande d'adhésion de Karim Ilboudo (UJFC, plus bas) : même
  // statut `pending`, direction inverse. La référente relance, elle n'approuve
  // pas — l'invitée n'a pas encore dit oui.
  membership(12, ORG.roac, PERSON.diallo, 'member', null, '2026-08-14T10:05:00Z', {
    status: 'pending',
    is_primary: false,
    invited_by: PERSON.sowFall,
    invited_at: '2026-08-14T10:05:00Z',
  }),
  // DEMANDE SPONTANÉE, en attente d'un référent : fin de mission chez Verdeo
  // (adhésion révoquée plus bas), Céline Lambert demande à rejoindre le ROAC.
  // Sans elle, la file de modération du référent n'aurait aucune donnée sur
  // l'organisation de démonstration.
  membership(13, ORG.roac, PERSON.lambert, 'member', 'Consultante littoral', '2026-08-12T15:40:00Z', {
    status: 'pending',
    is_primary: false,
  }),

  // OSED — la fiche complète
  membership(20, ORG.osed, PERSON.ouedraogo, 'manager', 'Directeur', '2026-02-24T16:30:00Z', {
    approved_by: PERSON.bakayoko,
    approved_at: '2026-03-11T09:15:00Z',
  }),
  membership(21, ORG.osed, PERSON.kabore, 'member', "Cheffe du département accès à l'énergie", '2026-03-12T09:05:00Z', {
    approved_by: PERSON.ouedraogo,
  }),
  // OSED — la fiche EN DOUBLON. Salamata Compaoré travaille dans la même
  // structure que Boureima Ouédraogo, mais dans une autre fiche : c'est la
  // conséquence visible du doublon, et ce que la fusion (A11) doit réparer.
  membership(22, ORG.osedSigle, PERSON.compaore, 'manager', 'Chargée de projets', '2026-06-11T13:45:00Z', {
    approved_by: PERSON.compaore,
  }),

  // ANTEB — Bénin
  membership(30, ORG.anteb, PERSON.zinsou, 'manager', 'Directeur de la transition écologique', '2026-03-28T07:50:00Z', {
    approved_by: PERSON.tremblay,
    approved_at: '2026-04-02T08:00:00Z',
  }),

  // COFEMAC — Cameroun
  membership(40, ORG.cofemac, PERSON.ngoBassong, 'manager', 'Coordonnatrice générale', '2026-04-10T09:25:00Z', {
    approved_by: PERSON.bakayoko,
    approved_at: '2026-04-18T11:30:00Z',
  }),

  // IMRE — Maroc
  membership(50, ORG.imre, PERSON.elFassi, 'manager', 'Chercheuse principale', '2026-04-20T13:15:00Z', {
    approved_by: PERSON.tremblay,
    approved_at: '2026-04-25T10:00:00Z',
  }),
  membership(51, ORG.imre, PERSON.benAmor, 'member', 'Directeur de recherche', '2026-04-21T08:30:00Z', {
    approved_by: PERSON.elFassi,
    is_primary: true,
  }),

  // FHRC — Haïti
  membership(60, ORG.fhrc, PERSON.josephPierre, 'manager', 'Responsable des programmes', '2026-05-06T17:40:00Z', {
    approved_by: PERSON.bakayoko,
  }),

  // CUDCM — Canada
  membership(70, ORG.cudcm, PERSON.gagnon, 'manager', 'Coordonnatrice de la chaire', '2026-05-11T08:05:00Z', {
    approved_by: PERSON.tremblay,
    approved_at: '2026-05-14T12:00:00Z',
  }),
  membership(71, ORG.cudcm, PERSON.lemoine, 'member', 'Professeure titulaire', '2026-05-11T08:12:00Z', {
    approved_by: PERSON.gagnon,
    is_primary: true,
  }),

  // CVDMF — Viêt Nam
  membership(80, ORG.cvdmf, PERSON.tranVanMinh, 'manager', 'Secrétaire général', '2026-05-18T04:10:00Z', {
    approved_by: PERSON.bakayoko,
    approved_at: '2026-05-22T06:30:00Z',
  }),

  // Verdéo Solutions — France
  membership(90, ORG.verdeo, PERSON.moreau, 'manager', 'Associé fondateur', '2026-06-02T09:55:00Z', {
    approved_by: PERSON.moreau,
  }),
  // Adhésion RÉVOQUÉE : fin de mission de consultance.
  membership(91, ORG.verdeo, PERSON.lambert, 'contributor', 'Consultante', '2026-05-29T18:10:00Z', {
    status: 'revoked',
    is_primary: false,
    approved_by: PERSON.moreau,
    approved_at: '2026-05-30T07:00:00Z',
    revoked_at: '2026-07-20T10:15:00Z',
  }),

  // UJFC — Côte d'Ivoire
  membership(100, ORG.ujfc, PERSON.koffi, 'manager', 'Président', '2026-06-09T10:20:00Z', {
    approved_by: PERSON.perretAdmin,
    approved_at: '2026-06-15T15:45:00Z',
  }),
  // Adhésion EN ATTENTE : c'est la file de modération du référent d'organisation.
  membership(101, ORG.ujfc, PERSON.ilboudo, 'member', 'Bénévole', '2026-07-14T20:35:00Z', {
    status: 'pending',
    is_primary: false,
  }),

  // MVOI — Madagascar
  membership(110, ORG.mvoi, PERSON.rakotomalala, 'manager', 'Journaliste environnement', '2026-07-28T06:15:00Z', {
    approved_by: PERSON.rakotomalala,
  }),
  membership(111, ORG.mvoi, PERSON.rasoanaivo, 'member', 'Rédactrice en chef', '2026-07-28T06:22:00Z', {
    approved_by: PERSON.rakotomalala,
    is_primary: true,
  }),
] satisfies Membership[]

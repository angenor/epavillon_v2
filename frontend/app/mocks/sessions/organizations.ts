/**
 * Données simulées de `programme.session_organizations`.
 *
 * La liste des co-organisateurs est REPRISE de la proposition à la
 * programmation, puis modifiable : une organisation peut se retirer entre le
 * dépôt du dossier et la tenue de l'activité, une autre se joindre à la
 * dernière minute. C'est pourquoi la table existe en double — côté dossier et
 * côté séance — plutôt que d'être partagée.
 *
 * Une différence volontaire avec `mocks/proposals/organizations.ts` : sur la
 * séance consacrée aux mangroves, l'IMRE n'apparaît plus. Son soutien n'avait
 * jamais été confirmé au dossier ; il n'a pas à figurer sur l'affiche.
 */

import type { SessionOrganization } from '~/types/programme/session'
import { ORG, SESSION } from '../ids'
import { plannedSessions } from './planned'
import { publishedSessions } from './published'

/** Porteur principal, repris du dossier ou de la séance créée par l'IFDD. */
const leadRows: SessionOrganization[] = [...publishedSessions, ...plannedSessions]
  .filter((s) => s.organization_id !== null)
  .map((s) => ({
    session_id: s.id,
    organization_id: s.organization_id!,
    role: 'lead',
    sort_order: 0,
    added_at: s.created_at,
  }))

const partnerRows = [
  // Adaptation côtière — co-organisation reprise à l'identique du dossier
  {
    session_id: SESSION.adaptationCotiere,
    organization_id: ORG.anteb,
    role: 'co_organizer',
    sort_order: 10,
    added_at: '2026-07-29T10:02:00Z',
  },
  {
    session_id: SESSION.adaptationCotiere,
    organization_id: ORG.imre,
    role: 'partner',
    sort_order: 20,
    added_at: '2026-07-29T10:03:00Z',
  },

  // Mini-réseaux solaires
  {
    session_id: SESSION.miniReseaux,
    organization_id: ORG.verdeo,
    role: 'partner',
    sort_order: 10,
    added_at: '2026-07-29T10:16:00Z',
  },

  // Pertes et préjudices — la configuration la plus riche
  {
    session_id: SESSION.pertesPrejudices,
    organization_id: ORG.roac,
    role: 'co_organizer',
    sort_order: 10,
    added_at: '2026-07-29T10:21:00Z',
  },
  {
    session_id: SESSION.pertesPrejudices,
    organization_id: ORG.cofemac,
    role: 'co_organizer',
    sort_order: 20,
    added_at: '2026-07-29T10:22:00Z',
  },
  {
    session_id: SESSION.pertesPrejudices,
    organization_id: ORG.cudcm,
    role: 'partner',
    sort_order: 30,
    added_at: '2026-07-29T10:23:00Z',
  },
  {
    session_id: SESSION.pertesPrejudices,
    organization_id: ORG.ifdd,
    role: 'sponsor',
    sort_order: 40,
    added_at: '2026-07-29T10:24:00Z',
  },

  // Transition juste
  {
    session_id: SESSION.transitionJuste,
    organization_id: ORG.ujfc,
    role: 'co_organizer',
    sort_order: 10,
    added_at: '2026-07-29T10:51:00Z',
  },

  // Mangroves — l'IMRE, soutien jamais confirmé au dossier, ne figure pas ici.
  {
    session_id: SESSION.mangroves,
    organization_id: ORG.roac,
    role: 'co_organizer',
    sort_order: 10,
    added_at: '2026-07-29T10:46:00Z',
  },

  // Accès au Fonds vert
  {
    session_id: SESSION.accesFondsVert,
    organization_id: ORG.fhrc,
    role: 'co_organizer',
    sort_order: 10,
    added_at: '2026-07-29T10:31:00Z',
  },
  {
    session_id: SESSION.accesFondsVert,
    organization_id: ORG.anteb,
    role: 'co_organizer',
    sort_order: 20,
    added_at: '2026-07-29T10:32:00Z',
  },
] satisfies SessionOrganization[]

export const sessionOrganizations: SessionOrganization[] = [...leadRows, ...partnerRows]

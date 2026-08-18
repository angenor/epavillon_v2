/**
 * RGPD — `identity.privacy_requests` et `identity.consents` (`030_identity.sql` § 5).
 *
 * Le RGPD était ABSENT DE LA V1. Une plateforme portée par un organe de l'OIF,
 * qui traite des données de ressortissants européens et africains, doit pouvoir
 * prouver un consentement et honorer une demande en trente jours. Ces deux tables
 * sont la trace de cette obligation ; l'écran annexe d'A12 en est la file.
 *
 * ── LES ÉCHÉANCES SONT RELATIVES À MAINTENANT ───────────────────────────────
 *
 * Seconde exception au principe « les mocks sont figés », après les jetons de
 * `auth.ts`, et pour la même raison : le sujet de cet écran est l'ÉCHÉANCE. Une
 * demande datée en dur serait en retard la semaine suivante, puis en retard de
 * trois ans, et la file n'aurait plus qu'un seul cas à montrer. Les demandes
 * closes, elles, gardent des dates fixes — leur échéance ne veut plus rien dire.
 *
 * ── QUATRE CAS, ET LE PLUS IMPORTANT EST LE TROISIÈME ───────────────────────
 *
 * Une demande d'EFFACEMENT ne se clôt pas comme les autres : elle appelle
 * `identity.anonymize_person()`, qui purge l'identité, supprime les comptes et
 * révoque les sessions — en conservant les agrégats de participation, pour que
 * les compteurs d'une COP passée ne s'effondrent pas. C'est irréversible, et
 * c'est pourquoi l'écran distingue « clore » et « anonymiser ».
 */

import type { Consent, CurrentConsent, PrivacyRequest } from '~/types/identity'
import { CONSENT, PERSON, PRIVACY_REQUEST } from './ids'

const DAY_MS = 24 * 60 * 60 * 1000

/** Une date décalée de N jours par rapport à maintenant. Négatif : dans le passé. */
const inDays = (days: number): string => new Date(Date.now() + days * DAY_MS).toISOString()

// ---------------------------------------------------------------------------
// Demandes — `identity.privacy_requests`
// ---------------------------------------------------------------------------

export const privacyRequests = [
  /**
   * EN RETARD. Reçue il y a quarante jours, échue il y a dix : la file doit
   * l'ouvrir en tête et le dire en rouge. C'est le seul cas où l'IFDD est en
   * faute, et un écran qui le noierait au milieu des autres ne servirait à rien.
   */
  {
    id: PRIVACY_REQUEST(1),
    person_id: PERSON.lambert,
    request_type: 'erasure',
    status: 'in_progress',
    due_at: inDays(-10),
    handled_by: PERSON.bakayoko,
    resolution: null,
    result_asset_id: null,
    created_at: inDays(-40),
    completed_at: null,
  },

  /** Reçue il y a trois jours : vingt-sept jours pour l'honorer. Le cas courant. */
  {
    id: PRIVACY_REQUEST(2),
    person_id: PERSON.rakotomalala,
    request_type: 'export',
    status: 'received',
    due_at: inDays(27),
    handled_by: null,
    resolution: null,
    result_asset_id: null,
    created_at: inDays(-3),
    completed_at: null,
  },

  /**
   * RECTIFICATION — le cas qu'on oublie. Ni export ni effacement : la personne
   * demande une correction, et le traitement n'est pas une manœuvre technique
   * mais une modification de sa fiche. L'écran ne doit donc pas proposer
   * l'anonymisation ici.
   */
  {
    id: PRIVACY_REQUEST(3),
    person_id: PERSON.tranVanMinh,
    request_type: 'rectification',
    status: 'in_progress',
    due_at: inDays(9),
    handled_by: PERSON.tremblay,
    resolution: null,
    result_asset_id: null,
    created_at: inDays(-21),
    completed_at: null,
  },

  /** CLOSE, dates fixes : une demande honorée n'a plus d'échéance à surveiller. */
  {
    id: PRIVACY_REQUEST(4),
    person_id: PERSON.moreau,
    request_type: 'export',
    status: 'completed',
    due_at: '2026-06-30T09:00:00Z',
    handled_by: PERSON.bakayoko,
    resolution: "Archive remise le 12 juin, accusé de réception de l'intéressé le 13.",
    result_asset_id: null,
    created_at: '2026-05-31T09:00:00Z',
    completed_at: '2026-06-12T14:20:00Z',
  },

  /**
   * REJETÉE, et le motif compte plus que le statut : une demande d'effacement
   * peut se heurter à une obligation de conservation. Sans ce cas, l'écran
   * n'aurait aucune raison d'exiger un motif de rejet.
   */
  {
    id: PRIVACY_REQUEST(5),
    person_id: PERSON.koffi,
    request_type: 'erasure',
    status: 'rejected',
    due_at: '2026-04-15T09:00:00Z',
    handled_by: PERSON.bakayoko,
    resolution:
      "Demande sans objet : la personne visée n'est pas le titulaire du compte. Identité non établie après deux relances.",
    created_at: '2026-03-16T09:00:00Z',
    completed_at: '2026-04-02T11:00:00Z',
    result_asset_id: null,
  },
] satisfies PrivacyRequest[]

// ---------------------------------------------------------------------------
// Consentements — `identity.consents`
//
// L'HISTORIQUE COMPLET EST CONSERVÉ, C'EST LA PREUVE. L'état courant se lit par
// la vue `current_consents`, qui garde la dernière ligne de chaque finalité —
// rejouée plus bas. Un consentement retiré n'efface pas celui qui l'a précédé :
// c'est justement ce qu'il faut pouvoir montrer.
// ---------------------------------------------------------------------------

function consent(
  n: number,
  person_id: string,
  purpose: string,
  is_granted: boolean,
  recorded_at: string,
  source = 'registration_form',
): Consent {
  return {
    id: CONSENT(n),
    person_id,
    purpose,
    is_granted,
    policy_version: '2026-01',
    source,
    ip_address: null,
    recorded_at,
  }
}

export const consents = [
  consent(1, PERSON.lambert, 'newsletter', true, '2026-05-29T18:05:00Z'),
  // RETRAIT : la même finalité, plus tard, refusée. Les deux lignes coexistent.
  consent(2, PERSON.lambert, 'newsletter', false, '2026-07-18T09:12:00Z', 'profile_settings'),
  consent(3, PERSON.lambert, 'directory_listing', false, '2026-07-18T09:12:00Z', 'profile_settings'),
  consent(4, PERSON.rakotomalala, 'newsletter', true, '2026-07-28T06:15:00Z'),
  consent(5, PERSON.rakotomalala, 'directory_listing', true, '2026-07-28T06:15:00Z'),
  consent(6, PERSON.rakotomalala, 'photo_usage', true, '2026-08-02T10:30:00Z', 'profile_settings'),
  consent(7, PERSON.moreau, 'newsletter', true, '2026-06-02T09:55:00Z'),
  consent(8, PERSON.moreau, 'analytics', false, '2026-06-02T09:55:00Z'),
  consent(9, PERSON.tranVanMinh, 'newsletter', true, '2026-05-18T04:10:00Z'),
  consent(10, PERSON.koffi, 'directory_listing', true, '2026-06-09T10:20:00Z'),
] satisfies Consent[]

/**
 * Vue `identity.current_consents` — `DISTINCT ON (person_id, purpose)`, la ligne
 * la plus récente de chaque finalité. Rejouée ici plutôt que recopiée : deux
 * listes divergeraient à la première ligne ajoutée.
 */
export function currentConsents(personId: string): CurrentConsent[] {
  const latest = new Map<string, Consent>()

  for (const entry of consents) {
    if (entry.person_id !== personId) continue
    const known = latest.get(entry.purpose)
    if (!known || known.recorded_at < entry.recorded_at) latest.set(entry.purpose, entry)
  }

  return [...latest.values()]
    .sort((a, b) => a.purpose.localeCompare(b.purpose, 'fr'))
    .map(({ person_id, purpose, is_granted, policy_version, recorded_at }) => ({
      person_id,
      purpose,
      is_granted,
      policy_version,
      recorded_at,
    }))
}

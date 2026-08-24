/**
 * LA FICHE D'UNE ORGANISATION (A11) — dénominations, domaines, membres,
 * activités, historique.
 *
 * C'est l'écran qui rend enfin visible CE QU'UNE ORGANISATION EST dans le
 * modèle : non pas une ligne avec un nom, mais un faisceau de dénominations, de
 * domaines de messagerie, d'adhésions et d'activités portées. La v1 n'avait
 * qu'un nom et un sigle côte à côte — et c'est très exactement pour cela qu'elle
 * fabriquait des doublons.
 *
 * ── LES DÉNOMINATIONS DÉRIVÉES NE SE RETIRENT PAS ───────────────────────────
 *
 * Le nom légal et le sigle sont recopiés dans `organization_names` par le
 * trigger `tg_sync_organization_names`. Ce ne sont pas des variantes ajoutées à
 * la main : elles suivent la fiche, et l'écran doit le dire plutôt que d'offrir
 * un bouton « retirer » qui ne ferait rien — la ligne reviendrait à la première
 * modification du nom.
 *
 * ── UN DOMAINE PARTAGÉ EST UN DOUBLON QUI SE VOIT ───────────────────────────
 *
 * `shared_with` nomme les autres fiches portant le même domaine. C'est le signal
 * le plus fiable du modèle (§ 3), et l'afficher sur la fiche évite d'attendre
 * que le worker remplisse la file : les deux OSED se reconnaissent ici, à l'œil
 * nu, avant toute détection automatique.
 *
 * ── L'HISTORIQUE N'EST PAS UNE TABLE ────────────────────────────────────────
 *
 * `platform.entity_history('org', 'organizations', id)` dépile `changed_fields`
 * du journal d'audit. Toute table portant `platform.tg_audit()` obtient donc son
 * historique gratuitement — `org.organizations` le porte. Les lignes écrites ici
 * sont celles qu'un journal d'audit aurait produites sur ce jeu de données ; les
 * écritures de la session s'y ajoutent au fil de la démonstration, ce que la
 * base ferait à l'identique.
 */

import type {
  DuplicatePair,
  OrganizationActivityRow,
  OrganizationDetail,
  OrganizationDomainRow,
  OrganizationHistoryEntry,
  OrganizationMemberRow,
  OrganizationNameRow,
} from '~/types/admin-organizations'
import { ORG, PERSON } from '../ids'
import { people } from '../people'
import { events } from '../event'
import { allProposals, proposalOrganizations } from '../proposals'
import { allSessions, sessionOrganizations } from '../sessions'
import {
  absorbedBy,
  effectiveDomains,
  effectiveMemberships,
  effectiveNames,
  effectiveOrganization,
  effectiveOrganizations,
  resolveOrganizationId,
  sessionHistoryOf,
  sessionMergeEntries,
} from './session'
import { countryOf, organizationTypeTerm, personName, scorecardOf } from './core'
import { pendingDuplicatesOf } from './duplicates'

// ---------------------------------------------------------------------------
// Historique — `platform.entity_history('org', 'organizations', id)`
// ---------------------------------------------------------------------------

function change(
  occurred_at: string,
  actor_id: string | null,
  actor_label: string | null,
  field: string,
  old_value: unknown,
  new_value: unknown,
): OrganizationHistoryEntry {
  return { occurred_at, actor_id, actor_label, action: 'update', field, old_value, new_value }
}

function creation(occurred_at: string, actor_id: string | null, actor_label: string | null): OrganizationHistoryEntry {
  return { occurred_at, actor_id, actor_label, action: 'insert', field: null, old_value: null, new_value: null }
}

/**
 * Les historiques écrits à la main.
 *
 * Trois fiches sur treize, et c'est assez : l'IFDD pour un parcours long, les
 * deux OSED parce que ce sont elles qu'on fusionne, et rien pour les autres —
 * treize historiques écrits à la main seraient illisibles sans lecteur. Même
 * choix qu'en A5 pour `proposals/history.ts`, documenté au même endroit.
 *
 * LE PLUS RÉCENT EN TÊTE, comme le rend `entity_history()` : `ORDER BY
 * occurred_at DESC`.
 */
const writtenHistories: Record<string, OrganizationHistoryEntry[]> = {
  [ORG.ifdd]: [
    change('2026-01-12T09:00:00Z', PERSON.adminPivot, 'Compte technique', 'verified_at', null, '2026-01-12T09:00:00Z'),
    change('2026-01-12T08:55:00Z', PERSON.adminPivot, 'Compte technique', 'status', 'candidate', 'active'),
    creation('2026-01-12T08:50:00Z', PERSON.adminPivot, 'Compte technique'),
  ],
  [ORG.osed]: [
    change('2026-05-30T10:10:00Z', PERSON.ouedraogo, 'Boureima Ouédraogo', 'contact_phone', null, '+226 25 36 08 11'),
    change('2026-03-11T09:15:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'verified_at', null, '2026-03-11T09:15:00Z'),
    change('2026-03-11T09:15:00Z', PERSON.bakayoko, 'Aminata Bakayoko', 'status', 'candidate', 'active'),
    change(
      '2026-03-02T11:40:00Z',
      PERSON.ouedraogo,
      'Boureima Ouédraogo',
      'description',
      null,
      {
        fr: "Centre de recherche appliquée sur l'accès à l'énergie en zone sahélienne : mini-réseaux solaires, cuisson propre, planification énergétique territoriale.",
      },
    ),
    creation('2026-02-24T16:30:00Z', PERSON.ouedraogo, 'Boureima Ouédraogo'),
  ],
  // La fiche en doublon : créée, jamais complétée, jamais vérifiée. Son
  // historique tient en une ligne, et c'est déjà une information — personne ne
  // l'a regardée depuis sa création.
  [ORG.osedSigle]: [creation('2026-06-11T13:45:00Z', PERSON.compaore, 'Salamata Compaoré')],
}

/** Historique d'une fiche : ce que le jeu porte, plus ce que la session a écrit. */
function historyOf(organizationId: string): OrganizationHistoryEntry[] {
  return [...sessionHistoryOf(organizationId), ...(writtenHistories[organizationId] ?? [])].sort(
    (a, b) => b.occurred_at.localeCompare(a.occurred_at),
  )
}

// ---------------------------------------------------------------------------
// Les panneaux de la fiche
// ---------------------------------------------------------------------------

function namesOf(organizationId: string): OrganizationNameRow[] {
  const organization = effectiveOrganization(organizationId)

  return effectiveNames()
    .filter((name) => name.organization_id === organizationId)
    .map((name) => ({
      id: name.id,
      name: name.name,
      kind: name.kind,
      locale: name.locale,
      is_confirmed: name.is_confirmed,
      created_by_name: personName(name.created_by),
      created_at: name.created_at,
      // Posée par `tg_sync_organization_names` : elle suit le nom légal et le
      // sigle de la fiche, et ne se retire pas à la main.
      is_derived:
        organization !== null &&
        ((name.kind === 'legal' && name.name === organization.legal_name) ||
          (name.kind === 'acronym' && name.name === organization.acronym)),
    }))
    .sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name, 'fr'))
}

function domainsOf(organizationId: string): OrganizationDomainRow[] {
  const all = effectiveDomains()

  return all
    .filter((domain) => domain.organization_id === organizationId)
    .map((domain) => ({
      id: domain.id,
      domain: domain.domain,
      verified_at: domain.verified_at,
      verification_method: domain.verification_method,
      auto_join: domain.auto_join,
      created_at: domain.created_at,
      shared_with: all
        .filter((other) => other.domain === domain.domain && other.organization_id !== organizationId)
        .map((other) => ({
          organization_id: other.organization_id,
          legal_name: effectiveOrganization(other.organization_id)?.legal_name ?? '',
        })),
    }))
    .sort((a, b) => a.domain.localeCompare(b.domain))
}

/**
 * Les membres, dans l'ordre où ils se traitent : ce qui attend d'abord, les
 * adhésions vivantes ensuite, les révocations en dernier.
 */
function membersOf(organizationId: string): OrganizationMemberRow[] {
  const rank: Record<string, number> = { pending: 0, active: 1, revoked: 2 }

  return effectiveMemberships()
    .filter((membership) => membership.organization_id === organizationId)
    .map((membership) => {
      const person = people.find((p) => p.id === membership.person_id)
      return {
        id: membership.id,
        person_id: membership.person_id,
        display_name: person?.display_name ?? '',
        primary_email: person?.primary_email ?? '',
        role: membership.role,
        status: membership.status,
        is_primary: membership.is_primary,
        job_title: membership.job_title,
        invited_at: membership.invited_at,
        approved_at: membership.approved_at,
        revoked_at: membership.revoked_at,
        created_at: membership.created_at,
      }
    })
    .sort(
      (a, b) =>
        (rank[a.status] ?? 3) - (rank[b.status] ?? 3) || a.display_name.localeCompare(b.display_name, 'fr'),
    )
}

/**
 * Les activités portées — dossiers ET séances, tous rôles confondus.
 *
 * LE RÔLE COMPTE AUTANT QUE L'ACTIVITÉ. Une organisation qui figure douze fois
 * comme soutien n'a pas le même parcours qu'une organisation qui a porté douze
 * dossiers : c'est `programme.proposal_organizations.role` qui les distingue, et
 * la v1 ne pouvait pas le dire, faute d'avoir la table.
 */
function activitiesOf(organizationId: string): OrganizationActivityRow[] {
  const eventOf = (eventId: string) => events.find((e) => e.id === eventId)
  const rows: OrganizationActivityRow[] = []

  for (const proposal of allProposals) {
    if (proposal.deleted_at !== null) continue
    const link = proposalOrganizations.find(
      (l) => l.proposal_id === proposal.id && resolveOrganizationId(l.organization_id) === organizationId,
    )
    if (!link) continue

    const event = eventOf(proposal.event_id)
    rows.push({
      kind: 'proposal',
      id: proposal.id,
      reference_code: proposal.reference_code,
      title: proposal.title,
      event_id: proposal.event_id,
      event_name: event?.title ?? { fr: '' },
      edition_year: event?.edition_year ?? 0,
      role: link.role,
      status: proposal.status,
      occurred_at: proposal.submitted_at ?? proposal.created_at,
    })
  }

  for (const session of allSessions) {
    const isLead =
      session.organization_id !== null && resolveOrganizationId(session.organization_id) === organizationId
    const link = sessionOrganizations.find(
      (l) => l.session_id === session.id && resolveOrganizationId(l.organization_id) === organizationId,
    )
    if (!isLead && !link) continue

    const event = eventOf(session.event_id)
    rows.push({
      kind: 'session',
      id: session.id,
      reference_code: null,
      title: session.title,
      event_id: session.event_id,
      event_name: event?.title ?? { fr: '' },
      edition_year: event?.edition_year ?? 0,
      role: isLead ? 'lead' : (link?.role ?? 'partner'),
      status: session.status,
      occurred_at: session.starts_at,
    })
  }

  // Le plus récent d'abord : c'est ce qu'on vient vérifier sur une fiche —
  // « cette organisation est-elle encore active ? ».
  return rows.sort((a, b) => (b.occurred_at ?? '').localeCompare(a.occurred_at ?? ''))
}

// ---------------------------------------------------------------------------
// La fiche entière
// ---------------------------------------------------------------------------

/**
 * TOUTE LA FICHE EN UNE RÉPONSE.
 *
 * ELLE S'OUVRE AUSSI POUR UNE FICHE ABSORBÉE, et c'est la promesse du modèle :
 * `merge_organizations()` conserve la fiche source en statut `merged` « pour que
 * les URL et identifiants externes déjà diffusés continuent de résoudre ».
 * L'écran la coiffe alors du renvoi vers la fiche vivante — il ne rend pas 404,
 * ce qui reviendrait à casser exactement ce que la fusion promet de préserver.
 */
export function organizationDetail(organizationId: string): OrganizationDetail | null {
  const organization = effectiveOrganization(organizationId)
  if (!organization) return null

  const type = organizationTypeTerm(organization.organization_type_code)
  const country = countryOf(organization.country_id)

  const mergedIntoId = resolveOrganizationId(organizationId)
  const mergedInto =
    mergedIntoId !== organizationId ? effectiveOrganization(mergedIntoId) : null

  const absorbed = [
    ...absorbedBy(organizationId),
    ...effectiveOrganizations()
      .filter((o) => o.merged_into_id === organizationId)
      .map((o) => o.id),
  ]

  return {
    organization_id: organization.id,
    legal_name: organization.legal_name,
    acronym: organization.acronym,
    slug: organization.slug,
    status: organization.status,
    organization_type_code: organization.organization_type_code,
    organization_type_label: type.label,
    country_id: organization.country_id,
    country_name: country.name,
    city: organization.city,
    description: organization.description,
    website: organization.website,
    contact_email: organization.contact_email,
    contact_phone: organization.contact_phone,
    verified_at: organization.verified_at,
    verified_by_name: personName(organization.verified_by),
    trust_score: scorecardOf(organization).score_confiance,
    created_at: organization.created_at,
    created_by_name: personName(organization.created_by),

    merged_into: mergedInto
      ? {
          organization_id: mergedInto.id,
          legal_name: mergedInto.legal_name,
          merged_at:
            organization.merged_at ??
            sessionMergeEntries().find((entry) => entry.source_id === organizationId)?.performed_at ??
            null,
        }
      : null,
    absorbed: [...new Set(absorbed)].map((id) => {
      const source = effectiveOrganization(id)
      return {
        organization_id: id,
        legal_name: source?.legal_name ?? '',
        merged_at:
          source?.merged_at ??
          sessionMergeEntries().find((entry) => entry.source_id === id)?.performed_at ??
          null,
      }
    }),

    scorecard: scorecardOf(organization),
    names: namesOf(organizationId),
    domains: domainsOf(organizationId),
    members: membersOf(organizationId),
    activities: activitiesOf(organizationId),
    history: historyOf(organizationId),
    merges: sessionMergeEntries().filter(
      (entry) => entry.source_id === organizationId || entry.target_id === organizationId,
    ),
    duplicates: pendingDuplicatesOf(organizationId) as DuplicatePair[],
  }
}

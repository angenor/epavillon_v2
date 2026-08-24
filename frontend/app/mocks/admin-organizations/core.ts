/**
 * ORGANISATIONS ET FUSION (A11) — le socle : la fiche de performance
 * reconstituée, et ce qui sert aux trois autres fichiers du dossier.
 *
 * CE FICHIER NE CONTIENT AUCUNE DONNÉE NOUVELLE. Il rejoue en TypeScript
 * `analytics.mv_organization_scorecard` (`130_analytics.sql` § 5) sur les treize
 * fiches de `org.ts`, leurs adhésions, leurs dossiers et leurs séances. Même
 * parti qu'`organization-search.ts`, qui rejoue `find_similar_organizations()`,
 * et pour la même raison : des chiffres écrits à la main seraient plausibles,
 * l'écran se réglerait sur eux, et il faudrait tout reprendre au branchement
 * (B7). Ici, la colonne « ratio » vaut ce que vaudra la vue.
 *
 * LE RATIO EST NUL, JAMAIS ZÉRO, SANS AUCUN DÉPÔT — c'est le `COMMENT ON` de la
 * colonne, et c'est ce qui distingue « n'a jamais essayé » de « échoue à chaque
 * fois ». Deux organisations du jeu sont dans ce cas.
 *
 * LES FICHES FUSIONNÉES Y RESTENT. La vue ne filtre rien : elle lit
 * `org.organizations` en entier, statut compris. C'est le back-office qui écarte
 * ce qu'il ne veut pas montrer, et l'historique d'une fiche absorbée doit rester
 * consultable — c'est la promesse de `resolve_organization()`.
 */

import type { OrganizationScorecard } from '~/types/analytics'
import type {
  DuplicateSide,
  OrganizationFacet,
  OrganizationListRow,
  OrganizationListScreen,
} from '~/types/admin-organizations'
import type { AdministeredEvents } from '~/types/identity'
import type { Organization } from '~/types/org'
import type { I18nText } from '~/types/shared'
import { countries, taxonomyTerms } from '../reference'
import { people } from '../people'
import { duplicateQueue } from './duplicates'
import {
  absorbedBy,
  computeTrustScore,
  effectiveDomains,
  effectiveMemberships,
  effectiveOrganizations,
  resolveOrganizationId,
} from './session'
import { allProposals, proposalOrganizations } from '../proposals'
import { allSessions, sessionOrganizations } from '../sessions'
import { registrations } from '../registrations'

// ---------------------------------------------------------------------------
// Les jointures de la vue, une fois pour toutes
// ---------------------------------------------------------------------------

/** Libellé et couleur d'un type d'organisation — `reference.taxonomy_terms`. */
export function organizationTypeTerm(code: string): { label: I18nText | null; color: string | null } {
  const term = taxonomyTerms.find((t) => t.taxonomy_code === 'organization_type' && t.code === code)
  return { label: term?.label ?? null, color: term?.color_hex ?? null }
}

/** Pays d'une organisation, résolu — la vue joint `reference.countries`. */
export function countryOf(countryId: string | null): {
  name: I18nText | null
  iso3: string | null
  oif: string
} {
  const country = countryId ? countries.find((c) => c.id === countryId) : undefined
  return {
    name: country?.name ?? null,
    iso3: country?.iso3 ?? null,
    oif: country?.oif_status ?? 'none',
  }
}

/** Nom affichable d'une personne, ou `null` — les auteurs de fiche sont facultatifs. */
export function personName(personId: string | null): string | null {
  if (!personId) return null
  const person = people.find((p) => p.id === personId)
  return person ? person.display_name : null
}

/** Adhésions ACTIVES d'une fiche. Une demande en attente ne prouve encore rien. */
export function activeMembers(organizationId: string): number {
  return effectiveMemberships().filter(
    (m) => m.organization_id === organizationId && m.status === 'active',
  ).length
}

/**
 * Dossiers d'une organisation — SON PORTAGE PRINCIPAL, `proposals.organization_id`.
 *
 * Les co-organisations ne comptent pas ici, et c'est la vue qui en décide :
 * `mv_organization_scorecard` agrège `programme.proposals` par
 * `organization_id`. Une activité co-portée est comptée à son PORTEUR, une seule
 * fois — sans quoi la somme des dossiers de toutes les organisations dépasserait
 * le nombre de dossiers déposés, et le ratio d'acceptation deviendrait
 * incomparable d'une fiche à l'autre.
 */
export function proposalsOf(organizationId: string) {
  // La REDIRECTION plutôt que la réécriture : après une fusion, les dossiers de
  // la fiche absorbée comptent pour la fiche absorbante — c'est
  // `org.resolve_organization()`, et c'est ce que fait la base en réaffectant la
  // colonne.
  return allProposals.filter(
    (p) => resolveOrganizationId(p.organization_id) === organizationId && p.deleted_at === null,
  )
}

/** Séances portées, toutes éditions confondues. */
export function sessionsOf(organizationId: string) {
  return allSessions.filter(
    (s) => s.organization_id !== null && resolveOrganizationId(s.organization_id) === organizationId,
  )
}

/**
 * `analytics.mv_organization_scorecard`, une ligne.
 *
 * Les colonnes du module Publications valent zéro : le module existe dans le
 * modèle, ses écrans sont hors du jalon et le jeu de données ne porte aucun
 * article. La vue les rendrait à zéro par `COALESCE` — c'est donc exact, et non
 * un raccourci.
 */
export function scorecardOf(organization: Organization): OrganizationScorecard {
  const country = countryOf(organization.country_id)
  const orgMemberships = effectiveMemberships().filter((m) => m.organization_id === organization.id)
  const proposals = proposalsOf(organization.id)
  const sessions = sessionsOf(organization.id)

  const submitted = proposals.filter((p) => p.submitted_at !== null)
  const accepted = proposals.filter((p) => p.status === 'accepted')
  const scored = submitted.map((p) => p.average_score).filter((n): n is number => n !== null)

  const sessionIds = new Set(sessions.map((s) => s.id))
  const sessionRegistrations = registrations.filter((r) => sessionIds.has(r.session_id))

  const lastMembership = orgMemberships.map((m) => m.created_at).sort().at(-1) ?? null
  const lastProposal = proposals.map((p) => p.created_at).sort().at(-1) ?? null
  const lastSession = sessions.map((s) => s.starts_at).sort().at(-1) ?? null

  return {
    organization_id: organization.id,
    legal_name: organization.legal_name,
    acronym: organization.acronym,
    slug: organization.slug,
    statut: organization.status,
    organization_type_code: organization.organization_type_code,
    country_id: organization.country_id,
    pays_iso3: country.iso3,
    pays_nom: country.name,
    statut_oif: country.oif,
    est_verifiee: organization.verified_at !== null,
    verified_at: organization.verified_at,
    // Recalculé, jamais lu tel quel : poser un sceau ou vérifier un domaine
    // déplace la fiche dans le tri par score, sans attendre le worker.
    score_confiance: computeTrustScore(organization.id),
    merged_into_id: organization.merged_into_id,

    membres_actifs: orgMemberships.filter((m) => m.status === 'active').length,
    membres_en_attente: orgMemberships.filter((m) => m.status === 'pending').length,
    referents: orgMemberships.filter((m) => m.status === 'active' && m.role === 'manager').length,

    propositions_deposees: submitted.length,
    propositions_en_brouillon: proposals.filter((p) => p.status === 'draft').length,
    propositions_acceptees: accepted.length,
    propositions_rejetees: proposals.filter((p) => p.status === 'rejected').length,
    propositions_retirees: proposals.filter((p) => p.status === 'withdrawn').length,
    evenements_couverts: new Set(submitted.map((p) => p.event_id)).size,
    note_moyenne_obtenue:
      scored.length > 0
        ? Math.round((scored.reduce((sum, n) => sum + n, 0) / scored.length) * 100) / 100
        : null,
    // LE ratio du cadrage : `NULLIF(propositions_deposees, 0)` en base, `null`
    // ici. Un ratio de 0 pour une organisation qui n'a rien déposé serait un
    // contresens, et c'est celui que la v1 affichait.
    ratio_acceptation:
      submitted.length > 0 ? Math.round((accepted.length / submitted.length) * 10000) / 10000 : null,

    sessions_programmees: sessions.filter((s) =>
      ['scheduled', 'live', 'completed'].includes(s.status),
    ).length,
    sessions_realisees: sessions.filter((s) => s.status === 'completed').length,
    sessions_annulees: sessions.filter((s) => s.status === 'cancelled').length,
    inscrits_a_ses_sessions: sessionRegistrations.filter((r) => r.status !== 'cancelled').length,
    presents_a_ses_sessions: sessionRegistrations.filter((r) => r.joined_at !== null).length,

    articles_publies: 0,
    articles_en_moderation: 0,
    octets_stockes: 0,

    // `GREATEST` en base : le dernier signe de vie, toutes natures confondues.
    // C'est le tri qui distingue une fiche dormante d'une fiche active.
    derniere_activite:
      [organization.updated_at, lastMembership, lastProposal, lastSession]
        .filter((value): value is string => value !== null)
        .sort()
        .at(-1) ?? null,
    inscrite_le: organization.created_at,
  }
}

/** Les treize fiches, chiffrées — fusions de la session comprises. */
export function organizationScorecards(): OrganizationScorecard[] {
  return effectiveOrganizations().map(scorecardOf)
}

// ---------------------------------------------------------------------------
// Un côté de paire — ce qu'il faut pour TRANCHER sans ouvrir les deux fiches
// ---------------------------------------------------------------------------

/**
 * La fiche réduite à ce qui permet de reconnaître un doublon : qui l'a créée et
 * quand, ce qu'elle porte déjà, ses domaines.
 *
 * LES DOMAINES SONT LÀ PARCE QUE C'EST LE MOTIF LE PLUS FIABLE. Deux fiches
 * portant `osed-sahel.org` sont la même maison, quels que soient les libellés
 * saisis — le § 3 du SQL le dit sans nuance. Les afficher côte à côte fait voir
 * le doublon avant même de lire les noms.
 */
export function duplicateSideOf(organizationId: string): DuplicateSide | null {
  const organization = effectiveOrganizations().find((o) => o.id === organizationId)
  if (!organization) return null

  const type = organizationTypeTerm(organization.organization_type_code)
  const country = countryOf(organization.country_id)

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
    website: organization.website,
    contact_email: organization.contact_email,
    verified_at: organization.verified_at,
    trust_score: computeTrustScore(organization.id),
    member_count: activeMembers(organization.id),
    // Le portage principal ET les co-organisations : ici, à l'inverse du ratio,
    // la question est « qu'est-ce que cette fiche perdrait ? », et une
    // co-organisation est un rattachement qui se déplacera.
    proposal_count: new Set([
      ...proposalsOf(organization.id).map((p) => p.id),
      ...proposalOrganizations
        .filter((link) => resolveOrganizationId(link.organization_id) === organization.id)
        .map((link) => link.proposal_id),
    ]).size,
    session_count: new Set([
      ...sessionsOf(organization.id).map((s) => s.id),
      ...sessionOrganizations
        .filter((link) => resolveOrganizationId(link.organization_id) === organization.id)
        .map((link) => link.session_id),
    ]).size,
    domains: effectiveDomains()
      .filter((d) => d.organization_id === organization.id)
      .map((d) => d.domain),
    created_at: organization.created_at,
    created_by_name: personName(organization.created_by),
  }
}

// ---------------------------------------------------------------------------
// Facettes
// ---------------------------------------------------------------------------

/** Facettes comptées sur le jeu de lignes AFFICHÉ, jamais sur la table entière. */
export function facetsOf(
  rows: OrganizationScorecard[],
): { countries: OrganizationFacet[]; types: OrganizationFacet[] } {
  const byCountry = new Map<string, OrganizationFacet>()
  const byType = new Map<string, OrganizationFacet>()

  for (const row of rows) {
    if (row.country_id) {
      const existing = byCountry.get(row.country_id)
      if (existing) existing.count += 1
      else
        byCountry.set(row.country_id, {
          value: row.country_id,
          label: row.pays_nom ?? row.pays_iso3 ?? '',
          count: 1,
        })
    }

    const existingType = byType.get(row.organization_type_code)
    if (existingType) existingType.count += 1
    else
      byType.set(row.organization_type_code, {
        value: row.organization_type_code,
        label: organizationTypeTerm(row.organization_type_code).label ?? row.organization_type_code,
        count: 1,
      })
  }

  const byLabel = (a: OrganizationFacet, b: OrganizationFacet) =>
    String(typeof a.label === 'string' ? a.label : (a.label.fr ?? '')).localeCompare(
      String(typeof b.label === 'string' ? b.label : (b.label.fr ?? '')),
      'fr',
    )

  return {
    countries: [...byCountry.values()].sort(byLabel),
    types: [...byType.values()].sort(byLabel),
  }
}

// ---------------------------------------------------------------------------
// L'écran de la liste
// ---------------------------------------------------------------------------

/**
 * TOUTE LA LISTE EN UNE RÉPONSE, FILTRÉE PAR LE PÉRIMÈTRE D'ADMINISTRATION.
 *
 * RÈGLE MÉTIER N° 8, APPLIQUÉE À UNE ENTITÉ QUI N'APPARTIENT À AUCUNE ÉDITION.
 * Une organisation n'est pas rattachée à une COP : filtrer « les organisations
 * de la COP31 » n'a pas de sens en soi. Ce qui en a un, c'est : les
 * organisations QUI ONT DÉPOSÉ OU TENU UNE ACTIVITÉ dans les éditions
 * administrées. Une coordonnatrice détachée sur la COP31 voit donc les fiches
 * dont elle a à connaître, et rien d'autre — ni les organisations d'une autre
 * COP, ni celles qui n'ont jamais rien déposé.
 *
 * `scoped_to_events` dit à l'écran que la liste a été restreinte. Sans cette
 * mention, une liste de six lignes laisserait croire que la plateforme compte
 * six organisations, et le premier réflexe serait d'en créer une septième —
 * c'est-à-dire de fabriquer le doublon que tout cet écran sert à éviter.
 */
export function organizationListScreen(scope: AdministeredEvents): OrganizationListScreen {
  const all = organizationScorecards()

  const inScope = (row: OrganizationScorecard): boolean => {
    if (scope.is_global) return true
    if (scope.event_ids.length === 0) return false

    const events = new Set(scope.event_ids)
    return (
      proposalsOf(row.organization_id).some((p) => events.has(p.event_id)) ||
      sessionsOf(row.organization_id).some((s) => events.has(s.event_id)) ||
      proposalOrganizations.some(
        (link) =>
          resolveOrganizationId(link.organization_id) === row.organization_id &&
          allProposals.some((p) => p.id === link.proposal_id && events.has(p.event_id)),
      )
    )
  }

  const visible = all.filter(inScope)
  const queue = duplicateQueue().pending

  // LES COLONNES SONT ÉNUMÉRÉES, JAMAIS ÉTALÉES. La liste ne rend pas la
  // projection entière : dix compteurs qu'aucun écran n'affiche — brouillons,
  // retraits, note moyenne, stockage, articles — restent dans la fiche de
  // performance et n'entrent pas dans la ligne. Un `...row` les aurait fait
  // paraître disponibles ici, alors que l'API ne les sélectionne pas.
  const rows: OrganizationListRow[] = visible.map((row) => {
    const type = organizationTypeTerm(row.organization_type_code)
    return {
      organization_id: row.organization_id,
      legal_name: row.legal_name,
      acronym: row.acronym,
      slug: row.slug,
      statut: row.statut,
      organization_type_code: row.organization_type_code,
      organization_type_label: type.label,
      organization_type_color: type.color,
      country_id: row.country_id,
      pays_iso3: row.pays_iso3,
      pays_nom: row.pays_nom,
      statut_oif: row.statut_oif,
      est_verifiee: row.est_verifiee,
      verified_at: row.verified_at,
      score_confiance: row.score_confiance,
      merged_into_id: row.merged_into_id,

      membres_actifs: row.membres_actifs,
      membres_en_attente: row.membres_en_attente,
      referents: row.referents,

      propositions_deposees: row.propositions_deposees,
      propositions_acceptees: row.propositions_acceptees,
      propositions_rejetees: row.propositions_rejetees,
      ratio_acceptation: row.ratio_acceptation,

      sessions_programmees: row.sessions_programmees,
      sessions_realisees: row.sessions_realisees,

      pending_duplicate_count: queue.filter(
        (pair) =>
          pair.left.organization_id === row.organization_id ||
          pair.right.organization_id === row.organization_id,
      ).length,
      absorbed_count:
        absorbedBy(row.organization_id).length +
        all.filter((other) => other.merged_into_id === row.organization_id).length,

      derniere_activite: row.derniere_activite,
      inscrite_le: row.inscrite_le,
    }
  })

  const facets = facetsOf(visible)

  return {
    rows,
    countries: facets.countries,
    types: facets.types,
    pending_duplicates: queue.length,
    scoped_to_events: !scope.is_global,
  }
}

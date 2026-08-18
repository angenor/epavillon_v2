/**
 * LE JOURNAL D'ÉCRITURES DE LA SESSION DE DÉMONSTRATION (A11).
 *
 * Les données simulées restent en lecture seule : rien de ce qui est écrit à la
 * main dans `org.ts`, `memberships.ts` ou `proposals/` n'est modifié. Ce fichier
 * empile PAR-DESSUS ce qu'une session de démonstration produit, et rend les
 * versions « effectives » que lisent les trois autres fichiers du dossier. Même
 * exception, et même durée de vie, que le journal d'`organization-search.ts` :
 * un module, donc jusqu'au prochain rechargement de la page.
 *
 * POURQUOI IL FAUT UN JOURNAL ICI, ALORS QU'AILLEURS ON S'EN PASSE. Une fusion
 * n'est pas une écriture qu'on peut se contenter d'ANNONCER : elle change ce que
 * six écrans affichent. Sans persistance, valider la fusion des deux fiches OSED
 * rendrait un décompte juste, puis la file les reproposerait à l'identique une
 * seconde plus tard, la liste continuerait d'afficher deux lignes, et la
 * démonstration donnerait à voir un outil qui ne fait rien. C'est le seul écran
 * du jalon où l'effet de l'action EST le sujet de l'écran.
 *
 * ── LA REDIRECTION PLUTÔT QUE LA RÉÉCRITURE ─────────────────────────────────
 *
 * `org.resolve_organization()` est la clé de voûte du modèle : après une fusion,
 * toute référence à la fiche absorbée résout vers la fiche absorbante, et « les
 * anciennes URL continuent de résoudre ». Le journal fait exactement cela — il
 * tient une table de redirection et l'applique aux lectures, plutôt que de
 * réécrire quarante dossiers et soixante-sept inscriptions. C'est plus fidèle au
 * modèle, et non un raccourci : en base non plus, la fusion ne « recopie » rien,
 * elle réaffecte une colonne.
 *
 * LES CHAÎNES SONT INTERDITES — `tg_forbid_merge_chains`. La redirection n'a donc
 * jamais plus d'un saut, et `resolveOrganizationId()` n'a pas à boucler.
 */

import type { Membership, Organization, OrganizationDomain, OrganizationName } from '~/types/org'
import type { DuplicateDecision } from '~/types/org'
import type { OrganizationHistoryEntry, OrganizationMergeEntry } from '~/types/admin-organizations'
import type { IsoDateTime } from '~/types/shared'
import { memberships } from '../memberships'
import { organizationDomains, organizationNames, organizations } from '../org'

// ---------------------------------------------------------------------------
// L'état de la session
// ---------------------------------------------------------------------------

/** Fiche absorbée → fiche absorbante. Un seul saut, jamais une chaîne. */
const mergedInto = new Map<string, string>()
/** Champs de fiche modifiés : choix de fusion, sceau de vérification. */
const organizationPatches = new Map<string, Partial<Organization>>()
/** Domaines vérifiés ou basculés en rattachement automatique pendant la session. */
const domainPatches = new Map<string, Partial<OrganizationDomain>>()
/** Dénominations confirmées ou infirmées pendant la session. */
const namePatches = new Map<string, Partial<OrganizationName>>()
/** Arbitrages portés sur les paires de doublons. */
const pairDecisions = new Map<
  string,
  { decision: DuplicateDecision; reviewed_at: IsoDateTime; reviewed_by: string | null; note: string | null }
>()
/** Fusions consignées — `org.merge_log`. */
const mergeEntries: OrganizationMergeEntry[] = []
/**
 * Lignes d'audit produites par les écritures de la session, par fiche.
 *
 * `platform.tg_audit()` les écrirait sans qu'aucun code applicatif s'en occupe :
 * poser un sceau, c'est un `UPDATE` sur `org.organizations`, donc une ligne
 * d'historique. Les tenir ici rend visible ce que la base ferait — sans quoi
 * l'onglet « Historique » afficherait un passé figé sous les yeux de quelqu'un
 * qui vient d'écrire.
 */
const sessionHistory = new Map<string, OrganizationHistoryEntry[]>()

// ---------------------------------------------------------------------------
// Lectures effectives
// ---------------------------------------------------------------------------

/** `org.resolve_organization()` : la fiche vivante, en suivant la redirection. */
export function resolveOrganizationId(id: string): string {
  return mergedInto.get(id) ?? id
}

/** Fiches absorbées depuis le début de la session. */
export function mergedOrganizationIds(): Set<string> {
  return new Set(mergedInto.keys())
}

/** Fiches que celle-ci a absorbées. */
export function absorbedBy(targetId: string): string[] {
  return [...mergedInto.entries()]
    .filter(([, target]) => target === targetId)
    .map(([source]) => source)
}

/** Les treize fiches, avec ce que la session leur a fait. */
export function effectiveOrganizations(): Organization[] {
  return organizations.map((organization) => {
    const patch = organizationPatches.get(organization.id)
    return patch ? { ...organization, ...patch } : organization
  })
}

export function effectiveOrganization(id: string): Organization | null {
  return effectiveOrganizations().find((o) => o.id === id) ?? null
}

/**
 * Les dénominations, redirigées.
 *
 * Étape 1 de `merge_organizations()` : celles de la source deviennent des
 * variantes de la cible, SAUF quand la cible porte déjà le même nom normalisé
 * pour la même nature — auquel cas la ligne source est supprimée. C'est ce
 * `DELETE ... WHERE NOT EXISTS` que reproduit le filtre ci-dessous.
 */
export function effectiveNames(): OrganizationName[] {
  const seen = new Set<string>()
  const out: OrganizationName[] = []

  for (const name of organizationNames) {
    const patch = namePatches.get(name.id)
    const organizationId = resolveOrganizationId(name.organization_id)
    const key = `${organizationId}|${(name.name_normalized ?? name.name).toLowerCase()}|${name.kind}`
    if (seen.has(key)) continue
    seen.add(key)
    out.push({ ...name, ...patch, organization_id: organizationId })
  }
  return out
}

/** Les domaines, redirigés. Un domaine déjà porté par la cible ne se dédouble pas. */
export function effectiveDomains(): OrganizationDomain[] {
  const seen = new Set<string>()
  const out: OrganizationDomain[] = []

  for (const domain of organizationDomains) {
    const patch = domainPatches.get(domain.id)
    const organizationId = resolveOrganizationId(domain.organization_id)
    const key = `${organizationId}|${domain.domain}`
    if (seen.has(key)) continue
    seen.add(key)
    out.push({ ...domain, ...patch, organization_id: organizationId })
  }
  return out
}

/**
 * Les adhésions, redirigées.
 *
 * `dedupe_on: {person_id}` : une personne adhérente des DEUX fiches n'en garde
 * qu'une après fusion. La base supprime la ligne source avant la bascule ; ici,
 * la première rencontrée l'emporte — et l'ordre du jeu de données place les
 * fiches complètes avant les doublons, donc l'adhésion conservée est celle de la
 * fiche de référence.
 */
export function effectiveMemberships(): Membership[] {
  const seen = new Set<string>()
  const out: Membership[] = []

  for (const membership of memberships) {
    const organizationId = resolveOrganizationId(membership.organization_id)
    const key = `${organizationId}|${membership.person_id}`
    if (seen.has(key)) continue
    seen.add(key)
    out.push({ ...membership, organization_id: organizationId })
  }
  return out
}

/** Lignes d'historique écrites pendant la session, plus récentes d'abord. */
export function sessionHistoryOf(organizationId: string): OrganizationHistoryEntry[] {
  return [...(sessionHistory.get(organizationId) ?? [])].sort((a, b) =>
    b.occurred_at.localeCompare(a.occurred_at),
  )
}

/** Arbitrage porté sur une paire pendant la session, ou `null`. */
export function settledDecisionOf(pairId: string) {
  return pairDecisions.get(pairId) ?? null
}

/** Le journal des fusions — `org.merge_log`, ordre décroissant. */
export function sessionMergeEntries(): OrganizationMergeEntry[] {
  return [...mergeEntries].sort((a, b) => b.performed_at.localeCompare(a.performed_at))
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

/** Applique un correctif à une fiche — choix de fusion, sceau. */
export function patchOrganization(id: string, patch: Partial<Organization>): void {
  organizationPatches.set(id, { ...organizationPatches.get(id), ...patch })
}

export function patchDomain(id: string, patch: Partial<OrganizationDomain>): void {
  domainPatches.set(id, { ...domainPatches.get(id), ...patch })
}

export function patchName(id: string, patch: Partial<OrganizationName>): void {
  namePatches.set(id, { ...namePatches.get(id), ...patch })
}

export function recordPairDecision(
  pairId: string,
  decision: DuplicateDecision,
  reviewedBy: string | null,
  note: string | null,
): void {
  pairDecisions.set(pairId, {
    decision,
    reviewed_at: new Date().toISOString(),
    reviewed_by: reviewedBy,
    note,
  })
}

/** Empile une ligne d'audit — ce que `platform.tg_audit()` écrirait. */
export function recordHistory(organizationId: string, entry: OrganizationHistoryEntry): void {
  sessionHistory.set(organizationId, [...(sessionHistory.get(organizationId) ?? []), entry])
}

/** Inscrit la redirection et la ligne de journal d'une fusion. */
export function recordMerge(entry: OrganizationMergeEntry): void {
  mergedInto.set(entry.source_id, entry.target_id)
  mergeEntries.push(entry)
}

// ---------------------------------------------------------------------------
// `org.compute_trust_score()` — § 7, rejouée
// ---------------------------------------------------------------------------

/**
 * Le score de confiance, recalculé.
 *
 * Il n'est PAS une colonne saisie : le worker l'écrit à partir de six signaux
 * pondérés. Le rejouer ici, plutôt que de figer une valeur, fait bouger la liste
 * quand on pose un sceau ou qu'on vérifie un domaine — c'est ce qui se passera
 * en production, au prochain passage du worker.
 */
export function computeTrustScore(organizationId: string): number {
  const organization = effectiveOrganization(organizationId)
  if (!organization) return 0

  const hasVerifiedDomain = effectiveDomains().some(
    (d) => d.organization_id === organizationId && d.verified_at !== null,
  )
  const activeMembers = effectiveMemberships().filter(
    (m) => m.organization_id === organizationId && m.status === 'active',
  ).length

  return Math.min(
    100,
    (organization.verified_at !== null ? 40 : 0) +
      (hasVerifiedDomain ? 25 : 0) +
      (organization.website !== null ? 5 : 0) +
      (organization.country_id !== null ? 5 : 0) +
      (organization.description !== null ? 5 : 0) +
      Math.min(20, 5 * activeMembers),
  )
}

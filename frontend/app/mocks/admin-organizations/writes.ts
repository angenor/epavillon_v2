/**
 * LES CINQ ÉCRITURES DES ÉCRANS D'ORGANISATION (A11) — ce que fera l'API, et ce
 * que la base sait déjà faire.
 *
 * ── LA FUSION, ET SON ORDRE D'OPÉRATIONS ────────────────────────────────────
 *
 * `org.merge_organizations(source, target, reason)` ne prend AUCUN choix de
 * champ. Les valeurs retenues champ par champ sont donc un `UPDATE` de la fiche
 * CIBLE, appliqué AVANT l'appel et dans la MÊME transaction — sinon une fusion
 * qui échoue laisse une fiche absorbante à moitié complétée, avec le nom de
 * l'autre et rien d'autre. L'ordre est celui-ci, et il n'est pas
 * interchangeable :
 *
 *   1. `UPDATE org.organizations SET … WHERE id = target`  (les choix)
 *   2. `SELECT org.merge_organizations(source, target, reason)`
 *   3. la fonction consigne `merge_log`, passe la source en `merged`, marque la
 *      paire `decision = 'merged'` et publie `org.organization.merged`.
 *
 * L'étape 3 n'est pas à écrire : elle appartient à la fonction. C'est aussi
 * pourquoi l'API n'a PAS à mettre la file à jour elle-même — le faire deux fois
 * finirait par diverger.
 *
 * ── CE QUE CE FICHIER REFUSE, ET POURQUOI ───────────────────────────────────
 *
 * Trois refus, tous portés par la base ou par le prompt :
 *   · `confirmation_mismatch` — le nom saisi ne désigne pas la fiche absorbée.
 *     Vérifié ICI et pas seulement dans l'écran : masquer un bouton n'a jamais
 *     empêché une requête ;
 *   · `already_merged` — `tg_forbid_merge_chains` refuse une cible elle-même
 *     fusionnée (« Cibler la fiche finale ») ; une fusion concurrente a pu passer
 *     entre l'ouverture de l'écran et la validation ;
 *   · `not_found` — l'une des deux fiches n'existe pas.
 * Aucun autre : une fusion n'est jamais refusée parce que les fiches se
 * ressemblent peu. C'est un arbitrage humain, pas une règle d'intégrité.
 *
 * ── LE SCEAU ET LE DOMAINE ──────────────────────────────────────────────────
 *
 * Poser un sceau ou vérifier un domaine change le score de confiance
 * (`compute_trust_score()` : 40 points et 25 points), donc le rang de la fiche
 * dans la liste triée par score. Ces écritures rendent la fiche ENTIÈRE
 * recomposée, pour la même raison qu'en A10 : rendre le seul objet modifié
 * laisserait trois panneaux afficher des valeurs fausses.
 *
 * `ux_organization_domains_verified` n'autorise qu'UNE organisation par domaine
 * vérifié. Vérifier `osed-sahel.org` sur la fiche en doublon alors que la fiche
 * de référence le porte déjà est donc refusé par la base — et l'écran doit dire
 * laquelle le détient, sans quoi le refus est incompréhensible.
 */

import type {
  DomainVerificationPayload,
  DuplicateDecisionPayload,
  DuplicateDecisionResult,
  MergeField,
  MergePayload,
  MergeResult,
  NameConfirmationPayload,
  OrganizationVerificationPayload,
  OrganizationWriteResult,
} from '~/types/admin-organizations'
import type { Organization } from '~/types/org'
import { duplicateCandidates } from '../org'
import { MERGE_LOG } from '../ids'
import { people } from '../people'
import {
  effectiveDomains,
  effectiveOrganization,
  patchDomain,
  patchName,
  patchOrganization,
  recordHistory,
  recordMerge,
  recordPairDecision,
  reopenPair,
  resolveOrganizationId,
  sessionMergeEntries,
} from './session'
import { duplicateQueue, mergePreview, referenceKey } from './duplicates'
import { organizationDetail } from './detail'
import { isMergeConfirmationValid } from '~/utils/organization-merge'

/** Nom lisible d'un acteur, tel que l'audit le dénormalise. */
function actorLabel(actorId: string | null): string | null {
  if (!actorId) return null
  return people.find((p) => p.id === actorId)?.display_name ?? null
}

/**
 * La fiche recomposée après une écriture. Une fiche devenue introuvable entre
 * l'écriture et la relecture n'est pas un enregistrement à moitié réussi : la
 * réponse le dit, plutôt que de rendre un succès sans fiche.
 */
function saved(organizationId: string): OrganizationWriteResult {
  const detail = organizationDetail(organizationId)
  return detail ? { status: 'saved', detail } : { status: 'not_found' }
}

// ---------------------------------------------------------------------------
// 1. La fusion
// ---------------------------------------------------------------------------

/**
 * FUSIONNER DEUX FICHES.
 *
 * La fiche absorbée SURVIT en statut `merged`, avec son pointeur : c'est ce qui
 * fait que « les anciennes adresses continueront de fonctionner », et l'écran le
 * promet à l'opérateur avant qu'il valide. Rien n'est supprimé, sinon les
 * rattachements que la cible portait déjà en double.
 */
export function mergeOrganizations(payload: MergePayload, actorId: string | null): MergeResult {
  const preview = mergePreview(payload.source_id, payload.target_id, payload.pair_id)
  if (!preview) {
    const source = effectiveOrganization(payload.source_id)
    const target = effectiveOrganization(payload.target_id)
    if (!source || !target) return { status: 'not_found' }
    // Les deux fiches existent, mais l'une d'elles est déjà absorbée : c'est le
    // refus de `tg_forbid_merge_chains`, pas une fiche introuvable. Le message
    // est celui du déclencheur, mot pour mot — l'API le reprend tel quel.
    return {
      status: 'already_merged',
      target: resolveOrganizationId(payload.target_id),
      message: `Fusion impossible : la fiche cible ${payload.target_id} est elle-même fusionnée. Cibler la fiche finale.`,
    }
  }

  if (!isMergeConfirmationValid(payload.confirmation_name, preview.source)) {
    return { status: 'confirmation_mismatch' }
  }

  const now = new Date().toISOString()
  const label = actorLabel(actorId)

  // 1. Les choix de champ, appliqués à la CIBLE. Sans choix, la valeur de la
  //    cible reste : c'est elle qui survit, ne rien décider n'écrase rien.
  const patch: Partial<Organization> = {}
  const applied: MergeField[] = []

  for (const comparison of preview.comparisons) {
    if (payload.field_choices[comparison.field] !== 'source') continue
    if (!comparison.differs) continue

    Object.assign(patch, { [comparison.field]: comparison.source_value })
    applied.push(comparison.field)
    recordHistory(payload.target_id, {
      occurred_at: now,
      actor_id: actorId,
      actor_label: label,
      action: 'update',
      field: comparison.field,
      old_value: comparison.target_value,
      new_value: comparison.source_value,
    })
  }
  if (applied.length > 0) patchOrganization(payload.target_id, patch)

  // 2. La fusion elle-même. Le décompte rendu est celui du registre, à la clé
  //    près que `merge_log.rows_reassigned` emploie.
  const rows: Record<string, number> = {}
  for (const line of preview.transfers) {
    const moved = line.strategy === 'delete' ? line.deleted : line.reassigned
    if (moved > 0) rows[referenceKey(line)] = moved
  }

  // 3. La source passe en `merged` : elle reste consultable, avec son pointeur.
  patchOrganization(payload.source_id, {
    status: 'merged',
    merged_into_id: payload.target_id,
    merged_at: now,
  })
  recordHistory(payload.source_id, {
    occurred_at: now,
    actor_id: actorId,
    actor_label: label,
    action: 'update',
    field: 'status',
    old_value: preview.source.status,
    new_value: 'merged',
  })

  recordMerge({
    id: MERGE_LOG(900 + sessionMergeEntries().length),
    source_id: payload.source_id,
    source_name: preview.source.legal_name,
    target_id: payload.target_id,
    target_name: preview.target.legal_name,
    performed_by_name: label,
    performed_at: now,
    rows_reassigned: rows,
    reason: payload.reason,
  })

  // La paire est marquée par la FONCTION, pas par l'appelant : le faire deux
  // fois finirait par diverger.
  const pair =
    payload.pair_id ??
    duplicateCandidates.find(
      (candidate) =>
        (candidate.left_id === payload.source_id && candidate.right_id === payload.target_id) ||
        (candidate.left_id === payload.target_id && candidate.right_id === payload.source_id),
    )?.id
  if (pair) recordPairDecision(pair, 'merged', actorId, payload.reason)

  return { status: 'merged', target: payload.target_id, rows_reassigned: rows, fields_applied: applied }
}

// ---------------------------------------------------------------------------
// 2. « Ce ne sont pas des doublons »
// ---------------------------------------------------------------------------

/**
 * RETIRER UNE PAIRE DE LA FILE, sans rien fusionner.
 *
 * `distinct` dit « ces deux entités sont réellement différentes, ne plus les
 * proposer » ; `deferred` dit « pas maintenant ». Le modèle porte les deux
 * (`ck` sur `duplicate_candidates.decision`), et les confondre reviendrait à
 * perdre la distinction entre une décision et un report — la paire reportée doit
 * revenir, la paire écartée non.
 */
export function decideDuplicatePair(
  payload: DuplicateDecisionPayload,
  actorId: string | null,
): DuplicateDecisionResult {
  const exists = duplicateCandidates.some((candidate) => candidate.id === payload.pair_id)
  if (!exists) return { status: 'not_found' }

  const avant =
    duplicateQueue().settled.find((entry) => entry.id === payload.pair_id) ?? null

  // REMETTRE DANS LA FILE. Un report posé sur une paire DÉJÀ SORTIE de la file
  // l'y ramène — écartée comme reportée. C'est le geste « réexaminer », et il
  // vaut d'abord pour les paires écartées : on se trompe en écartant, pas en
  // reportant. Le tri se faisant sur `reviewed_at`, réenregistrer une décision
  // laisserait la paire rangée et le bouton mentirait (défaut du 20/08).
  //
  // La fusion, elle, ne se reprend pas : l'API répond par un refus, et l'écran
  // n'offre pas le bouton sur une paire fusionnée.
  if (payload.decision === 'deferred' && avant && avant.decision !== 'merged') {
    reopenPair(payload.pair_id)
  } else {
    recordPairDecision(payload.pair_id, payload.decision, actorId, payload.note)
  }

  const queue = duplicateQueue()
  const pair =
    queue.settled.find((entry) => entry.id === payload.pair_id) ??
    queue.pending.find((entry) => entry.id === payload.pair_id) ??
    null

  // La paire arbitrée accompagne TOUJOURS la réponse : ne pas la retrouver
  // signifie qu'elle n'existe pas, et non qu'un arbitrage s'est perdu.
  return pair ? { status: 'recorded', pair } : { status: 'not_found' }
}

// ---------------------------------------------------------------------------
// 3. Le sceau de vérification
// ---------------------------------------------------------------------------

/**
 * POSER OU RETIRER LE SCEAU DE L'IFDD.
 *
 * `verified_at` et `verified_by` vont ensemble : un sceau sans auteur ne
 * permettrait pas de dire qui en répond. La base les porte tous deux, et le
 * retrait remet les deux à nul plutôt que de garder un auteur orphelin.
 */
export function setOrganizationVerification(
  payload: OrganizationVerificationPayload,
  actorId: string | null,
): OrganizationWriteResult {
  const organization = effectiveOrganization(payload.organization_id)
  if (!organization) return { status: 'not_found' }

  const now = new Date().toISOString()
  patchOrganization(payload.organization_id, {
    verified_at: payload.verified ? now : null,
    verified_by: payload.verified ? actorId : null,
    // Une fiche qu'on vérifie sort de l'état `candidate` : on ne pose pas un
    // sceau sur une fiche qu'on n'a pas encore admise.
    status: payload.verified && organization.status === 'candidate' ? 'active' : organization.status,
    updated_at: now,
  })
  recordHistory(payload.organization_id, {
    occurred_at: now,
    actor_id: actorId,
    actor_label: actorLabel(actorId),
    action: 'update',
    field: 'verified_at',
    old_value: organization.verified_at,
    new_value: payload.verified ? now : null,
  })

  return saved(payload.organization_id)
}

// ---------------------------------------------------------------------------
// 4. La vérification d'un domaine
// ---------------------------------------------------------------------------

/**
 * VÉRIFIER UN DOMAINE À LA MAIN, ET OUVRIR OU FERMER LE RATTACHEMENT AUTOMATIQUE.
 *
 * `verification_method` vaut `manual` : les deux autres méthodes — jeton DNS et
 * défi par courriel — appartiennent au worker et ne se déclenchent pas depuis un
 * bouton du back-office.
 *
 * `ck_domain_autojoin_requires_verification` interdit `auto_join` sans
 * vérification : retirer la vérification retire donc le rattachement
 * automatique, sans quoi la base refuserait l'écriture. C'est la contrainte qui
 * décide, pas l'écran.
 */
export function setDomainVerification(
  payload: DomainVerificationPayload,
  actorId: string | null,
): OrganizationWriteResult {
  const domains = effectiveDomains()
  const domain = domains.find((d) => d.id === payload.domain_id)
  if (!domain) return { status: 'not_found' }

  // Un domaine vérifié appartient à UNE organisation — `ux_organization_domains_verified`.
  if (payload.verified && domain.verified_at === null) {
    const holder = domains.find(
      (other) =>
        other.domain === domain.domain &&
        other.verified_at !== null &&
        other.organization_id !== payload.organization_id,
    )
    if (holder) {
      return {
        status: 'domain_taken',
        conflict_with: {
          organization_id: holder.organization_id,
          legal_name: effectiveOrganization(holder.organization_id)?.legal_name ?? '',
        },
      }
    }
  }

  const now = new Date().toISOString()
  patchDomain(payload.domain_id, {
    verified_at: payload.verified ? (domain.verified_at ?? now) : null,
    verification_method: payload.verified ? (domain.verification_method ?? 'manual') : null,
    auto_join: payload.verified && payload.auto_join,
  })
  recordHistory(payload.organization_id, {
    occurred_at: now,
    actor_id: actorId,
    actor_label: actorLabel(actorId),
    action: 'update',
    field: 'organization_domains.verified_at',
    old_value: domain.verified_at,
    new_value: payload.verified ? (domain.verified_at ?? now) : null,
  })

  return saved(payload.organization_id)
}

// ---------------------------------------------------------------------------
// 5. La confirmation d'une dénomination
// ---------------------------------------------------------------------------

/**
 * CONFIRMER OU INFIRMER UNE DÉNOMINATION.
 *
 * `is_confirmed` ne décide pas si la dénomination SERT à la recherche — elle y
 * sert toujours, confirmée ou non, et c'est ce qui permet de retrouver une fiche
 * par une faute d'orthographe connue. Elle décide de son AFFICHAGE : une
 * variante saisie à l'import ne s'affiche pas tant qu'un administrateur ne l'a
 * pas regardée. Les confondre reviendrait à faire disparaître de la recherche ce
 * qu'on voulait seulement ne pas montrer.
 */
export function setNameConfirmation(
  payload: NameConfirmationPayload,
  actorId: string | null,
): OrganizationWriteResult {
  const detail = organizationDetail(payload.organization_id)
  const name = detail?.names.find((entry) => entry.id === payload.name_id)
  if (!detail || !name) return { status: 'not_found' }

  patchName(payload.name_id, { is_confirmed: payload.is_confirmed })
  recordHistory(payload.organization_id, {
    occurred_at: new Date().toISOString(),
    actor_id: actorId,
    actor_label: actorLabel(actorId),
    action: 'update',
    field: 'organization_names.is_confirmed',
    old_value: name.is_confirmed,
    new_value: payload.is_confirmed,
  })

  return saved(payload.organization_id)
}

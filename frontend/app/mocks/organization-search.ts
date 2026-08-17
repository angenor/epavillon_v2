/**
 * RECHERCHE ET CRÉATION D'ORGANISATION — ce que fera l'API au prompt B2, et que
 * la base sait déjà faire.
 *
 * Ce fichier ne contient AUCUNE donnée nouvelle : il rejoue en TypeScript
 * `org.find_similar_organizations()` (`040_organizations.sql` § 5) sur les fiches
 * de `org.ts`, plus les deux écritures de l'écran de rattachement (A2). Il est à
 * `org.ts` ce que `auth.ts` est à `people.ts`.
 *
 * POURQUOI REJOUER LA FONCTION PLUTÔT QU'ÉCRIRE DES RÉSULTATS À LA MAIN. Trois
 * résultats figés se seraient contentés d'être plausibles ; l'écran aurait été
 * réglé sur eux, et le jour du branchement (B7) il aurait fallu tout reprendre.
 * Ici, taper « institut » sur le formulaire produit exactement ce que produirait
 * la base : les mêmes fiches, dans le même ordre, avec les mêmes scores. Les
 * seuils, les pondérations et les bonus sont ceux du SQL, et rien d'autre.
 *
 * LES TROIS SEUILS VIENNENT DE POSTGRESQL, PAS D'UNE ESTIMATION :
 *   · 0,3  — `pg_trgm.similarity_threshold`, l'opérateur `%` ;
 *   · 0,6  — `pg_trgm.word_similarity_threshold`, l'opérateur `<%` ;
 *   · 85   — le seuil de « correspondance forte », commenté dans le § 5.
 * Les deux premiers ont été relevés sur la base (`SELECT show_limit()`).
 *
 * CE QUI RESTE UNE APPROXIMATION, ET C'EST DIT : `word_similarity` explore, dans
 * PostgreSQL, toutes les extensions de mots de la chaîne cible. On se contente
 * ici des fenêtres de mots contiguës — assez pour retrouver « institut » dans
 * « institut de la francophonie… », qui est le cas dont dépend l'écran, et
 * vérifié contre la vraie fonction sur les fiches de démonstration.
 */

import type {
  Membership,
  Organization,
  OrganizationDomain,
  OrganizationName,
  SimilarOrganization,
} from '~/types/org'
import type {
  CreateOrganizationPayload,
  CreateOrganizationResult,
  EmailDomainMatch,
  JoinOrganizationPayload,
  JoinOrganizationResult,
  OrganizationSearchQuery,
} from '~/types/organization-join'
import { MEMBERSHIP, ORG_CREATED } from './ids'
import { memberships } from './memberships'
import { organizationDomains, organizationNames, organizations } from './org'
import { people } from './people'

// ---------------------------------------------------------------------------
// Les utilitaires de `000_bootstrap.sql`
// ---------------------------------------------------------------------------

/**
 * `platform.normalize_label()` : minuscules, sans accents, sans ponctuation,
 * espaces réduits. C'est la forme sur laquelle porte TOUTE la comparaison — la
 * base indexe `name_normalized`, pas `name`.
 */
export function normalizeLabel(value: string | null | undefined): string {
  return (value ?? '')
    .normalize('NFD')
    // Diacritiques décomposés, retirés : « é » devient « e », comme `unaccent`.
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

/** `platform.extract_domain()` : le domaine d'une adresse ou d'une URL. */
export function extractDomain(value: string | null | undefined): string {
  return (value ?? '')
    .replace(/^\s*(https?:\/\/)?(www\.)?/i, '')
    .replace(/^[^@]*@/, '')
    .replace(/[/?#].*$/, '')
    .toLowerCase()
    .trim()
}

/**
 * `org.public_email_domains` — les messageries grand public, reprises telles
 * quelles du § 3. Un domaine qui y figure ne prouve aucune appartenance : deux
 * ONG ne sont pas la même parce que leurs référents utilisent Gmail.
 */
export const publicEmailDomains: readonly string[] = [
  'gmail.com', 'yahoo.com', 'yahoo.fr', 'hotmail.com', 'hotmail.fr',
  'outlook.com', 'outlook.fr', 'live.com', 'live.fr', 'icloud.com',
  'protonmail.com', 'proton.me', 'orange.fr', 'wanadoo.fr', 'free.fr',
  'aol.com', 'gmx.com', 'mail.com', 'yandex.com', 'zoho.com',
]

// ---------------------------------------------------------------------------
// Les mesures de `pg_trgm`
// ---------------------------------------------------------------------------

/** Seuil de l'opérateur `%` — `pg_trgm.similarity_threshold`. */
const SIMILARITY_THRESHOLD = 0.3
/** Seuil de l'opérateur `<%` — `pg_trgm.word_similarity_threshold`. */
const WORD_SIMILARITY_THRESHOLD = 0.6
/** Pondération d'une correspondance sur un FRAGMENT du nom (voir le § 5). */
const WORD_MATCH_WEIGHT = 0.6
/** Score plancher d'une correspondance par PRÉFIXE, deux ou trois lettres comprises. */
const PREFIX_MATCH_SCORE = 0.55

/**
 * Trigrammes d'une chaîne, à la façon de `show_trgm()` : chaque mot est encadré
 * de deux espaces devant et d'un derrière, puis découpé par trois.
 * `show_trgm('osed')` donne bien `{"  o"," os","ose","sed","ed "}`.
 */
function trigrams(value: string): Set<string> {
  const out = new Set<string>()
  for (const word of value.split(' ')) {
    if (word.length === 0) continue
    const padded = `  ${word} `
    for (let i = 0; i + 3 <= padded.length; i += 1) out.add(padded.slice(i, i + 3))
  }
  return out
}

/** `similarity(a, b)` : trigrammes communs sur trigrammes réunis. */
function similarity(a: string, b: string): number {
  const left = trigrams(a)
  const right = trigrams(b)
  if (left.size === 0 || right.size === 0) return 0
  let shared = 0
  for (const t of left) if (right.has(t)) shared += 1
  return shared / (left.size + right.size - shared)
}

/**
 * `word_similarity(q, cible)` : le terme ressemble-t-il à un ou plusieurs MOTS
 * de la cible ?
 *
 * LA DIVISION N'EST PAS CELLE DE `similarity`, et c'est tout l'intérêt de cette
 * mesure. PostgreSQL rapporte les trigrammes communs au nombre de trigrammes de
 * la REQUÊTE seule, non à leur réunion : la longueur du nom cible ne pénalise
 * donc plus le terme court. « franco » retrouve « francophonie » à 6/7 ≈ 0,86,
 * là où `similarity` tombait à 0,43 et passait sous tous les seuils.
 *
 * Écrit d'abord avec la division de `similarity`, ce qui donnait 0 sur « franco »
 * quand la base rendait 51,4 : l'écart a été trouvé en comparant les deux, terme
 * par terme, et non en relisant le code.
 */
function wordSimilarity(query: string, target: string): number {
  const queryTrigrams = trigrams(query)
  if (queryTrigrams.size === 0) return 0

  const words = target.split(' ').filter(Boolean)
  if (words.length === 0) return 0

  // Fenêtres contiguës — l'approximation annoncée en tête de fichier. Une
  // fenêtre un peu plus longue que la requête laisse la place aux mots-outils
  // (« de », « la ») que le terme saisi n'a pas.
  const span = query.split(' ').filter(Boolean).length + 1
  let best = 0
  for (let start = 0; start < words.length; start += 1) {
    for (let length = 1; length <= span && start + length <= words.length; length += 1) {
      const windowTrigrams = trigrams(words.slice(start, start + length).join(' '))
      let shared = 0
      for (const t of queryTrigrams) if (windowTrigrams.has(t)) shared += 1
      best = Math.max(best, shared / queryTrigrams.size)
    }
  }
  return best
}

// ---------------------------------------------------------------------------
// Le journal d'écritures de la session de démonstration
//
// LES MOCKS RESTENT FIGÉS ; CE JOURNAL NE L'EST PAS, et c'est une exception
// assumée — la seconde du projet après les jetons datés relativement à
// maintenant (`mocks/auth.ts`).
//
// POURQUOI. Le rattachement à une organisation n'est pas un écran isolé : c'est
// une ÉTAPE, dont d'autres actions dépendent (`middleware/requires-organization`).
// Sans persistance, rejoindre une organisation puis reprendre l'action
// interrompue ramenait à l'écran de rattachement — la garde relisait des données
// où rien n'avait été écrit. La démonstration donnait donc à voir une boucle qui
// n'existe pas, et le seul moyen de la lever aurait été de ne pas éprouver la
// garde du tout.
//
// CE QUE ÇA N'EST PAS. Rien n'est écrit sur disque ni dans le navigateur : ce
// journal vit dans le module, donc jusqu'au prochain rechargement de la page.
// C'est exactement la durée d'une démonstration, et cela garde vraie la phrase
// « les données simulées sont en lecture seule » — on ne modifie AUCUNE des
// lignes écrites à la main, on en empile de nouvelles par-dessus.
// ---------------------------------------------------------------------------

/** Adhésions créées pendant la session courante. */
const sessionMemberships: Membership[] = []
/** Fiches créées pendant la session courante. */
const sessionOrganizations: Organization[] = []

/** Les treize fiches du jeu, plus celles créées depuis. */
export function organizationsWithSession(): Organization[] {
  return [...organizations, ...sessionOrganizations]
}

/** Une fiche par son identifiant, créations de la session comprises. */
export function organizationById(id: string): Organization | null {
  return organizationsWithSession().find((o) => o.id === id) ?? null
}

/**
 * Les dénominations, celles du jeu et celles que le trigger
 * `org.tg_sync_organization_names` créerait pour une fiche nouvelle : son nom
 * légal et son sigle, tous deux confirmés. C'est la base qui les écrit, pas
 * l'application — les reproduire ici évite qu'une fiche créée pendant la
 * démonstration soit introuvable par la recherche qui vient de la refuser.
 */
function namesWithSession(): OrganizationName[] {
  const derived: OrganizationName[] = []
  for (const organization of sessionOrganizations) {
    derived.push(sessionName(organization.id, organization.legal_name, 'legal'))
    if (organization.acronym) derived.push(sessionName(organization.id, organization.acronym, 'acronym'))
  }
  return [...organizationNames, ...derived]
}

function sessionName(organizationId: string, name: string, kind: OrganizationName['kind']): OrganizationName {
  return {
    id: `${organizationId}-${kind}`,
    organization_id: organizationId,
    name,
    name_normalized: normalizeLabel(name),
    kind,
    locale: null,
    is_confirmed: true,
    created_by: null,
    created_at: new Date().toISOString(),
  }
}

/** Toutes les adhésions, celles du jeu et celles de la session. */
function allMemberships(): Membership[] {
  return [...memberships, ...sessionMemberships]
}

// ---------------------------------------------------------------------------
// `org.find_similar_organizations()`
// ---------------------------------------------------------------------------

/** Ordre de préférence des dénominations à égalité de score — voir le § 5. */
const KIND_RANK: Record<string, number> = {
  legal: 1, acronym: 2, short: 3, translation: 4, former: 5, misspelling: 6,
}

/** Adhésions ACTIVES d'une organisation. Les demandes en attente ne comptent pas. */
export function activeMemberCount(organizationId: string): number {
  return allMemberships().filter((m) => m.organization_id === organizationId && m.status === 'active').length
}

export function findSimilarOrganizations(query: OrganizationSearchQuery): SimilarOrganization[] {
  const q = normalizeLabel(query.name)
  if (q.length === 0) return []

  const rawDomain = extractDomain(query.website ?? query.email ?? '')
  const usableDomain = rawDomain && !publicEmailDomains.includes(rawDomain) ? rawDomain : ''
  const domainMatches = new Set(
    usableDomain
      ? organizationDomains.filter((d) => d.domain === usableDomain).map((d) => d.organization_id)
      : [],
  )

  /** Meilleure dénomination par organisation : score, puis rang de la dénomination. */
  const best = new Map<string, { score: number; name: string; rank: number }>()

  for (const denomination of namesWithSession()) {
    const normalized = normalizeLabel(denomination.name)
    const whole = similarity(normalized, q)
    const word = wordSimilarity(q, normalized)
    const isPrefix = normalized.startsWith(q)

    // Le WHERE du § 5 : l'un des trois signaux suffit à retenir la ligne.
    if (whole < SIMILARITY_THRESHOLD && word < WORD_SIMILARITY_THRESHOLD && !isPrefix) continue

    const score = Math.max(
      whole,
      word * WORD_MATCH_WEIGHT,
      isPrefix ? PREFIX_MATCH_SCORE : 0,
    )
    const rank = (KIND_RANK[denomination.kind] ?? 6) + (denomination.is_confirmed ? 0 : 10)
    const current = best.get(denomination.organization_id)
    // Meilleur score ; à score égal, la dénomination la mieux placée. Sans ce
    // départage, l'écran annoncerait « trouvée sous : <faute d'orthographe> »
    // pour un résultat par ailleurs juste — voir le § 5 du SQL.
    if (!current || score > current.score || (score === current.score && rank < current.rank)) {
      best.set(denomination.organization_id, { score, name: denomination.name, rank })
    }
  }

  // Une fiche trouvée par son SEUL domaine entre aussi dans les candidats, sans
  // dénomination correspondante : c'est le signal le plus fiable du modèle.
  for (const organizationId of domainMatches) {
    if (!best.has(organizationId)) best.set(organizationId, { score: 0, name: '', rank: 99 })
  }

  const results: SimilarOrganization[] = []

  for (const [organizationId, match] of best) {
    const organization = organizationById(organizationId)
    if (!organization) continue
    if (organization.status !== 'active' && organization.status !== 'candidate') continue

    const sharedDomain = domainMatches.has(organizationId)
    const sameCountry = Boolean(query.country_id) && organization.country_id === query.country_id
    const acronymMatch = normalizeLabel(organization.acronym) === q && q.length > 0

    const score = Math.round(
      (match.score * 100 + (sharedDomain ? 40 : 0) + (sameCountry ? 10 : 0) + (acronymMatch ? 25 : 0)) * 10,
    ) / 10

    results.push({
      organization_id: organization.id,
      legal_name: organization.legal_name,
      acronym: organization.acronym,
      organization_type_code: organization.organization_type_code,
      country_id: organization.country_id,
      city: organization.city,
      status: organization.status,
      verified_at: organization.verified_at,
      member_count: activeMemberCount(organization.id),
      matched_name: match.name === '' ? null : match.name,
      score,
      match_reasons: [
        match.score > SIMILARITY_THRESHOLD ? 'name_similarity' : null,
        sharedDomain ? 'shared_domain' : null,
        sameCountry ? 'same_country' : null,
        acronymMatch ? 'acronym_match' : null,
      ].filter((reason): reason is string => reason !== null),
    })
  }

  return results
    .sort((a, b) => b.score - a.score || a.legal_name.localeCompare(b.legal_name, 'fr'))
    .slice(0, query.limit ?? 10)
}

// ---------------------------------------------------------------------------
// Ce que le domaine d'une adresse révèle
// ---------------------------------------------------------------------------

/**
 * Fiche portant le domaine de cette adresse. `null` pour une messagerie grand
 * public : c'est la règle du § 3, et elle prime sur toute ressemblance.
 *
 * Quand DEUX fiches portent le domaine — ce qui est exactement le cas des deux
 * OSED —, la vérifiée l'emporte. Proposer la fiche en doublon reviendrait à
 * l'alimenter, et l'écran fabriquerait le désordre qu'il est là pour éviter.
 */
export function organizationForEmail(email: string | null | undefined): EmailDomainMatch | null {
  const domain = extractDomain(email)
  if (domain.length === 0 || publicEmailDomains.includes(domain)) return null

  const candidates = organizationDomains
    .filter((d) => d.domain === domain)
    .sort(byVerifiedThenAutoJoin)

  for (const record of candidates) {
    const organization = organizationById(record.organization_id)
    if (!organization) continue
    if (organization.status !== 'active' && organization.status !== 'candidate') continue

    return {
      domain,
      organization,
      domain_record: record,
      can_auto_join: record.auto_join && record.verified_at !== null,
      member_count: activeMemberCount(organization.id),
    }
  }
  return null
}

function byVerifiedThenAutoJoin(a: OrganizationDomain, b: OrganizationDomain): number {
  const verified = Number(b.verified_at !== null) - Number(a.verified_at !== null)
  return verified !== 0 ? verified : Number(b.auto_join) - Number(a.auto_join)
}

// ---------------------------------------------------------------------------
// Les deux écritures de l'écran
//
// Rien n'est écrit : les données simulées sont en lecture seule. Ces fonctions
// calculent la RÉPONSE que rendra l'API — c'est ce dont l'écran a besoin pour se
// comporter juste, et la seule chose qu'il puisse en observer.
// ---------------------------------------------------------------------------

/** Adhésion existante d'une personne à une organisation, révocations exclues. */
export function membershipOf(personId: string, organizationId: string): Membership | null {
  return (
    allMemberships().find(
      (m) => m.person_id === personId && m.organization_id === organizationId && m.status !== 'revoked',
    ) ?? null
  )
}

/** Adhésions vivantes d'une personne — actives ou en attente d'un référent. */
export function membershipsOfPerson(personId: string): Membership[] {
  return allMemberships().filter((m) => m.person_id === personId && m.status !== 'revoked')
}

/**
 * Demande de rattachement.
 *
 * L'ISSUE DÉPEND DU DOMAINE, PAS DE LA VOLONTÉ DE L'UTILISATEUR. Une adresse
 * portant un domaine vérifié et marqué `auto_join` (§ 3 : « un agent qui
 * s'inscrit avec une adresse @ifdd.francophonie.org rejoint l'organisation sans
 * intervention humaine ») ouvre une adhésion ACTIVE. Partout ailleurs, un
 * référent doit accepter : l'adhésion naît `pending`, ce que l'écran doit dire
 * sans détour plutôt que de laisser croire l'affaire réglée.
 */
export function joinOrganization(personId: string, payload: JoinOrganizationPayload): JoinOrganizationResult {
  const organization = organizations.find((o) => o.id === payload.organization_id)
  if (!organization) throw new Error(`Organisation ${payload.organization_id} introuvable.`)

  const existing = membershipOf(personId, organization.id)
  if (existing) {
    return {
      status: 'already_member',
      organization,
      membership_status: existing.status === 'active' ? 'active' : 'pending',
    }
  }

  const person = people.find((p) => p.id === personId)
  const domainMatch = organizationForEmail(person?.primary_email)
  const auto = domainMatch !== null && domainMatch.organization.id === organization.id && domainMatch.can_auto_join

  const now = new Date().toISOString()
  const membership: Membership = {
    id: MEMBERSHIP(900 + sessionMemberships.length),
    organization_id: organization.id,
    person_id: personId,
    // Qui rejoint est membre ; le rôle de référent se gagne en créant la fiche
    // ou s'accorde depuis l'espace organisation.
    role: 'member',
    status: auto ? 'active' : 'pending',
    // `tg_default_primary_membership` : la première adhésion active devient la
    // principale. On reproduit la règle, on ne la réinvente pas.
    is_primary: auto && !membershipsOfPerson(personId).some((m) => m.status === 'active'),
    job_title: payload.job_title,
    approved_by: null,
    approved_at: auto ? now : null,
    revoked_at: null,
    created_at: now,
    updated_at: now,
  }
  sessionMemberships.push(membership)

  return {
    status: auto ? 'joined' : 'pending',
    membership_id: membership.id,
    organization,
  }
}

/**
 * Création d'une organisation.
 *
 * LA SEULE ISSUE D'ÉCHEC EST LE DOUBLON EXACT, celui que la base refuse
 * elle-même (`ux_organizations_name_country` : même nom normalisé, même pays,
 * fiche vivante). Une simple ressemblance ne bloque RIEN : l'écran a montré ce
 * qui existait, la personne a maintenu, la fiche est créée — en `candidate`,
 * pour que l'IFDD la regarde. Rendre l'erreur visible, pas l'empêcher.
 */
export function createOrganization(personId: string, payload: CreateOrganizationPayload): CreateOrganizationResult {
  const normalized = normalizeLabel(payload.legal_name)

  const clash = organizationsWithSession().find(
    (o) =>
      normalizeLabel(o.legal_name) === normalized &&
      o.country_id === payload.country_id &&
      (o.status === 'active' || o.status === 'candidate'),
  )
  if (clash) {
    return {
      status: 'name_taken',
      existing: {
        organization_id: clash.id,
        legal_name: clash.legal_name,
        acronym: clash.acronym,
        organization_type_code: clash.organization_type_code,
        country_id: clash.country_id,
        city: clash.city,
        status: clash.status,
        verified_at: clash.verified_at,
        member_count: activeMemberCount(clash.id),
        matched_name: clash.legal_name,
        score: 100,
        match_reasons: ['name_similarity', 'same_country'],
      },
    }
  }

  const now = new Date().toISOString()
  const organization: Organization = {
    // Identifiant de la fiche que l'API créerait. Réservé au-delà des treize
    // fiches du jeu, pour qu'il ne recouvre jamais l'une d'elles.
    id: ORG_CREATED,
    legal_name: payload.legal_name,
    legal_name_normalized: normalized,
    acronym: payload.acronym,
    acronym_normalized: payload.acronym ? normalizeLabel(payload.acronym) : null,
    slug: normalized.replace(/ /g, '-').slice(0, 160),
    organization_type_code: payload.organization_type_code,
    country_id: payload.country_id,
    city: payload.city,
    description: payload.description,
    website: payload.website,
    contact_email: null,
    contact_phone: null,
    // JAMAIS `active` : une fiche créée depuis un formulaire public attend d'être
    // regardée. C'est ce qui alimente la file de dédoublonnage du back-office.
    status: 'candidate',
    merged_into_id: null,
    merged_at: null,
    verified_at: null,
    verified_by: null,
    // `org.compute_trust_score()`, § 7 : ni sceau (0), ni domaine vérifié (0),
    // site 5, pays 5, description 5, et 5 pour l'unique membre — la personne qui
    // vient de la créer.
    trust_score:
      (payload.website ? 5 : 0) + 5 + (payload.description ? 5 : 0) + 5,
    created_by: personId,
    created_at: now,
    updated_at: now,
  }

  const membership: Membership = {
    id: MEMBERSHIP(950 + sessionOrganizations.length),
    organization_id: organization.id,
    person_id: personId,
    // Qui crée la fiche en devient le référent : personne d'autre ne peut
    // l'approuver, et quelqu'un doit pouvoir accepter les adhésions suivantes.
    role: 'manager',
    status: 'active',
    is_primary: !membershipsOfPerson(personId).some((m) => m.status === 'active'),
    job_title: payload.job_title,
    approved_by: personId,
    approved_at: now,
    revoked_at: null,
    created_at: now,
    updated_at: now,
  }
  sessionOrganizations.push(organization)
  sessionMemberships.push(membership)

  return {
    status: 'created',
    organization,
    membership_id: membership.id,
    role: 'manager',
  }
}


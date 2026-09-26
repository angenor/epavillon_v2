/**
 * Les sessions de négociation officielles : ce que l'écran en affiche.
 *
 * Pur, comme la file et les thématiques : l'état « En cours », le jour ouvert, le
 * filtre et la coupure se trompent en silence, un test doit les prendre sans
 * navigateur. **Tout jour se compte dans le fuseau de la COP**, jamais celui du
 * téléphone : une session de 23:30 à Belém tombe le lendemain à Dakar.
 */
import type { OfficialSession, OfficialSessions } from '~/types/negotiation-sessions'
import { dayKeyInZone, instantFromWallClock } from '../datetime.ts'

export type EtatAffiche = 'prevue' | 'en-cours' | 'deplacee' | 'annulee' | 'terminee'

/** Le type de réunion d'une coordination de groupe (vocabulaire `negotiation_meeting_type`). */
export const TYPE_COORDINATION = 'group_coordination'

export const cleDesSessions = (slug: string) => `sessions:${slug}`

const instant = (iso: string) => new Date(iso).getTime()

/** Le lendemain d'une clé `AAAA-MM-JJ`, calculé sur le calendrier, pas sur 24 heures. */
export function lendemain(jour: string): string {
  const [a, m, j] = jour.split('-').map(Number) as [number, number, number]
  return new Date(Date.UTC(a, m - 1, j + 1)).toISOString().slice(0, 10)
}

/** La fin d'une session ; absente, minuit du jour de son début dans le fuseau de la COP. */
export function finEffective(session: Pick<OfficialSession, 'start_at' | 'end_at'>, fuseau: string): number {
  if (session.end_at) return instant(session.end_at)
  const minuit = instantFromWallClock(`${lendemain(dayKeyInZone(session.start_at, fuseau))} 00:00`, fuseau)
  return minuit ? instant(minuit) : instant(session.start_at)
}

/**
 * Annulée d'abord ; puis l'heure (terminée, en cours) ; « Déplacée » seulement
 * tant qu'elle n'a pas commencé — en cours, c'est l'heure qui importe.
 */
export function etatAffiche(session: OfficialSession, maintenant: Date, fuseau: string): EtatAffiche {
  if (session.status === 'cancelled') return 'annulee'
  const t = maintenant.getTime()
  if (t >= finEffective(session, fuseau)) return 'terminee'
  if (t >= instant(session.start_at)) return 'en-cours'
  return session.previous ? 'deplacee' : 'prevue'
}

export const jourDe = (session: Pick<OfficialSession, 'start_at'>, fuseau: string) =>
  dayKeyInZone(session.start_at, fuseau)

/** Les jours de la bande : ceux qui ont au moins une session, annulées comprises. */
export function joursDeLaBande(sessions: readonly OfficialSession[], fuseau: string): string[] {
  return [...new Set(sessions.map((s) => jourDe(s, fuseau)))].sort()
}

/** FR-003 : aujourd'hui s'il a des sessions, sinon le prochain jour qui en a, sinon le dernier. */
export function jourAOuvrir(jours: readonly string[], maintenant: Date, fuseau: string): string | null {
  if (jours.length === 0) return null
  const aujourdhui = dayKeyInZone(maintenant, fuseau)
  return jours.find((j) => j >= aujourdhui) ?? jours[jours.length - 1] ?? null
}

/** FR-005 : par heure de début actuelle ; une annulée garde sa place. */
export function trierParDebut(sessions: readonly OfficialSession[]): OfficialSession[] {
  return [...sessions].sort(
    (a, b) =>
      instant(a.start_at) - instant(b.start_at) ||
      (a.end_at ? instant(a.end_at) : Infinity) - (b.end_at ? instant(b.end_at) : Infinity) ||
      a.id.localeCompare(b.id),
  )
}

export function sessionsDuJour(sessions: readonly OfficialSession[], jour: string, fuseau: string): OfficialSession[] {
  return trierParDebut(sessions.filter((s) => jourDe(s, fuseau) === jour))
}

export const estCoordination = (s: OfficialSession) => s.group !== null || s.type?.code === TYPE_COORDINATION

export interface Suivis {
  thematiques: readonly string[]
  groupes: readonly string[]
}

/**
 * FR-006 — « Mes thématiques ». Une coordination ne passe que par son groupe :
 * ceux cochés, ou tous s'il n'y en a aucun ; sans groupe reconnu, jamais.
 */
export function passeLeFiltre(session: OfficialSession, suivis: Suivis): boolean {
  if (estCoordination(session)) {
    if (!session.group) return false
    return suivis.groupes.length === 0 || suivis.groupes.includes(session.group.code)
  }
  return session.theme === null || suivis.thematiques.includes(session.theme)
}

/** « Mon groupe » : une coordination d'un groupe coché. */
export const estDeMonGroupe = (session: OfficialSession, groupes: readonly string[]) =>
  session.group !== null && groupes.includes(session.group.code)

export function filtrer(
  sessions: readonly OfficialSession[],
  filtre: 'miennes' | 'toutes',
  suivis: Suivis,
): OfficialSession[] {
  return filtre === 'toutes' ? [...sessions] : sessions.filter((s) => passeLeFiltre(s, suivis))
}

export interface EtatVide {
  /** La prochaine session qui passe le filtre, ni annulée ni commencée ; nulle s'il n'y en a pas. */
  prochain: OfficialSession | null
  /** Les sessions du jour que « Toutes » montrerait. */
  autresDuJour: number
}

/** FR-011 : ce que dit « Mes thématiques » vide. */
export function etatVide(
  sessions: readonly OfficialSession[],
  jour: string,
  suivis: Suivis,
  maintenant: Date,
  fuseau: string,
): EtatVide {
  const t = maintenant.getTime()
  const prochain =
    trierParDebut(sessions).find(
      (s) => s.status !== 'cancelled' && instant(s.start_at) > t && passeLeFiltre(s, suivis),
    ) ?? null
  return { prochain, autresDuJour: sessions.filter((s) => jourDe(s, fuseau) === jour).length }
}

// ---------------------------------------------------------------------------
// La lecture gardée, et la coupure (FR-039, principe XII)
// ---------------------------------------------------------------------------

export interface SessionsGardees {
  slug: string
  /** L'`ETag` de la réponse : la relecture suivante l'envoie en `If-None-Match`. */
  empreinte: string | null
  /** Nulle : l'édition est inconnue de l'import (`404`), ce qui vaut coupé. */
  lues: OfficialSessions | null
}

/**
 * Ce qui remplace la garde après une lecture. Toute réponse la remplace — une
 * coupure aussi, et c'est le point : une liste ancienne ne survit pas à une
 * coupure lue. Un `304` garde la liste et prend l'empreinte.
 */
export function apresLecture(
  garde: SessionsGardees | null,
  slug: string,
  lu: { lues: OfficialSessions | null; empreinte: string | null } | { inchange: true; empreinte: string },
): SessionsGardees {
  if ('inchange' in lu) {
    if (garde?.slug === slug) return { ...garde, empreinte: lu.empreinte }
    throw new Error('304 sans liste gardée')
  }
  const lues = lu.lues && lu.lues.state === 'cut' ? { ...lu.lues, sessions: [] } : lu.lues
  return { slug, empreinte: lu.empreinte, lues }
}

export type AffichageDesSessions =
  | { etat: 'inconnu' }
  | {
      etat: 'coupe'
      raison: 'disabled' | 'unreachable'
      /** Depuis quand la source ne répond plus. */
      depuis: string | null
      /** Nulle si l'édition est inconnue de l'import. */
      programme: string | null
      luA: string | null
    }
  | {
      etat: 'sert'
      sessions: OfficialSession[]
      programme: string
      luA: string | null
      fuseau: string
      ville: string | null
    }

/**
 * **Coupé, rien d'autre** : jamais une session à côté d'une coupure, quel que soit
 * ce que porte la valeur. Une garde d'une autre édition ne compte pas.
 */
export function affichageDesSessions(garde: SessionsGardees | null, slug: string | null): AffichageDesSessions {
  if (!garde || !slug || garde.slug !== slug) return { etat: 'inconnu' }
  const lues = garde.lues
  if (!lues) return { etat: 'coupe', raison: 'disabled', depuis: null, programme: null, luA: null }
  if (lues.state === 'cut') {
    return {
      etat: 'coupe',
      raison: lues.cut_reason ?? 'disabled',
      depuis: lues.failing_since,
      programme: lues.official_programme_url || null,
      luA: lues.read_at,
    }
  }
  return {
    etat: 'sert',
    sessions: lues.sessions,
    programme: lues.official_programme_url,
    luA: lues.read_at,
    fuseau: lues.edition.timezone,
    ville: lues.edition.city,
  }
}

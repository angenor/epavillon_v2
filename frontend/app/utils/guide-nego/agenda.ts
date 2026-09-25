/**
 * « Mon agenda » : chevauchements, prochaine session de « Ma journée », rappel dû,
 * et l'agenda tel qu'il s'affiche avant que le réseau ait répondu.
 *
 * Pur et testé, comme `sessions.ts`. **Une annulée ne chevauche rien, ne se
 * rappelle pas et n'est jamais « la prochaine »** (FR-033, FR-035, FR-036).
 */
import type { MyAgenda, OfficialSession } from '~/types/negotiation-sessions'
import { etatAffiche } from './sessions.ts'

export const CLE_LECTURE_AGENDA = 'mon-agenda'
/** Une clé de file par session : deux intentions sur la même, la dernière gagne. */
export const PREFIXE_FILE_AGENDA = 'agenda-'
export const RAPPEL_AVANT_MS = 15 * 60_000

/** Ce qui part dans la file : dans l'agenda (et le rappel voulu), ou retirée. */
export interface IntentionAgenda {
  garder: boolean
  remind: boolean
}

const instant = (iso: string) => new Date(iso).getTime()

/**
 * Deux créneaux se recouvrent-ils ? **Fin absente : la session vaut son seul
 * début** — on n'invente pas de durée. Elle chevauche ce qui est en cours à son
 * début, ou commence au même instant.
 */
export function serecouvrent(
  a: Pick<OfficialSession, 'start_at' | 'end_at'>,
  b: Pick<OfficialSession, 'start_at' | 'end_at'>,
): boolean {
  const sa = instant(a.start_at)
  const sb = instant(b.start_at)
  const ea = a.end_at ? instant(a.end_at) : null
  const eb = b.end_at ? instant(b.end_at) : null
  if (ea === null && eb === null) return sa === sb
  if (ea === null) return sb <= sa && sa < (eb as number)
  if (eb === null) return sa <= sb && sb < ea
  return sa < eb && sb < ea
}

/** FR-033 : pour chaque session suivie, les autres qui la chevauchent, par début. */
export function chevauchements(sessions: readonly OfficialSession[]): Map<string, OfficialSession[]> {
  const actives = sessions
    .filter((s) => s.status !== 'cancelled')
    .sort((a, b) => instant(a.start_at) - instant(b.start_at))
  const resultat = new Map<string, OfficialSession[]>()
  for (const s of actives) {
    const autres = actives.filter((o) => o.id !== s.id && serecouvrent(s, o))
    if (autres.length > 0) resultat.set(s.id, autres)
  }
  return resultat
}

/** Les sessions de l'agenda, dans l'ordre de leurs débuts ; une entrée sans session lue est tue. */
export function sessionsDeLAgenda(agenda: MyAgenda, sessions: readonly OfficialSession[]): OfficialSession[] {
  const suivies = new Set(agenda.entries.map((e) => e.session_id))
  return sessions.filter((s) => suivies.has(s.id)).sort((a, b) => instant(a.start_at) - instant(b.start_at))
}

export interface ProchaineSession {
  session: OfficialSession
  source: 'agenda' | 'thematiques'
}

/**
 * FR-036 : la prochaine de l'agenda qui n'est ni terminée ni annulée — une en
 * cours compte ; à défaut, la prochaine des thématiques suivies ; sinon rien.
 */
export function prochaineSession(
  agenda: readonly OfficialSession[],
  sessions: readonly OfficialSession[],
  thematiques: readonly string[],
  maintenant: Date,
  fuseau: string,
): ProchaineSession | null {
  const vivante = (s: OfficialSession) => {
    const etat = etatAffiche(s, maintenant, fuseau)
    return etat !== 'terminee' && etat !== 'annulee'
  }
  const premiere = (liste: readonly OfficialSession[]) =>
    [...liste].filter(vivante).sort((a, b) => instant(a.start_at) - instant(b.start_at))[0] ?? null

  const dansLAgenda = premiere(agenda)
  if (dansLAgenda) return { session: dansLAgenda, source: 'agenda' }
  const suivie = premiere(sessions.filter((s) => s.theme !== null && thematiques.includes(s.theme)))
  return suivie ? { session: suivie, source: 'thematiques' } : null
}

/** FR-034 : les rappels dus, `maintenant ∈ [début − 15 min, début[`, jamais sur une annulée. */
export function rappelsDus(agenda: MyAgenda, sessions: readonly OfficialSession[], maintenant: Date): OfficialSession[] {
  const armes = new Set(agenda.entries.filter((e) => e.remind).map((e) => e.session_id))
  const t = maintenant.getTime()
  return sessions
    .filter((s) => armes.has(s.id) && s.status !== 'cancelled')
    .filter((s) => t >= instant(s.start_at) - RAPPEL_AVANT_MS && t < instant(s.start_at))
    .sort((a, b) => instant(a.start_at) - instant(b.start_at))
}

/**
 * L'agenda après une intention, tel qu'il s'affiche aussitôt. Un ajout garde
 * l'heure d'ajout d'une entrée existante : changer le rappel n'est pas ajouter.
 */
export function appliquerIntention(
  agenda: MyAgenda,
  sessionId: string,
  intention: IntentionAgenda,
  maintenant: Date,
): MyAgenda {
  const autres = agenda.entries.filter((e) => e.session_id !== sessionId)
  if (!intention.garder) return { ...agenda, entries: autres }
  const existante = agenda.entries.find((e) => e.session_id === sessionId)
  return {
    ...agenda,
    entries: [
      ...autres,
      { session_id: sessionId, remind: intention.remind, added_at: existante?.added_at ?? maintenant.toISOString() },
    ],
  }
}

/** L'agenda lu, plus les intentions encore dans la file : une lecture partie avant un envoi ne défait rien. */
export function avecLaFile(
  agenda: MyAgenda,
  enFile: Readonly<Record<string, IntentionAgenda>>,
  maintenant: Date,
): MyAgenda {
  return Object.entries(enFile).reduce((a, [id, intention]) => appliquerIntention(a, id, intention, maintenant), agenda)
}

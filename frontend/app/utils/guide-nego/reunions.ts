/**
 * Les réunions de la Francophonie : ce que l'écran en affiche, et l'inscription telle
 * qu'elle se voit avant que le réseau ait répondu.
 *
 * Pur et testé, comme `sessions.ts`. **Tout jour se compte dans le fuseau de la COP.**
 * Pas d'« En cours » : une réunion commencée reste Prévue (ou Inscrite) jusqu'à sa fin.
 */
import type {
  FrancophoneMeeting,
  FrancophoneMeetings,
  MeetingRegistration,
  MyMeetingRegistrations,
} from '~/types/negotiation-meetings'
import type { MyAgenda, OfficialSession } from '~/types/negotiation-sessions'
import { dayKeyInZone } from '../datetime.ts'
import { sessionsDeLAgenda } from './agenda.ts'
import { sessionsDuJour } from './sessions.ts'

export const cleDesReunions = (slug: string) => `reunions:${slug}`
export const CLE_LECTURE_MES_INSCRIPTIONS_REUNIONS = 'mes-inscriptions-reunions'
export const PREFIXE_FILE_INSCRIPTION_REUNION = 'inscription-reunion:'

const instant = (iso: string) => new Date(iso).getTime()

// ---------------------------------------------------------------------------
// L'état affiché (FR-013)
// ---------------------------------------------------------------------------

/** Noms de `etats.ts` : une marque par ligne. */
export type EtatReunion = 'annulee' | 'terminee' | 'inscrite' | 'liste-attente' | 'complet' | 'prevue'

type Inscription = Pick<MeetingRegistration, 'status' | 'waitlist_position'>

export const estTerminee = (r: Pick<FrancophoneMeeting, 'end_at'>, maintenant: Date) =>
  maintenant.getTime() >= instant(r.end_at)

/** Une capacité abaissée sous les inscrites reste « Complet ». Sans capacité, jamais. */
export const estComplete = (r: Pick<FrancophoneMeeting, 'capacity' | 'registered_count'>) =>
  r.capacity !== null && r.registered_count >= r.capacity

export function etatDeLaReunion(r: FrancophoneMeeting, inscription: Inscription | null, maintenant: Date): EtatReunion {
  if (r.status === 'cancelled') return 'annulee'
  if (estTerminee(r, maintenant)) return 'terminee'
  if (inscription?.status === 'registered') return 'inscrite'
  if (inscription?.status === 'waitlisted') return 'liste-attente'
  if (estComplete(r)) return 'complet'
  return 'prevue'
}

// ---------------------------------------------------------------------------
// Tri, jour, prochaine
// ---------------------------------------------------------------------------

export function trierLesReunions(reunions: readonly FrancophoneMeeting[]): FrancophoneMeeting[] {
  return [...reunions].sort(
    (a, b) => instant(a.start_at) - instant(b.start_at) || instant(a.end_at) - instant(b.end_at) || a.id.localeCompare(b.id),
  )
}

/** Annulées comprises : leur marque le dit. */
export function reunionsDuJour(reunions: readonly FrancophoneMeeting[], jour: string, fuseau: string): FrancophoneMeeting[] {
  return trierLesReunions(reunions.filter((r) => dayKeyInZone(r.start_at, fuseau) === jour))
}

/** La première à venir qui n'est pas annulée. */
export function prochaineReunion(reunions: readonly FrancophoneMeeting[], maintenant: Date): FrancophoneMeeting | null {
  const t = maintenant.getTime()
  return trierLesReunions(reunions).find((r) => r.status !== 'cancelled' && instant(r.start_at) > t) ?? null
}

// ---------------------------------------------------------------------------
// L'inscription (FR-008 à FR-010)
// ---------------------------------------------------------------------------

/** Les règles de la base : publiée, non commencée, inscription demandée, dans la fenêtre (bornes nulles : ouvertes). */
export function inscriptionOuverte(r: FrancophoneMeeting, maintenant: Date): boolean {
  const t = maintenant.getTime()
  if (r.status !== 'scheduled' || t >= instant(r.start_at) || !r.requires_registration) return false
  if (r.registration_opens_at && t < instant(r.registration_opens_at)) return false
  if (r.registration_closes_at && t >= instant(r.registration_closes_at)) return false
  return true
}

export type LibelleDInscription = 'inscrire' | 'inscrite' | 'rejoindre-attente' | 'liste-attente' | 'complet' | 'closes'

export interface BoutonDInscription {
  libelle: LibelleDInscription
  /** Nul : le bouton dit un état, il n'agit pas. */
  geste: 'inscrire' | 'desinscrire' | null
  /** « Liste d'attente — position N » ; nulle tant que le serveur ne l'a pas dite. */
  position: number | null
}

/**
 * Rien pour une annulée, une terminée ou une réunion sans inscription. Se désinscrire
 * reste possible jusqu'au début (FR-008).
 */
export function boutonDInscription(
  r: FrancophoneMeeting,
  inscription: Inscription | null,
  maintenant: Date,
): BoutonDInscription | null {
  if (r.status === 'cancelled' || estTerminee(r, maintenant)) return null
  const avantLeDebut = maintenant.getTime() < instant(r.start_at)
  if (inscription?.status === 'registered') {
    return { libelle: 'inscrite', geste: avantLeDebut ? 'desinscrire' : null, position: null }
  }
  if (inscription?.status === 'waitlisted') {
    return { libelle: 'liste-attente', geste: avantLeDebut ? 'desinscrire' : null, position: inscription.waitlist_position }
  }
  if (!r.requires_registration) return null
  if (!inscriptionOuverte(r, maintenant)) return { libelle: 'closes', geste: null, position: null }
  if (estComplete(r)) {
    return r.waitlist_enabled
      ? { libelle: 'rejoindre-attente', geste: 'inscrire', position: null }
      : { libelle: 'complet', geste: null, position: null }
  }
  return { libelle: 'inscrire', geste: 'inscrire', position: null }
}

/** Ce qui part dans la file : l'état voulu, et la référence de ce geste-ci. */
export interface IntentionInscription {
  inscrire: boolean
  /** Neuve à chaque « M'inscrire » ; nulle pour une désinscription. */
  client_ref: string | null
}

export const inscriptionDe = (mes: MyMeetingRegistrations, reunionId: string) =>
  mes.registrations.find((i) => i.meeting_id === reunionId) ?? null

export const lienVisio = (mes: MyMeetingRegistrations, reunionId: string) =>
  mes.video.find((v) => v.meeting_id === reunionId)?.url ?? null

/** Accès perdu, déconnexion : aucun lien ne reste sur le téléphone. */
export const sansLesLiens = (mes: MyMeetingRegistrations): MyMeetingRegistrations => ({ ...mes, video: [] })

/**
 * Les inscriptions après une intention, telles qu'elles s'affichent aussitôt. **Le lien
 * de visio part dès l'intention de désinscription** (R6). Une inscription déjà active
 * reste telle quelle ; une nouvelle se montre en attente si la réunion lue est pleine.
 */
export function appliquerIntentionInscription(
  mes: MyMeetingRegistrations,
  reunionId: string,
  reunion: FrancophoneMeeting | null,
  intention: IntentionInscription,
  maintenant: Date,
): MyMeetingRegistrations {
  const autres = mes.registrations.filter((i) => i.meeting_id !== reunionId)
  if (!intention.inscrire) {
    return { registrations: autres, video: mes.video.filter((v) => v.meeting_id !== reunionId) }
  }
  if (inscriptionDe(mes, reunionId)) return mes
  const enAttente = reunion !== null && estComplete(reunion) && reunion.waitlist_enabled
  const nouvelle: MeetingRegistration = {
    meeting_id: reunionId,
    status: enAttente ? 'waitlisted' : 'registered',
    waitlist_position: null,
    client_ref: intention.client_ref,
    registered_at: maintenant.toISOString(),
  }
  return { ...mes, registrations: [...autres, nouvelle] }
}

export function avecLaFile(
  mes: MyMeetingRegistrations,
  enFile: Readonly<Record<string, IntentionInscription>>,
  reunions: readonly FrancophoneMeeting[],
  maintenant: Date,
): MyMeetingRegistrations {
  return Object.entries(enFile).reduce(
    (acc, [id, intention]) =>
      appliquerIntentionInscription(acc, id, reunions.find((r) => r.id === id) ?? null, intention, maintenant),
    mes,
  )
}

// ---------------------------------------------------------------------------
// La garde de la liste
// ---------------------------------------------------------------------------

export interface ReunionsGardees {
  slug: string
  /** Nulle : l'édition n'est pas connue de l'API. */
  lues: FrancophoneMeetings | null
  empreinte: string | null
}

// ---------------------------------------------------------------------------
// « Ma journée » : les lignes Sessions et Réunions du bloc « trois agendas » (FR-021)
// ---------------------------------------------------------------------------

export type LigneSessionsDuJour =
  | { source: 'agenda' | 'thematiques'; sessions: OfficialSession[] }
  | { source: 'aucune'; sessions: [] }

/** Les sessions du jour de « Mon agenda » ; à défaut, celles de mes thématiques. */
export function ligneSessionsDuJour(
  agenda: MyAgenda,
  sessions: readonly OfficialSession[],
  thematiques: readonly string[],
  maintenant: Date,
  fuseau: string,
): LigneSessionsDuJour {
  const jour = dayKeyInZone(maintenant, fuseau)
  const dansLAgenda = sessionsDuJour(sessionsDeLAgenda(agenda, sessions), jour, fuseau)
  if (dansLAgenda.length > 0) return { source: 'agenda', sessions: dansLAgenda }
  const suivies = sessionsDuJour(
    sessions.filter((s) => s.theme !== null && thematiques.includes(s.theme)),
    jour,
    fuseau,
  )
  return suivies.length > 0 ? { source: 'thematiques', sessions: suivies } : { source: 'aucune', sessions: [] }
}

export type LigneReunionsDuJour =
  | { reunions: FrancophoneMeeting[]; prochaine: null }
  | { reunions: []; prochaine: FrancophoneMeeting | null }

/** Les réunions du jour ; sinon « Rien aujourd'hui. Prochaine : … », ou rien du tout. */
export function ligneReunionsDuJour(
  reunions: readonly FrancophoneMeeting[],
  maintenant: Date,
  fuseau: string,
): LigneReunionsDuJour {
  const duJour = reunionsDuJour(reunions, dayKeyInZone(maintenant, fuseau), fuseau)
  return duJour.length > 0 ? { reunions: duJour, prochaine: null } : { reunions: [], prochaine: prochaineReunion(reunions, maintenant) }
}

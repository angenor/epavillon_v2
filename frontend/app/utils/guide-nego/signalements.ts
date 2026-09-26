/**
 * Les signalements du réseau et les notifications : ce que l'application en affiche.
 *
 * Pur et testé, comme `sessions.ts`. **Tout jour se compte dans le fuseau de la COP.**
 * Le serveur ne sert que ce qui est publié et non retiré ; ce qui se juge ici, c'est
 * ce qui a vieilli depuis la lecture — une copie gardée d'hier ne montre pas l'encart
 * d'une session terminée ni la réunion d'un jour passé.
 */
import type { Notification, NotificationFeed } from '~/types/engagement'
import type { MyReport, ReportPayload, ReportQueue, ReportQueueItem, ReportReason, RejectReason } from '~/types/negotiation-reports'
import type { MyAgenda, NetworkMeeting, NetworkReport, OfficialSession } from '~/types/negotiation-sessions'
import { dayKeyInZone, formatTime, instantFromWallClock } from '../datetime.ts'
import { finEffective, lendemain } from './sessions.ts'
import type { NomDEtat } from './etats.ts'
import type { NomDePicto } from './pictogrammes.ts'

export const CLE_LECTURE_MES_SIGNALEMENTS = 'mes-signalements'
export const PREFIXE_FILE_SIGNALEMENT = 'signalement:'
export const CLE_LECTURE_NOTIFICATIONS = 'notifications'
/** Une intention par notification : la file garde une entrée par clé, un lot écraserait le précédent. */
export const PREFIXE_FILE_LU = 'lu:'
export const CLE_LECTURE_REGLAGE_NOTIFICATIONS = 'reglage-notifications'
export const CLE_FILE_COURRIEL = 'notifications-courriel'
export const CLE_FILE_THEMATIQUES_NOTIFIEES = 'thematiques-notifiees'

const instant = (iso: string) => new Date(iso).getTime()

/**
 * La référence posée par le téléphone. `randomUUID` manque hors origine sécurisée
 * (HTTP sur une adresse locale) : le repli forme un UUID v4 à partir du même hasard.
 */
export function referenceClient(): string {
  const hasard = globalThis.crypto
  if (hasard?.randomUUID) return hasard.randomUUID()
  const o = hasard?.getRandomValues
    ? hasard.getRandomValues(new Uint8Array(16))
    : Uint8Array.from({ length: 16 }, () => Math.floor(Math.random() * 256))
  o[6] = (o[6]! & 0x0f) | 0x40
  o[8] = (o[8]! & 0x3f) | 0x80
  const h = Array.from(o, (x) => x.toString(16).padStart(2, '0')).join('')
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`
}

// ---------------------------------------------------------------------------
// La saisie (FR-003, FR-004)
// ---------------------------------------------------------------------------

export type MotifDeChangement = Exclude<ReportReason, 'unannounced'>

export const MOTIFS_DE_CHANGEMENT: readonly MotifDeChangement[] = ['cancelled', 'time', 'venue', 'other']

/** Pictogramme et couleur d'un motif, comme la maquette 09 ; « Autre chose » garde le gris des pictogrammes. */
export const DESSIN_DU_MOTIF: Record<ReportReason, { picto: NomDePicto; teinte: NomDEtat | null }> = {
  cancelled: { picto: 'close', teinte: 'annulee' },
  time: { picto: 'moved', teinte: 'deplacee' },
  venue: { picto: 'pin', teinte: 'deplacee' },
  other: { picto: 'info', teinte: null },
  unannounced: { picto: 'diamond', teinte: 'non-annoncee' },
}

export const PRECISION_MAX = 600

const rempli = (texte: string | null | undefined) => {
  const t = texte?.trim()
  return t ? t : undefined
}

/** « HH:MM » du jour `AAAA-MM-JJ`, dans le fuseau de la COP ; vide ou illisible : rien. */
export function heureDuJour(jour: string, heure: string, fuseau: string): string | undefined {
  if (!/^\d{2}:\d{2}$/.test(heure.trim())) return undefined
  return instantFromWallClock(`${jour} ${heure.trim()}`, fuseau) ?? undefined
}

export interface SaisieDeChangement {
  heure: string
  salle: string
  precision: string
}

/** Le corps d'un changement, hors `client_ref` et `edition` ; la valeur proposée ne suit que son motif. */
export function corpsDuChangement(
  motif: MotifDeChangement,
  saisie: SaisieDeChangement,
  session: Pick<OfficialSession, 'id' | 'start_at'>,
  fuseau: string,
): Omit<ReportPayload, 'client_ref' | 'edition'> {
  const proposed_start = motif === 'time' ? heureDuJour(dayKeyInZone(session.start_at, fuseau), saisie.heure, fuseau) : undefined
  const proposed_venue = motif === 'venue' ? rempli(saisie.salle) : undefined
  return {
    reason: motif,
    session_id: session.id,
    ...(proposed_start ? { proposed_start } : {}),
    ...(proposed_venue ? { proposed_venue } : {}),
    ...(rempli(saisie.precision) ? { detail: rempli(saisie.precision) } : {}),
  }
}

export interface SaisieDeReunion {
  quoi: string
  ou: string
  quand: string
  /** `null` : « Autre », ou rien de choisi. */
  thematique: string | null
}

/** `null` tant que « Quoi » est vide : c'est le seul champ requis. */
export function corpsDeLaReunion(
  saisie: SaisieDeReunion,
  jour: string,
  fuseau: string,
): Omit<ReportPayload, 'client_ref' | 'edition'> | null {
  const what = rempli(saisie.quoi)
  if (!what) return null
  const proposed_start = heureDuJour(jour, saisie.quand, fuseau)
  const proposed_venue = rempli(saisie.ou)
  return {
    reason: 'unannounced',
    what,
    day: jour,
    theme: saisie.thematique,
    ...(proposed_start ? { proposed_start } : {}),
    ...(proposed_venue ? { proposed_venue } : {}),
  }
}

// ---------------------------------------------------------------------------
// L'encart et le repère (FR-017, FR-020)
// ---------------------------------------------------------------------------

/** Les encarts à montrer, le plus récent d'abord ; aucun une fois la session terminée. */
export function encartsAffiches(session: OfficialSession, maintenant: Date, fuseau: string): NetworkReport[] {
  const encarts = session.network_reports ?? []
  if (encarts.length === 0 || maintenant.getTime() >= finEffective(session, fuseau)) return []
  return [...encarts].sort((a, b) => instant(b.validated_at) - instant(a.validated_at))
}

export interface RepereSignale {
  validation: string
  /** « HH:MM » dans le fuseau de la COP, pour le libellé complet des lecteurs d'écran. */
  heure: string
}

/** Le repère de ligne — losange et « Signalé » —, nul si aucun encart ne s'affiche. */
export function repereSignale(
  session: OfficialSession,
  maintenant: Date,
  fuseau: string,
  locale = 'fr-FR',
): RepereSignale | null {
  const dernier = encartsAffiches(session, maintenant, fuseau)[0]
  if (!dernier) return null
  return { validation: dernier.validated_at, heure: formatTime(dernier.validated_at, { timeZone: fuseau, locale }) }
}

// ---------------------------------------------------------------------------
// Les réunions non annoncées (FR-018, FR-021, FR-022)
// ---------------------------------------------------------------------------

/** Minuit à la fin du jour `AAAA-MM-JJ`, dans le fuseau de la COP. */
export function finDuJour(jour: string, fuseau: string): number {
  const minuit = instantFromWallClock(`${lendemain(jour)} 00:00`, fuseau)
  return minuit ? instant(minuit) : Date.parse(`${lendemain(jour)}T00:00:00Z`)
}

export const reunionVisible = (reunion: NetworkMeeting, maintenant: Date, fuseau: string) =>
  maintenant.getTime() < finDuJour(reunion.day, fuseau)

/** Sans heure, elle se range en fin de journée. */
export const debutDeTri = (reunion: NetworkMeeting, fuseau: string) =>
  reunion.start_at ? instant(reunion.start_at) : finDuJour(reunion.day, fuseau) - 1

export function reunionsVisibles(
  reunions: readonly NetworkMeeting[],
  maintenant: Date,
  fuseau: string,
): NetworkMeeting[] {
  return reunions
    .filter((r) => reunionVisible(r, maintenant, fuseau))
    .sort((a, b) => debutDeTri(a, fuseau) - debutDeTri(b, fuseau) || a.id.localeCompare(b.id))
}

export function reunionsDuJour(
  reunions: readonly NetworkMeeting[],
  jour: string,
  maintenant: Date,
  fuseau: string,
): NetworkMeeting[] {
  return reunionsVisibles(reunions, maintenant, fuseau).filter((r) => r.day === jour)
}

/** Les jours de la bande, réunions non annoncées comprises : un jour sans session officielle peut en porter. */
export function joursAvecReunions(
  jours: readonly string[],
  reunions: readonly NetworkMeeting[],
  maintenant: Date,
  fuseau: string,
): string[] {
  return [...new Set([...jours, ...reunionsVisibles(reunions, maintenant, fuseau).map((r) => r.day)])].sort()
}

export type LigneDuJour =
  | { genre: 'session'; session: OfficialSession }
  | { genre: 'reseau'; reunion: NetworkMeeting }

/** Sessions et réunions non annoncées d'un même jour, dans l'ordre des débuts. */
export function lignesDuJour(
  sessions: readonly OfficialSession[],
  reunions: readonly NetworkMeeting[],
  jour: string,
  maintenant: Date,
  fuseau: string,
): LigneDuJour[] {
  const debut = (l: LigneDuJour) =>
    l.genre === 'session' ? instant(l.session.start_at) : debutDeTri(l.reunion, fuseau)
  return [
    ...sessions.filter((s) => dayKeyInZone(s.start_at, fuseau) === jour).map((session) => ({ genre: 'session' as const, session })),
    ...reunionsDuJour(reunions, jour, maintenant, fuseau).map((reunion) => ({ genre: 'reseau' as const, reunion })),
  ].sort((a, b) => debut(a) - debut(b) || (a.genre === b.genre ? 0 : a.genre === 'session' ? -1 : 1))
}

/** « Mes thématiques » : comme une session, une réunion sans thématique passe toujours. */
export const reunionPasseLeFiltre = (reunion: NetworkMeeting, thematiques: readonly string[]) =>
  reunion.theme === null || thematiques.includes(reunion.theme)

/** Les réunions gardées dans « Mon agenda » ; une entrée sans réunion lue est tue. */
export function reunionsDeLAgenda(agenda: MyAgenda, reunions: readonly NetworkMeeting[]): NetworkMeeting[] {
  const suivies = new Set((agenda.network_entries ?? []).map((e) => e.network_meeting_id))
  return reunions.filter((r) => suivies.has(r.id))
}

export interface JourDeReunions {
  jour: string
  reunions: NetworkMeeting[]
}

/** Source coupée : les réunions encore à venir, par jour. */
export function reunionsParJour(reunions: readonly NetworkMeeting[], maintenant: Date, fuseau: string): JourDeReunions[] {
  const jours = new Map<string, NetworkMeeting[]>()
  for (const r of reunionsVisibles(reunions, maintenant, fuseau)) jours.set(r.day, [...(jours.get(r.day) ?? []), r])
  return [...jours].sort(([a], [b]) => a.localeCompare(b)).map(([jour, liste]) => ({ jour, reunions: liste }))
}

// ---------------------------------------------------------------------------
// « Mes signalements » (FR-006, FR-008)
// ---------------------------------------------------------------------------

export type EtatSignalement = 'en-attente' | 'envoye' | 'valide' | 'non-retenu'

export interface LigneSignalement {
  signalement: MyReport
  etat: EtatSignalement
}

/** Ce qui attend dans la file, tel que « Mes signalements » le montre avant l'envoi. */
export interface SignalementEnFile {
  corps: ReportPayload
  prise_a: string
}

export function signalementEnAttente(enFile: SignalementEnFile, session: MyReport['session']): MyReport {
  const c = enFile.corps
  return {
    id: c.client_ref,
    client_ref: c.client_ref,
    reason: c.reason,
    session,
    what: c.what ?? null,
    proposed_start: c.proposed_start ?? null,
    proposed_venue: c.proposed_venue ?? null,
    day: c.day ?? null,
    theme: c.theme ?? null,
    network_meeting_id: null,
    detail: c.detail ?? null,
    status: 'submitted',
    submitted_at: enFile.prise_a,
    decided_at: null,
    reject_reason: null,
    reject_detail: null,
  }
}

const ETATS: Record<MyReport['status'], Exclude<EtatSignalement, 'en-attente'>> = {
  submitted: 'envoye',
  validated: 'valide',
  rejected: 'non-retenu',
}

/** Les lus, plus ceux de la file que la lecture ne porte pas encore ; le plus récent d'abord. */
export function lignesDesSignalements(lus: readonly MyReport[], enAttente: readonly MyReport[]): LigneSignalement[] {
  const recus = new Set(lus.map((r) => r.client_ref))
  return [
    ...enAttente.filter((r) => !recus.has(r.client_ref)).map((signalement) => ({ signalement, etat: 'en-attente' as const })),
    ...lus.map((signalement) => ({ signalement, etat: ETATS[signalement.status] })),
  ].sort((a, b) => instant(b.signalement.submitted_at) - instant(a.signalement.submitted_at))
}

export interface TexteDeLEtat {
  /** Clé de texte : « Envoyé — partira au retour du réseau », « Envoyé », « Validé », « Non retenu ». */
  cle: EtatSignalement
  /** L'instant à dire : l'envoi, ou la décision. */
  a: string
  motif: RejectReason | null
  precision: string | null
}

export function texteDeLEtat(ligne: LigneSignalement): TexteDeLEtat {
  const s = ligne.signalement
  const decide = ligne.etat === 'valide' || ligne.etat === 'non-retenu'
  return {
    cle: ligne.etat,
    a: decide ? (s.decided_at ?? s.submitted_at) : s.submitted_at,
    motif: ligne.etat === 'non-retenu' ? s.reject_reason : null,
    precision: ligne.etat === 'non-retenu' ? s.reject_detail : null,
  }
}

/** FR-008 : « Votre signalement — envoyé à HH:MM, en vérification », tant qu'il n'est pas tranché. */
export function signalementEnCours(sessionId: string, lignes: readonly LigneSignalement[]): LigneSignalement | null {
  return (
    lignes.find((l) => l.signalement.session?.id === sessionId && (l.etat === 'en-attente' || l.etat === 'envoye')) ??
    null
  )
}

// ---------------------------------------------------------------------------
// La file de validation — en ligne seulement (FR-013)
// ---------------------------------------------------------------------------

export type EtatDuTraite = 'non-retenu' | 'en-publication' | 'valide' | 'retire'

/** La marque d'un signalement tranché ; seul un signalement affiché se retire. */
export function etatDuTraite(item: ReportQueueItem): EtatDuTraite {
  if (item.status === 'rejected') return 'non-retenu'
  if (item.withdrawn_at) return 'retire'
  return item.published_at ? 'valide' : 'en-publication'
}

export const peutSeRetirer = (item: ReportQueueItem) => etatDuTraite(item) === 'valide'

/** La file après une décision : un élément revenu à `submitted` (annulé) reprend sa place. */
export function apresDecision(file: ReportQueue, element: ReportQueueItem): ReportQueue {
  const pending = file.pending.filter((r) => r.id !== element.id)
  const decided = file.decided_today.filter((r) => r.id !== element.id)
  if (element.status === 'submitted') {
    return {
      pending: [...pending, element].sort((a, b) => instant(a.submitted_at) - instant(b.submitted_at)),
      decided_today: decided,
    }
  }
  return { pending, decided_today: [element, ...decided] }
}

// ---------------------------------------------------------------------------
// Les notifications (FR-027)
// ---------------------------------------------------------------------------

/** Le fil lu, plus les lectures encore dans la file. */
export function avecLesLectures(feed: NotificationFeed, lues: ReadonlySet<string>, maintenant: Date): NotificationFeed {
  let marquees = 0
  const items = feed.items.map((n) => {
    if (n.read_at || !lues.has(n.id)) return n
    marquees += 1
    return { ...n, read_at: maintenant.toISOString() }
  })
  return { items, unread_count: Math.max(0, feed.unread_count - marquees) }
}

export const idsNonLues = (items: readonly Notification[]) => items.filter((n) => !n.read_at).map((n) => n.id)

export interface JourDeNotifications {
  jour: string
  notifications: Notification[]
}

/** Par jour dans le fuseau de la COP, le plus récent d'abord. */
export function notificationsParJour(items: readonly Notification[], fuseau: string): JourDeNotifications[] {
  const jours = new Map<string, Notification[]>()
  for (const n of [...items].sort((a, b) => instant(b.created_at) - instant(a.created_at))) {
    const jour = dayKeyInZone(n.created_at, fuseau)
    jours.set(jour, [...(jours.get(jour) ?? []), n])
  }
  return [...jours].map(([jour, notifications]) => ({ jour, notifications }))
}

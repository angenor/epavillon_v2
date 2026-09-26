/**
 * Le Pavillon de la Francophonie : ce que l'écran affiche des activités de l'édition, et
 * l'inscription telle qu'elle se voit avant que le réseau ait répondu.
 *
 * Pur et testé, comme `reunions.ts`. **Tout jour se compte dans le fuseau de la COP.**
 * **Aucun filtre de thématique** (décision du 22/09, R5) : chaque règle rend toutes les
 * activités publiées qu'on lui donne, quelles que soient les thématiques suivies.
 */
import type { Venue } from '~/types/event/venue'
import type { Registration, RegistrationFormField } from '~/types/programme/registration'
import type { PublicScheduleRow } from '~/types/views'
import { dayKeyInZone } from '../datetime.ts'
import { jourAOuvrir, lendemain } from './sessions.ts'

export const cleDuPavillon = (slugEdition: string) => `pavillon:${slugEdition}`
export const cleDeLActivite = (slugEdition: string, slug: string) => `pavillon-activite:${slugEdition}:${slug}`
export const CLE_LECTURE_MES_INSCRIPTIONS_PAVILLON = 'mes-inscriptions-pavillon'
export const PREFIXE_FILE_INSCRIPTION_PAVILLON = 'inscription-pavillon:'
/** Une inscription posée sans réseau, que le serveur n'a pas encore numérotée. */
export const PREFIXE_INSCRIPTION_LOCALE = 'locale:'

export const cheminDeLActivite = (slug: string) => `/guide-nego/francophonie/pavillon/${encodeURIComponent(slug)}`

const instant = (iso: string) => new Date(iso).getTime()

type Activite = PublicScheduleRow

// ---------------------------------------------------------------------------
// Le lieu
// ---------------------------------------------------------------------------

/** Le stand de l'édition ; à défaut, le premier lieu connu. */
export function lieuDuPavillon(lieux: readonly Venue[]): Venue | null {
  return lieux.find((l) => l.kind === 'pavilion') ?? lieux[0] ?? null
}

// ---------------------------------------------------------------------------
// Les jours
// ---------------------------------------------------------------------------

export const jourDeLActivite = (a: Pick<Activite, 'starts_at'>, fuseau: string) => dayKeyInZone(a.starts_at, fuseau)

/** La veille d'une clé `AAAA-MM-JJ`, sur le calendrier. */
export function veille(jour: string): string {
  const [a, m, j] = jour.split('-').map(Number) as [number, number, number]
  return new Date(Date.UTC(a, m - 1, j - 1)).toISOString().slice(0, 10)
}

export function trierLesActivites(activites: readonly Activite[]): Activite[] {
  return [...activites].sort(
    (a, b) => instant(a.starts_at) - instant(b.starts_at) || instant(a.ends_at) - instant(b.ends_at) || a.id.localeCompare(b.id),
  )
}

/**
 * Toute l'édition, jours passés compris : du premier au dernier jour qui porte une
 * activité, sans trou — un jour creux reste dans la bande, vide.
 */
export function joursDeLaBande(activites: readonly Activite[], fuseau: string): string[] {
  const jours = activites.map((a) => jourDeLActivite(a, fuseau)).sort()
  const premier = jours[0]
  const dernier = jours[jours.length - 1]
  if (!premier || !dernier) return []
  const bande: string[] = []
  for (let j = premier; j <= dernier; j = lendemain(j)) bande.push(j)
  return bande
}

/** Aujourd'hui s'il est dans l'édition, sinon le prochain jour, sinon le dernier. */
export const jourDuPavillonAOuvrir = (jours: readonly string[], maintenant: Date, fuseau: string) =>
  jourAOuvrir(jours, maintenant, fuseau)

/** Annulées et reportées comprises : leur état le dit. */
export function activitesDuJour(activites: readonly Activite[], jour: string, fuseau: string): Activite[] {
  return trierLesActivites(activites.filter((a) => jourDeLActivite(a, fuseau) === jour))
}

/** « Hier — rediffusions » : les activités de la veille qui ont une rediffusion. */
export function rediffusionsDeLaVeille(activites: readonly Activite[], jour: string, fuseau: string): Activite[] {
  return activitesDuJour(activites, veille(jour), fuseau).filter((a) => !!a.replay_url)
}

export interface JourSuivant {
  jour: string
  activites: Activite[]
}

/** « Les jours suivants » : après le jour affiché, jour par jour. */
export function joursSuivants(activites: readonly Activite[], jour: string, fuseau: string): JourSuivant[] {
  const parJour = new Map<string, Activite[]>()
  for (const a of trierLesActivites(activites)) {
    const j = jourDeLActivite(a, fuseau)
    if (j > jour) parJour.set(j, [...(parJour.get(j) ?? []), a])
  }
  return [...parJour].map(([j, liste]) => ({ jour: j, activites: liste }))
}

// ---------------------------------------------------------------------------
// L'état de l'activité
// ---------------------------------------------------------------------------

export type EtatActivite = 'prevue' | 'en-cours' | 'terminee' | 'annulee' | 'reportee'

const estAnnulee = (a: Pick<Activite, 'status' | 'temporal_state'>) =>
  a.status === 'cancelled' || a.temporal_state === 'cancelled'
const estReportee = (a: Pick<Activite, 'status' | 'temporal_state'>) =>
  a.status === 'postponed' || a.temporal_state === 'postponed'

/** L'annulation et le report d'abord ; puis l'heure, qui se recalcule sans réseau. */
export function etatDeLActivite(a: Activite, maintenant: Date): EtatActivite {
  if (estAnnulee(a)) return 'annulee'
  if (estReportee(a)) return 'reportee'
  const t = maintenant.getTime()
  if (t >= instant(a.ends_at) || a.status === 'completed') return 'terminee'
  if (t >= instant(a.starts_at) || a.status === 'live') return 'en-cours'
  return 'prevue'
}

/** La première à venir qui n'est ni annulée ni reportée. */
export function prochaineActivite(activites: readonly Activite[], maintenant: Date): Activite | null {
  const t = maintenant.getTime()
  return trierLesActivites(activites).find((a) => !estAnnulee(a) && !estReportee(a) && instant(a.starts_at) > t) ?? null
}

// ---------------------------------------------------------------------------
// La marque : inscription ou rediffusion
// ---------------------------------------------------------------------------

type Inscription = Pick<Registration, 'status' | 'waitlist_position'>

export type MarqueActivite =
  | { nom: 'inscrite' }
  | { nom: 'liste-attente'; position: number | null }
  | { nom: 'complet' }
  | { nom: 'rediffusion'; url: string; minutes: number | null }
  | { nom: 'sans-inscription' }

/** Minutes arrondies ; nulle quand la durée n'est pas connue. */
export const minutesDeRediffusion = (a: Pick<Activite, 'replay_duration_seconds'>) =>
  a.replay_duration_seconds ? Math.max(1, Math.round(a.replay_duration_seconds / 60)) : null

/** Colonne absente (jeux d'exemple du site) : la valeur par défaut du modèle, inscription requise. */
export const inscriptionRequise = (a: Pick<Activite, 'registration_required'>) => a.registration_required ?? true

/** Une jauge abaissée sous les inscrits reste « Complet ». Sans jauge, jamais. */
export const estComplete = (a: Pick<Activite, 'capacity' | 'registered_count'>) =>
  a.capacity !== null && a.registered_count >= a.capacity

/**
 * Nulle quand l'état suffit (annulée, reportée) ou quand le bouton parle (inscription
 * ouverte). Une activité terminée montre sa rediffusion avant l'inscription.
 */
export function marqueDeLActivite(a: Activite, inscription: Inscription | null, maintenant: Date): MarqueActivite | null {
  const etat = etatDeLActivite(a, maintenant)
  if (etat === 'annulee' || etat === 'reportee') return null
  if (etat === 'terminee' && a.replay_url) return { nom: 'rediffusion', url: a.replay_url, minutes: minutesDeRediffusion(a) }
  if (inscription?.status === 'registered' || inscription?.status === 'attended') return { nom: 'inscrite' }
  if (inscription?.status === 'waitlisted') return { nom: 'liste-attente', position: inscription.waitlist_position }
  if (etat === 'terminee') return null
  if (!inscriptionRequise(a)) return { nom: 'sans-inscription' }
  if (estComplete(a) && !a.waitlist_enabled) return { nom: 'complet' }
  return null
}

// ---------------------------------------------------------------------------
// Le bouton d'inscription (FR-006, FR-007)
// ---------------------------------------------------------------------------

export type LibelleInscriptionPavillon =
  | 'inscrire'
  | 'inscrite'
  | 'rejoindre-attente'
  | 'liste-attente'
  | 'complet'
  | 'closes'
  | 'pas-encore'

export interface BoutonInscriptionPavillon {
  libelle: LibelleInscriptionPavillon
  /** Nul : le bouton dit un état, il n'agit pas. */
  geste: 'inscrire' | 'annuler' | null
  position: number | null
  /** L'ouverture des inscriptions, pour « pas encore ouvertes ». */
  ouvreLe: string | null
}

/** Aucun bouton sur une activité annulée, reportée, commencée, ou qui ne prend pas d'inscription. */
export function boutonDInscriptionPavillon(
  a: Activite,
  inscription: Inscription | null,
  maintenant: Date,
): BoutonInscriptionPavillon | null {
  if (etatDeLActivite(a, maintenant) !== 'prevue') return null
  const bouton = (libelle: LibelleInscriptionPavillon, geste: BoutonInscriptionPavillon['geste'], position: number | null = null) => ({
    libelle,
    geste,
    position,
    ouvreLe: null,
  })
  if (inscription?.status === 'registered') return bouton('inscrite', 'annuler')
  if (inscription?.status === 'waitlisted') return bouton('liste-attente', 'annuler', inscription.waitlist_position)
  if (!inscriptionRequise(a)) return null
  const t = maintenant.getTime()
  if (a.registration_opens_at && t < instant(a.registration_opens_at)) {
    return { ...bouton('pas-encore', null), ouvreLe: a.registration_opens_at }
  }
  if (a.registration_closes_at && t >= instant(a.registration_closes_at)) return bouton('closes', null)
  if (estComplete(a)) return a.waitlist_enabled ? bouton('rejoindre-attente', 'inscrire') : bouton('complet', null)
  return bouton('inscrire', 'inscrire')
}

// ---------------------------------------------------------------------------
// Le formulaire (R4)
// ---------------------------------------------------------------------------

/** Les réponses préremplies : le pays du profil, en code ISO2, dans chaque champ pays. */
export function reponsesPreremplies(champs: readonly RegistrationFormField[], paysIso2: string | null): Record<string, unknown> {
  if (!paysIso2) return {}
  return Object.fromEntries(champs.filter((c) => c.field_type === 'country').map((c) => [c.code, paysIso2]))
}

/**
 * « D'un geste » : aucun champ obligatoire, ou le seul obligatoire est le pays et le
 * profil le connaît. Un champ sensible obligatoire demande son consentement : jamais.
 */
export function formulaireDUnGeste(champs: readonly RegistrationFormField[], paysIso2: string | null): boolean {
  const obligatoires = champs.filter((c) => c.is_active && c.is_required)
  return obligatoires.every((c) => c.field_type === 'country' && !!paysIso2 && !c.is_sensitive)
}

/** Une réponse donnée à un champ sensible exige le consentement, comme sur le site. */
export function consentementRequis(champs: readonly RegistrationFormField[], reponses: Record<string, unknown>): boolean {
  return champs.some((c) => c.is_sensitive && reponses[c.code] !== undefined && reponses[c.code] !== null && reponses[c.code] !== '')
}

// ---------------------------------------------------------------------------
// Mes inscriptions, et l'intention qui part dans la file (R3)
// ---------------------------------------------------------------------------

/** L'état voulu, jamais un delta : la dernière intention gagne. */
export type IntentionPavillon =
  | { etat: 'inscrite'; reponses: Record<string, unknown>; consentement: boolean }
  | { etat: 'annulee' }

/** La ligne vivante de la séance : toute autre que `cancelled`, la plus récente d'abord. */
export function inscriptionDeLaSeance(inscriptions: readonly Registration[], sessionId: string): Registration | null {
  return (
    inscriptions
      .filter((i) => i.session_id === sessionId && i.status !== 'cancelled')
      .sort((a, b) => b.created_at.localeCompare(a.created_at))[0] ?? null
  )
}

/** L'identifiant à annuler au départ : une ligne vivante que le serveur connaît. */
export function inscriptionAAnnuler(inscriptions: readonly Registration[], sessionId: string): Registration | null {
  const vivante = inscriptionDeLaSeance(inscriptions, sessionId)
  return vivante && !vivante.id.startsWith(PREFIXE_INSCRIPTION_LOCALE) ? vivante : null
}

/**
 * Les inscriptions après une intention, telles qu'elles s'affichent aussitôt. Une
 * inscription déjà vivante reste telle quelle ; une nouvelle se montre en attente si
 * l'activité lue est pleine avec liste d'attente.
 */
export function appliquerIntentionPavillon(
  inscriptions: readonly Registration[],
  sessionId: string,
  activite: Activite | null,
  intention: IntentionPavillon,
  personne: string,
  maintenant: Date,
): Registration[] {
  const horodatage = maintenant.toISOString()
  if (intention.etat === 'annulee') {
    return inscriptions
      .filter((i) => !(i.session_id === sessionId && i.id.startsWith(PREFIXE_INSCRIPTION_LOCALE)))
      .map((i) =>
        i.session_id === sessionId && i.status !== 'cancelled'
          ? { ...i, status: 'cancelled' as const, waitlist_position: null, cancelled_at: horodatage }
          : i,
      )
  }
  if (inscriptionDeLaSeance(inscriptions, sessionId)) return [...inscriptions]
  const enAttente = activite !== null && estComplete(activite) && !!activite.waitlist_enabled
  const locale: Registration = {
    id: `${PREFIXE_INSCRIPTION_LOCALE}${sessionId}`,
    session_id: sessionId,
    person_id: personne,
    organization_id: null,
    status: enAttente ? 'waitlisted' : 'registered',
    answers: intention.reponses,
    locale: 'fr',
    waitlist_position: null,
    joined_at: null,
    attendance_minutes: null,
    certificate_asset_id: null,
    source: 'api',
    cancelled_at: null,
    cancelled_reason: null,
    created_at: horodatage,
    updated_at: horodatage,
  }
  return [...inscriptions, locale]
}

export function avecLaFilePavillon(
  inscriptions: readonly Registration[],
  enFile: Readonly<Record<string, IntentionPavillon>>,
  activites: readonly Activite[],
  personne: string,
  maintenant: Date,
): Registration[] {
  return Object.entries(enFile).reduce<Registration[]>(
    (acc, [id, intention]) =>
      appliquerIntentionPavillon(acc, id, activites.find((a) => a.id === id) ?? null, intention, personne, maintenant),
    [...inscriptions],
  )
}

// ---------------------------------------------------------------------------
// « Ma journée » : la ligne Pavillon du bloc « trois agendas » (FR-009)
// ---------------------------------------------------------------------------

export type LignePavillonDuJour =
  | { activites: Activite[]; prochaine: null }
  | { activites: []; prochaine: Activite | null }

/** Les activités du jour ; sinon « Rien aujourd'hui. Prochaine : … », ou rien du tout. */
export function lignePavillonDuJour(activites: readonly Activite[], maintenant: Date, fuseau: string): LignePavillonDuJour {
  const duJour = activitesDuJour(activites, dayKeyInZone(maintenant, fuseau), fuseau)
  return duJour.length > 0
    ? { activites: duJour, prochaine: null }
    : { activites: [], prochaine: prochaineActivite(activites, maintenant) }
}

// ---------------------------------------------------------------------------
// La garde
// ---------------------------------------------------------------------------

export interface PavillonGarde {
  slug: string
  eventId: string
  activites: Activite[]
  lieu: Venue | null
  empreinte: string | null
}

/**
 * « Ma journée » : ses blocs, et le jour qu'elle affiche.
 *
 * **L'ordre des blocs est fixe** (FR-013) et vit ici, pas dans le gabarit : un test
 * le tient sans navigateur, et chaque étape qui remplit un bloc (1, 3a, 3b, 4, 5) le
 * trouve à sa place.
 */
import type { LibraryDocument } from '~/types/negotiation-documents'
import type { OfficialSession } from '~/types/negotiation-sessions'
import { dayKeyInZone, wallClockInZone } from '../datetime.ts'
import type { Progression, Recent } from './appareil-lecture.ts'
import { jourCivil } from './connexion.ts'
import { encartsAffiches } from './signalements.ts'
import { etatAffiche, sessionsDuJour } from './sessions.ts'

export const BLOCS_DE_MA_JOURNEE = [
  'prochaine-session',
  'trois-agendas',
  'changements',
  'documents',
  'lexique',
] as const

export type BlocDeMaJournee = (typeof BLOCS_DE_MA_JOURNEE)[number]

/**
 * « Jeudi 12 novembre » — **le jour seul, sans fuseau nommé**.
 *
 * Nommer le fuseau de l'appareil tromperait : son identifiant dit « Istanbul » à
 * Antalya, « Lome » et « Ndjamena » sans accent, « Douala » à Yaoundé, et le nom long
 * du moteur dit « heure moyenne de Greenwich » à Dakar. 0c n'affiche aucune heure
 * d'événement ; le fuseau nommé viendra en 3a, du lieu de l'édition.
 */
export function jourLisible(date: Date, locale: string, fuseau?: string): string {
  const texte = new Intl.DateTimeFormat(locale, { weekday: 'long', day: 'numeric', month: 'long', timeZone: fuseau }).format(date)
  return texte.charAt(0).toLocaleUpperCase(locale) + texte.slice(1)
}

export function memeJour(a: Date, b: Date): boolean {
  return jourCivil(a) === jourCivil(b)
}

export interface DocumentRecent {
  document: LibraryDocument
  /** La page notée pour la version servie ; nulle si aucune, ou pour une autre version. */
  page: number | null
  /** Instant ISO de la dernière lecture : l'ouverture, ou la dernière page notée si elle est plus tardive. */
  lu: string
}

/**
 * Les derniers ouverts sur ce téléphone, croisés avec la bibliothèque gardée : un
 * document dépublié n'y est plus, il ne paraît plus. Un réservé fermé à la personne
 * ne paraît pas : sur un téléphone partagé, il dirait ce qu'une autre a lu.
 */
export function documentsRecents(
  recents: readonly Recent[],
  documents: readonly LibraryDocument[],
  progressionDe: (id: string, version: string) => Progression | null,
): DocumentRecent[] {
  const parId = new Map(documents.map((d) => [d.id, d]))
  return recents.flatMap((recent) => {
    const document = parId.get(recent.id)
    if (!document || (document.restricted && !document.accessible)) return []
    const progression = progressionDe(document.id, document.version)
    const lu = progression && progression.a > recent.a ? progression.a : recent.a
    return [{ document, page: progression?.page ?? null, lu }]
  })
}

// ---------------------------------------------------------------------------
// Le temps au centre (ADR-023) : compte à rebours, jour de la COP, fil du jour
// ---------------------------------------------------------------------------

const instant = (iso: string) => new Date(iso).getTime()

/** Jours civils de `de` à `a`, deux clés `AAAA-MM-JJ` : compté sur le calendrier, pas sur 24 heures. */
export function joursEntre(de: string, a: string): number {
  const utc = (jour: string) => {
    const [an, mois, j] = jour.split('-').map(Number) as [number, number, number]
    return Date.UTC(an, mois - 1, j)
  }
  return Math.round((utc(a) - utc(de)) / 86_400_000)
}

export type CompteARebours =
  | { unite: 'minutes'; minutes: number }
  | { unite: 'heures'; heures: number; minutes: number }
  | { unite: 'demain' }
  | { unite: 'jours'; jours: number }

/**
 * « 42 min » sous l'heure, « 3 h 20 » dans la journée, au-delà le jour : « Demain »,
 * puis « 35 jours ». Les jours se comptent dans le fuseau de la COP.
 */
export function compteARebours(cible: string, maintenant: Date, fuseau: string): CompteARebours {
  const minutes = Math.max(0, Math.ceil((instant(cible) - maintenant.getTime()) / 60_000))
  if (minutes < 60) return { unite: 'minutes', minutes }
  const jours = joursEntre(dayKeyInZone(maintenant, fuseau), dayKeyInZone(cible, fuseau))
  if (jours <= 0) return { unite: 'heures', heures: Math.floor(minutes / 60), minutes: minutes % 60 }
  return jours === 1 ? { unite: 'demain' } : { unite: 'jours', jours }
}

export type MomentDeLaCop = 'avant' | 'pendant'

/** Avant l'ouverture seulement si on la connaît : une garde sans date ouvre l'accueil de la COP. */
export function momentDeLaCop(edition: { debut?: string } | null, maintenant: Date): MomentDeLaCop {
  return edition?.debut && maintenant.getTime() < instant(edition.debut) ? 'avant' : 'pendant'
}

/** « jour 2 sur 12 » ; nul hors de la COP ou sans ses dates. */
export function jourDeLaCop(
  edition: { debut?: string; fin?: string } | null,
  maintenant: Date,
  fuseau: string,
): { jour: number; total: number } | null {
  if (!edition?.debut || !edition.fin) return null
  const ouverture = dayKeyInZone(edition.debut, fuseau)
  const jour = joursEntre(ouverture, dayKeyInZone(maintenant, fuseau)) + 1
  const total = joursEntre(ouverture, dayKeyInZone(edition.fin, fuseau)) + 1
  return jour >= 1 && jour <= total ? { jour, total } : null
}

/** Un créneau d'un agenda ; `fin` absente, il ne vaut que son début (on n'invente pas de durée). */
export interface CreneauDuFil {
  debut: string
  fin: string | null
  marque?: boolean
}

export interface PlaceDansLeFil {
  /** Fractions de la plage, de 0 à 1. */
  gauche: number
  /** Nulle : sans fin, le créneau se dessine en point. */
  largeur: number | null
  marque: boolean
}

export interface FilDuJour {
  /** Heures murales, entières. */
  debut: number
  fin: number
  graduations: number[]
  pistes: PlaceDansLeFil[][]
  /** Fraction de la plage ; nulle hors plage. */
  maintenant: number | null
}

const PLAGE_DU_FIL = { debut: 8, fin: 20 } as const

/** L'heure murale d'un instant dans le jour `jour` : 0 la veille, 24 le lendemain. */
function heureDansLeJour(iso: string | Date, jour: string, fuseau: string): number {
  const mur = wallClockInZone(iso, fuseau)
  const ecart = joursEntre(jour, mur.slice(0, 10))
  if (ecart < 0) return 0
  if (ecart > 0) return 24
  return Number(mur.slice(11, 13)) + Number(mur.slice(14, 16)) / 60
}

/**
 * Le fil du jour : 08 h à 20 h, élargi si un créneau en sort, puis jusqu'à une
 * plage divisible par trois pour que les quatre graduations tombent sur des heures.
 */
export function filDuJour(pistes: readonly (readonly CreneauDuFil[])[], maintenant: Date, fuseau: string): FilDuJour {
  const jour = dayKeyInZone(maintenant, fuseau)
  const heures = pistes.flat().map((c) => ({
    debut: heureDansLeJour(c.debut, jour, fuseau),
    fin: c.fin ? heureDansLeJour(c.fin, jour, fuseau) : null,
    marque: c.marque ?? false,
  }))
  let debut = Math.min(PLAGE_DU_FIL.debut, ...heures.map((h) => Math.floor(h.debut)))
  let fin = Math.max(PLAGE_DU_FIL.fin, ...heures.map((h) => Math.ceil(h.fin ?? h.debut)))
  while ((fin - debut) % 3 !== 0) {
    if (fin < 24) fin += 1
    else debut -= 1
  }
  const duree = fin - debut
  const fraction = (h: number) => (h - debut) / duree
  let i = 0
  const places = pistes.map((piste) =>
    piste.map(() => {
      const h = heures[i++] as { debut: number; fin: number | null; marque: boolean }
      return {
        gauche: fraction(h.debut),
        largeur: h.fin === null ? null : Math.max(0, h.fin - h.debut) / duree,
        marque: h.marque,
      }
    }),
  )
  const ici = heureDansLeJour(maintenant, jour, fuseau)
  return {
    debut,
    fin,
    graduations: [0, 1, 2, 3].map((k) => debut + (k * duree) / 3),
    pistes: places,
    maintenant: ici >= debut && ici <= fin ? fraction(ici) : null,
  }
}

export type GenreDeChangement = 'salle' | 'heure' | 'deplacee' | 'annulee'

export interface ChangementDuJour {
  session: OfficialSession
  genre: GenreDeChangement
  /** Le réseau se pose par-dessus la source, sans la modifier : son encart l'emporte à l'affichage. */
  origine: 'source' | 'reseau'
  /** La nouvelle salle, ou la nouvelle heure de début (ISO). */
  salle: string | null
  debut: string
}

function changementDe(session: OfficialSession, maintenant: Date, fuseau: string): ChangementDuJour | null {
  const base = { session, salle: session.venue, debut: session.start_at }
  if (session.status === 'cancelled') return { ...base, genre: 'annulee', origine: 'source' }
  const encart = encartsAffiches(session, maintenant, fuseau)[0]
  if (encart?.reason === 'cancelled') return { ...base, genre: 'annulee', origine: 'reseau' }
  if (encart?.reason === 'venue' && encart.proposed_venue)
    return { ...base, genre: 'salle', origine: 'reseau', salle: encart.proposed_venue }
  if (encart?.reason === 'time' && encart.proposed_start)
    return { ...base, genre: 'heure', origine: 'reseau', debut: encart.proposed_start }
  const avant = session.previous
  if (!avant) return null
  const salle = avant.venue !== session.venue
  const heure = avant.start_at !== session.start_at
  if (!salle && !heure) return null
  return { ...base, genre: salle && heure ? 'deplacee' : salle ? 'salle' : 'heure', origine: 'source' }
}

/** Les changements des sessions du jour qui ne sont pas terminées, par heure de début. */
export function changementsDuJour(
  sessions: readonly OfficialSession[],
  maintenant: Date,
  fuseau: string,
): ChangementDuJour[] {
  return sessionsDuJour(sessions, dayKeyInZone(maintenant, fuseau), fuseau)
    .filter((s) => s.status === 'cancelled' || etatAffiche(s, maintenant, fuseau) !== 'terminee')
    .flatMap((s) => changementDe(s, maintenant, fuseau) ?? [])
}

export interface JalonDatee {
  cle: string
  date: string
}

export interface PisteJusquALaCop {
  /** Fraction où commence la COP, de 0 (aujourd'hui) à 1 (clôture). */
  ouverture: number
  jalons: { cle: string; position: number }[]
}

/** La piste d'aujourd'hui à la clôture ; un jalon passé ou après l'ouverture n'y est pas. */
export function pisteJusquALaCop(
  maintenant: Date,
  debut: string,
  fin: string,
  jalons: readonly JalonDatee[],
): PisteJusquALaCop {
  const t = maintenant.getTime()
  const duree = Math.max(1, instant(fin) - t)
  const position = (iso: string) => Math.min(1, Math.max(0, (instant(iso) - t) / duree))
  return {
    ouverture: position(debut),
    jalons: [...jalons]
      .filter((j) => instant(j.date) > t && instant(j.date) < instant(debut))
      .sort((a, b) => instant(a.date) - instant(b.date))
      .map((j) => ({ cle: j.cle, position: position(j.date) })),
  }
}

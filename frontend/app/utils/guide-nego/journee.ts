/**
 * « Ma journée » : ses blocs, et le jour qu'elle affiche.
 *
 * **L'ordre des blocs est fixe** (FR-013) et vit ici, pas dans le gabarit : un test
 * le tient sans navigateur, et chaque étape qui remplit un bloc (1, 3a, 3b, 4, 5) le
 * trouve à sa place.
 */
import { jourCivil } from './connexion.ts'

export const BLOCS_DE_MA_JOURNEE = [
  'prochaine-session',
  'changements',
  'trois-agendas',
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
export function jourLisible(date: Date, locale: string): string {
  const texte = new Intl.DateTimeFormat(locale, { weekday: 'long', day: 'numeric', month: 'long' }).format(date)
  return texte.charAt(0).toLocaleUpperCase(locale) + texte.slice(1)
}

export function memeJour(a: Date, b: Date): boolean {
  return jourCivil(a) === jourCivil(b)
}

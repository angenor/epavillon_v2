/**
 * L'état de connexion, et la façon de dire l'heure d'une lecture.
 *
 * « lu à » et « Synchronisé à » disent l'heure du TÉLÉPHONE, sans fuseau : elles
 * servent à juger la fraîcheur contre l'horloge affichée au-dessus. Seules les heures
 * d'événement portent leur fuseau (écart 32).
 */
export interface EtatConnexion {
  enLigne: boolean
  /** Instant ISO de la lecture réussie la plus récente, toutes lectures confondues. */
  luA: string | null
  /** Le bandeau se montre une fois par épisode hors connexion. */
  bandeauVu: boolean
}

export const connexionInitiale = (enLigne: boolean): EtatConnexion => ({
  enLigne,
  luA: null,
  bandeauVu: false,
})

function plusRecent(a: string | null, b: string | null): string | null {
  if (!a) return b
  if (!b) return a
  return a > b ? a : b
}

export function apresReussite(etat: EtatConnexion, luA: string): EtatConnexion {
  return { enLigne: true, luA: plusRecent(etat.luA, luA), bandeauVu: false }
}

/** Un épisode hors connexion qui commence remet le bandeau à montrer ; un épisode en cours, non. */
export function apresEchec(etat: EtatConnexion): EtatConnexion {
  return etat.enLigne ? { ...etat, enLigne: false, bandeauVu: false } : etat
}

/**
 * Le navigateur annonce le retour du réseau. On le croit — c'est ce qui rend les
 * boutons à leur état actif et fait partir la file — et la prochaine lecture le
 * confirme ou le dément. Le bandeau ne bouge pas : seule une lecture réussie le retire.
 */
export function apresAnnonceEnLigne(etat: EtatConnexion): EtatConnexion {
  return etat.enLigne ? etat : { ...etat, enLigne: true }
}

export function apresLectureGardee(etat: EtatConnexion, luA: string): EtatConnexion {
  return { ...etat, luA: plusRecent(etat.luA, luA) }
}

export type MomentLecture =
  | { quand: 'aujourdhui'; heure: string }
  | { quand: 'hier'; heure: string }
  | { quand: 'avant'; heure: string; jour: string }

const jourCivil = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()

/**
 * « à 14:05 » le jour même, « hier à 23:10 », puis « le 11 nov. à 23:10 » : une heure
 * seule tromperait dès le lendemain. L'année s'ajoute quand elle diffère.
 */
export function momentDeLecture(luA: string, maintenant: Date, locale = 'fr'): MomentLecture {
  const lecture = new Date(luA)
  const heure = new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit', hour12: false }).format(lecture)
  const ecartJours = Math.round((jourCivil(maintenant) - jourCivil(lecture)) / 86_400_000)

  if (ecartJours <= 0) return { quand: 'aujourdhui', heure }
  if (ecartJours === 1) return { quand: 'hier', heure }

  const memeAnnee = lecture.getFullYear() === maintenant.getFullYear()
  const jour = new Intl.DateTimeFormat(locale, {
    day: 'numeric',
    month: 'short',
    ...(memeAnnee ? {} : { year: 'numeric' }),
  }).format(lecture)
  return { quand: 'avant', heure, jour }
}

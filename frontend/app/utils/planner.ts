/**
 * LE PLANIFICATEUR DE CRÉNEAUX (A9) — fonctions pures.
 *
 * Filtrage et tri du panneau latéral, facettes, index des conflits par séance,
 * conversions d'heure entre la grille et le modèle. Rien ici ne touche au DOM ni
 * à i18n : les libellés arrivent déjà traduits, et les dates déjà situées.
 *
 * DEUX RÈGLES SE JOUENT DANS CE FICHIER, et toutes deux ont déjà coûté un bogue
 * ailleurs :
 *
 *   · LE FUSEAU. La bibliothèque de calendrier place ses blocs à l'heure de la
 *     MACHINE. On convertit donc vers le fuseau de l'ÉDITION avant de lui donner
 *     quoi que ce soit (`wallClockInZone`), et on reconvertit dans l'autre sens
 *     ce qu'elle rend (`instantFromDroppedDate`). Sans ce double passage, un
 *     déplacement fait depuis Dakar décalerait le créneau de trois heures, sans
 *     qu'aucune erreur ne soit levée.
 *
 *   · AUCUN CHEVAUCHEMENT N'EST BLOQUÉ. Aucune fonction d'ici ne rend « refusé ».
 *     `conflictsBySession()` classe pour AFFICHER, jamais pour empêcher.
 */

import type { PlannerFacet, PlannerSession, UnplacedFilters, UnplacedSortKey, UnplacedFacets } from '~/types/admin-planner'
import type { ScheduleConflict, ConflictSeverity } from '~/types/programme/session'
import type { IsoDateTime, TimeZoneName, Uuid } from '~/types/shared'

// ---------------------------------------------------------------------------
// Panneau latéral : filtrer, trier, compter
// ---------------------------------------------------------------------------

/**
 * Libellés que les fonctions pures ne peuvent pas produire seules.
 *
 * Deux natures de texte, et les confondre est le défaut n° 1 de la v1 : le
 * FORMAT est un libellé d'interface, traduit par i18n ; le TITRE, le nom de
 * l'organisation et la THÉMATIQUE sont des données de la base, résolues par
 * l'utilitaire multilingue. Aucune des deux ne se lit ici en `.fr`.
 */
export interface PlannerSessionText {
  /** Format traduit — « Sur place », « En ligne », « Hybride ». */
  format: (session: PlannerSession) => string
  /** Organisation résolue : sigle à défaut du nom légal. */
  organization: (session: PlannerSession) => string
  /** Titre résolu dans la locale active. */
  title: (session: PlannerSession) => string
  /** Libellé d'une thématique, venu de `reference.taxonomy_terms`. */
  theme: (badge: PlannerSession['themes'][number]) => string
}

/** Normalise pour la recherche : sans accents, sans casse. */
function fold(value: string): string {
  return value
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

/**
 * Filtre les activités à placer.
 *
 * La recherche porte sur le titre, l'organisation et le NUMÉRO DE DOSSIER : au
 * téléphone avec une organisation, c'est le numéro qu'on a sous les yeux, pas le
 * titre exact.
 */
export function filterUnplaced(
  sessions: PlannerSession[],
  filters: UnplacedFilters,
  text: PlannerSessionText,
): PlannerSession[] {
  const needle = fold(filters.search.trim())

  return sessions.filter((session) => {
    if (needle) {
      const haystack = fold(
        [text.title(session), text.organization(session), session.reference_code ?? ''].join(' '),
      )
      if (!haystack.includes(needle)) return false
    }
    if (filters.themes.length > 0) {
      const codes = session.themes.map((theme) => theme.code)
      if (!filters.themes.some((code) => codes.includes(code))) return false
    }
    if (filters.formats.length > 0 && !filters.formats.includes(session.format)) return false
    if (
      filters.organizations.length > 0 &&
      (session.organization_id === null || !filters.organizations.includes(session.organization_id))
    ) {
      return false
    }
    return true
  })
}

/**
 * Trie les activités à placer.
 *
 * LA NOTE EST LE TRI PAR DÉFAUT, décroissante : c'est l'ordre du comité, et la
 * première question de l'équipe devant un pavillon à remplir. Les activités sans
 * dossier — donc sans note — ferment la liste au lieu de l'ouvrir : les mettre
 * en tête ferait passer les dossiers les mieux notés sous la ligne de flottaison.
 */
export function sortUnplaced(
  sessions: PlannerSession[],
  key: UnplacedSortKey,
  text: PlannerSessionText,
): PlannerSession[] {
  const rows = [...sessions]

  switch (key) {
    case 'score':
      return rows.sort((a, b) => (b.average_score ?? -1) - (a.average_score ?? -1))
    case 'duration':
      return rows.sort(
        (a, b) => (b.requested_duration_minutes ?? 0) - (a.requested_duration_minutes ?? 0),
      )
    case 'preferred':
      // Sans créneau souhaité, l'activité passe en fin de liste : elle n'attend
      // rien de particulier, l'équipe la posera où il reste de la place.
      return rows.sort((a, b) =>
        (a.preferred_start_at ?? '9999').localeCompare(b.preferred_start_at ?? '9999'),
      )
    case 'title':
      return rows.sort((a, b) => text.title(a).localeCompare(text.title(b)))
  }
}

/**
 * Facettes du panneau : chaque valeur avec son décompte.
 *
 * Calculées sur la liste NON filtrée, sinon les valeurs disparaissent au fur et
 * à mesure qu'on filtre et l'on ne peut plus élargir sa recherche.
 */
export function unplacedFacets(
  sessions: PlannerSession[],
  text: PlannerSessionText,
): UnplacedFacets {
  const themes = new Map<string, PlannerFacet>()
  const formats = new Map<string, PlannerFacet>()
  const organizations = new Map<string, PlannerFacet>()

  for (const session of sessions) {
    for (const theme of session.themes) {
      const existing = themes.get(theme.code)
      if (existing) existing.count += 1
      else
        themes.set(theme.code, {
          value: theme.code,
          // Libellé et couleur viennent de la BASE (`reference.taxonomy_terms`),
          // jamais d'un fichier i18n : c'est le défaut n° 1 de la v1.
          label: text.theme(theme),
          count: 1,
          color: theme.color,
        })
    }

    const format = formats.get(session.format)
    if (format) format.count += 1
    else formats.set(session.format, { value: session.format, label: text.format(session), count: 1 })

    if (session.organization_id) {
      const organization = organizations.get(session.organization_id)
      if (organization) organization.count += 1
      else
        organizations.set(session.organization_id, {
          value: session.organization_id,
          label: text.organization(session),
          count: 1,
        })
    }
  }

  const byCount = (a: PlannerFacet, b: PlannerFacet) => b.count - a.count || a.label.localeCompare(b.label)

  return {
    themes: [...themes.values()].sort(byCount),
    formats: [...formats.values()].sort(byCount),
    organizations: [...organizations.values()].sort(byCount),
  }
}

// ---------------------------------------------------------------------------
// Conflits
// ---------------------------------------------------------------------------

/** Résumé d'un conflit du point de vue d'UNE séance. */
export interface SessionConflictMark {
  severity: ConflictSeverity
  /** Le pire des conflits de cette séance décide de sa marque dans la grille. */
  kinds: ScheduleConflict['conflict_kind'][]
  count: number
}

/**
 * Index des conflits par séance : les deux séances d'une paire y figurent.
 *
 * `detect_conflicts()` rend une PAIRE (`session_a`, `session_b`) ; la grille,
 * elle, marque des BLOCS. Sans cet index, seul le premier bloc de chaque paire
 * serait signalé, et l'équipe chercherait longtemps pourquoi le second n'a rien.
 */
export function conflictsBySession(conflicts: ScheduleConflict[]): Map<Uuid, SessionConflictMark> {
  const marks = new Map<Uuid, SessionConflictMark>()

  const add = (sessionId: Uuid, conflict: ScheduleConflict): void => {
    const existing = marks.get(sessionId)
    if (!existing) {
      marks.set(sessionId, { severity: conflict.severity, kinds: [conflict.conflict_kind], count: 1 })
      return
    }
    existing.count += 1
    // Le bloquant l'emporte : un bloc qui porte les deux se marque en rouge.
    if (conflict.severity === 'blocking') existing.severity = 'blocking'
    if (!existing.kinds.includes(conflict.conflict_kind)) existing.kinds.push(conflict.conflict_kind)
  }

  for (const conflict of conflicts) {
    add(conflict.session_a, conflict)
    add(conflict.session_b, conflict)
  }
  return marks
}

/** Décompte par gravité, pour le compteur du bandeau. */
export function countBySeverity(conflicts: ScheduleConflict[]): Record<ConflictSeverity, number> {
  return {
    blocking: conflicts.filter((conflict) => conflict.severity === 'blocking').length,
    warning: conflicts.filter((conflict) => conflict.severity === 'warning').length,
  }
}

// ---------------------------------------------------------------------------
// Heures : de la grille au modèle, et retour
// ---------------------------------------------------------------------------

/**
 * Heure murale d'une `Date` rendue par le calendrier.
 *
 * La bibliothèque raisonne en heure LOCALE de la machine : ses dates se lisent
 * par leurs composantes, jamais par leur instant. Passer par `toISOString()`
 * appliquerait le décalage du poste et déplacerait le bloc.
 */
export function wallClockFromLocalDate(date: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0')
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}` +
    ` ${pad(date.getHours())}:${pad(date.getMinutes())}`
  )
}

/**
 * L'instant correspondant à un bloc déposé ou redimensionné.
 *
 * L'heure lue sur la grille est celle du PAVILLON : déposer un bloc à 14 h veut
 * dire 14 h à Belém, quel que soit l'endroit d'où l'équipe travaille.
 */
export function instantFromDroppedDate(date: Date, timeZone: TimeZoneName): IsoDateTime | null {
  return instantFromWallClock(wallClockFromLocalDate(date), timeZone)
}

/** Fin d'un créneau, à partir d'un début et d'une durée en minutes. */
export function endOfSlot(startsAt: IsoDateTime, minutes: number): IsoDateTime {
  return new Date(Date.parse(startsAt) + minutes * 60_000).toISOString()
}

/** Durée d'une séance en minutes, telle qu'elle est actuellement placée. */
export function durationOf(session: PlannerSession): number {
  return Math.max(15, Math.round((Date.parse(session.ends_at) - Date.parse(session.starts_at)) / 60_000))
}

/**
 * Durée à donner à une activité qu'on place pour la première fois : celle
 * demandée au dépôt, à défaut celle du créneau déjà porté par la séance.
 */
export function plannedDuration(session: PlannerSession): number {
  return session.requested_duration_minutes ?? durationOf(session)
}

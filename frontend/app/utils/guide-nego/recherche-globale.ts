/**
 * La recherche globale (récit 4) : ce qui se décide sans Nuxt — les sessions du jour,
 * les réunions de la Francophonie et les activités du Pavillon trouvées, et les documents trouvés par leur fiche
 * ou par leur texte, en ligne comme sur les copies gardées.
 */
import type { LibraryDocument, PageHit } from '~/types/negotiation-documents'
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import type { PublicScheduleRow } from '~/types/views'
import type { OfficialSession } from '~/types/negotiation-sessions'
import { correspondALaRecherche } from './documents.ts'
import { expressionRepliee, LONGUEUR_MINIMALE, type Passage } from './lecteur.ts'
import { trierLesActivites } from './pavillon.ts'
import { sessionsDuJour } from './sessions.ts'
import { normalizeSearch } from '../proposal-list.ts'

/** Au-delà, un document long noierait les autres groupes. */
export const PASSAGES_PAR_DOCUMENT = 5

export const rechercheLancee = (saisie: string): boolean => expressionRepliee(saisie).length >= LONGUEUR_MINIMALE

/** Les sessions du jour que la liste ouvrirait, dont le titre ou la salle contient la saisie. */
export function sessionsTrouvees(
  sessions: readonly OfficialSession[],
  saisie: string,
  jour: string | null,
  fuseau: string,
): OfficialSession[] {
  const cherche = normalizeSearch(saisie)
  if (!jour || !rechercheLancee(saisie)) return []
  return sessionsDuJour(sessions, jour, fuseau).filter((s) =>
    [s.title_en, s.title_fr, s.venue].some((c) => !!c && normalizeSearch(c).includes(cherche)),
  )
}

/** Les réunions de l'édition dont le titre, dans l'une de ses langues, ou le lieu contient la saisie. */
export function reunionsTrouvees(reunions: readonly FrancophoneMeeting[], saisie: string): FrancophoneMeeting[] {
  if (!rechercheLancee(saisie)) return []
  const cherche = normalizeSearch(saisie)
  return reunions.filter((r) =>
    [...Object.values(r.title), r.venue].some((c) => !!c && normalizeSearch(c).includes(cherche)),
  )
}

/** Les activités du Pavillon de l'édition dont le titre, dans l'une de ses langues, contient la saisie. */
export function activitesTrouvees(activites: readonly PublicScheduleRow[], saisie: string): PublicScheduleRow[] {
  if (!rechercheLancee(saisie)) return []
  const cherche = normalizeSearch(saisie)
  return trierLesActivites(activites.filter((a) => Object.values(a.title).some((c) => !!c && normalizeSearch(c).includes(cherche))))
}

export interface PassageTrouve {
  /** Le rang de la page, pour `?page=` du lecteur. */
  page: number
  etiquette: string
  extrait: string
}

/** Les `TextHit` de l'API, tels que l'état `readonly` d'un composable les rend. */
export type PagesServies = readonly { readonly document_id: string; readonly pages: readonly Readonly<PageHit>[] }[]

export interface DocumentTrouve {
  document: LibraryDocument
  /** Vide quand le document n'est trouvé que par son titre, son résumé ou son éditeur. */
  passages: PassageTrouve[]
}

export const passageDuTelephone = (p: Passage): PassageTrouve => ({
  page: p.page,
  etiquette: p.etiquette,
  extrait: p.extrait.avant + p.extrait.trouve + p.extrait.apres,
})

/**
 * Les documents trouvés, ceux dont la fiche répond d'abord. `enLigne` : les pages
 * servies par l'API, qui cherche le texte de tous les documents accessibles ; nul sans
 * elle, et le texte ne se cherche alors que dans les copies gardées.
 */
export function documentsTrouves(
  documents: readonly LibraryDocument[],
  saisie: string,
  enLigne: PagesServies | null,
  gardes: ReadonlyMap<string, readonly Passage[]>,
): DocumentTrouve[] {
  if (!rechercheLancee(saisie)) return []
  const passagesDe = new Map<string, PassageTrouve[]>()
  if (enLigne) {
    for (const hit of enLigne) {
      passagesDe.set(hit.document_id, hit.pages.map((p) => ({ page: p.index, etiquette: p.label, extrait: p.excerpt })))
    }
  } else {
    for (const [id, passages] of gardes) if (passages.length) passagesDe.set(id, passages.map(passageDuTelephone))
  }
  const borne = (d: LibraryDocument): DocumentTrouve => ({
    document: d,
    passages: (passagesDe.get(d.id) ?? []).slice(0, PASSAGES_PAR_DOCUMENT),
  })
  const parFiche = documents.filter((d) => correspondALaRecherche(d, saisie, null))
  const retenus = new Set(parFiche.map((d) => d.id))
  const parTexte = [...passagesDe.keys()].flatMap((id) => {
    const d = retenus.has(id) ? undefined : documents.find((x) => x.id === id)
    return d ? [d] : []
  })
  return [...parFiche, ...parTexte].map(borne)
}

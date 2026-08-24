/**
 * GESTION DES ÉVÉNEMENTS, BACK-OFFICE (A10) — point d'entrée.
 *
 * Ne contient aucune donnée ni aucune logique : il ne fait que ré-exporter.
 *
 *   core.ts     les tableaux rendus mutables, les dates civiles, la ligne de
 *               liste et les listes de référence du formulaire
 *   detail.ts   les LECTURES : la composition des six onglets
 *   writes.ts   les ÉCRITURES de l'édition et de ses quatre premiers onglets
 *   calls.ts    l'APPEL À PROPOSITIONS et son COMITÉ — l'autre permission
 *
 * La dépendance ne va que dans un sens — `writes`/`calls` → `detail` → `core` —
 * et ce découpage tient chaque fichier sous le garde-fou de mille lignes de
 * `CLAUDE.md`.
 */

export { editionListScreen, editionFormOptions } from './core'
export { editionDetail, planDayGeneration } from './detail'
export {
  saveEdition,
  attachEditionImages,
  generateEventDays,
  saveEventDay,
  saveTrack,
  removeTrack,
  saveVenue,
  removeVenue,
  saveRoom,
  removeRoom,
  saveChannel,
  removeChannel,
} from './writes'
export { saveCall, defaultCriteriaGrid, saveCommittee } from './calls'

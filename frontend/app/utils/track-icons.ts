/**
 * Les pictogrammes qu'un fil peut porter (`programme_tracks.icon_code`) : des
 * dessins de `UiIcon`, dont le code est le nom. La liste vit ici et pas en
 * base : un code sans son tracé ne dessinerait rien (arbitrage du 29/10, comme
 * les modèles de l'atelier de couverture). Ajouter un dessin : son tracé dans
 * `UiIcon`, son code ici, son libellé dans `admin.event.tabs.tracksTab.form.icons`.
 */
export const TRACK_ICONS = [
  'coins',
  'sprout',
  'handshake',
  'globe',
  'book',
  'mic',
  'gender',
  'droplet',
  'sun',
  'tree',
  'building',
  'wave',
] as const

export type TrackIconCode = (typeof TRACK_ICONS)[number]

/** Le dessin d'un code ; nul sans code, ou pour un code que le site ne connaît plus. */
export function trackIcon(code: string | null | undefined): TrackIconCode | null {
  return code && (TRACK_ICONS as readonly string[]).includes(code) ? (code as TrackIconCode) : null
}

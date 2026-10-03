/** `public/bird-cursor.js` — l'oiseau de l'accueil. Seule la surface utilisée est décrite. */
export interface BirdCursorOptions {
  scale?: number
  near?: number
  idle?: number
  stiff?: number
  damp?: number
  lag?: number
  gaze?: number
  standoff?: number
  zIndex?: number
}

declare global {
  interface Window {
    BirdCursor?: {
      mount: (options?: BirdCursorOptions) => void
      destroy: () => void
    }
  }
}

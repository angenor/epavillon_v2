/**
 * Les ressources d'une journée (10/11) : le genre se lit dans l'adresse, il ne
 * se saisit pas. Une vidéo YouTube se regarde sur place, un PDF se télécharge,
 * le reste s'ouvre. Sans Vue, pour les tests.
 */

export type TrackResourceKind = 'video' | 'pdf' | 'page'

export interface TrackResourceKindOf {
  kind: TrackResourceKind
  /** Identifiant YouTube d'une vidéo. */
  videoId: string | null
  /** « ifdd.francophonie.org » ; nul pour un chemin du site. */
  site: string | null
}

const YOUTUBE_ID = /^[\w-]{11}$/

/** L'identifiant d'une vidéo YouTube : `watch?v=`, `youtu.be/`, `shorts/`, `embed/`, `live/`. */
export function youtubeIdOf(address: string): string | null {
  let url: URL
  try {
    url = new URL(address)
  } catch {
    return null
  }
  const host = url.hostname.replace(/^(www\.|m\.)/, '')
  let id: string | null = null
  if (host === 'youtu.be') id = url.pathname.slice(1).split('/')[0] ?? null
  else if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    if (url.pathname === '/watch') id = url.searchParams.get('v')
    else id = url.pathname.match(/^\/(?:shorts|embed|live)\/([^/?#]+)/)?.[1] ?? null
  }
  return id && YOUTUBE_ID.test(id) ? id : null
}

export function resourceKindOf(address: string): TrackResourceKindOf {
  if (address.startsWith('/')) {
    return { kind: /\.pdf$/i.test(address.split(/[?#]/)[0] ?? '') ? 'pdf' : 'page', videoId: null, site: null }
  }
  let url: URL | null = null
  try {
    url = new URL(address)
  } catch {
    url = null
  }
  const site = url ? url.hostname.replace(/^www\./, '') : null
  const videoId = youtubeIdOf(address)
  if (videoId) return { kind: 'video', videoId, site }
  return { kind: url && /\.pdf$/i.test(url.pathname) ? 'pdf' : 'page', videoId: null, site }
}

/** Vignette d'une vidéo : la grande est en 4:3, bandes noires comprises (l'affichage les rogne). */
export const youtubeThumbnail = (videoId: string): string => `https://img.youtube.com/vi/${videoId}/hqdefault.jpg`

/** Lecteur sans cookie de suivi tant qu'on ne lance pas la vidéo. */
export const youtubeEmbed = (videoId: string): string => `https://www.youtube-nocookie.com/embed/${videoId}?autoplay=1&rel=0`

export const youtubeWatch = (videoId: string): string => `https://www.youtube.com/watch?v=${videoId}`

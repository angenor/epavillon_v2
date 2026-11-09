import type { OnlineSpeakerInput } from '~/types/programme/online-speaker'

/** Les deux adresses mises en copie de chaque envoi, telles que l'API les pose : affichées, jamais envoyées d'ici. */
export const SPEAKER_LINKS_CC = ['angenor99@gmail.com', 'issa.bado@francophonie.org'] as const

export const ZOOM_RETRY_MS = 3_000
export const ZOOM_WAIT_MAX_MS = 60_000

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

/** Le jeton du lien d'inscription part en en-tête : une adresse finit dans les journaux et les `Referer`. */
export const speakerLinkHeaders = (token: string | null | undefined): Record<string, string> | undefined =>
  token ? { 'X-Speaker-Link': token } : undefined

export function blankOnlineSpeaker(): OnlineSpeakerInput {
  return {
    civility: null,
    first_name: '',
    last_name: '',
    job_title: '',
    organization: '',
    email: '',
    country_iso2: null,
    bio: null,
    consent: false,
  }
}

export type OnlineSpeakerField = 'first_name' | 'last_name' | 'job_title' | 'organization' | 'email'

/** Le minimum vérifiable ici : champs requis et forme de l'adresse. Le reste, l'API le dit. */
export function onlineSpeakerProblems(input: OnlineSpeakerInput): Partial<Record<OnlineSpeakerField, 'required' | 'email'>> {
  const problems: Partial<Record<OnlineSpeakerField, 'required' | 'email'>> = {}
  for (const field of ['first_name', 'last_name', 'job_title', 'organization', 'email'] as const) {
    if (!input[field].trim()) problems[field] = 'required'
  }
  if (!problems.email && !EMAIL_PATTERN.test(input.email.trim())) problems.email = 'email'
  return problems
}

export function cleanOnlineSpeaker(input: OnlineSpeakerInput): OnlineSpeakerInput {
  const bio = input.bio?.trim() ?? ''
  return {
    ...input,
    first_name: input.first_name.trim(),
    last_name: input.last_name.trim(),
    job_title: input.job_title.trim(),
    organization: input.organization.trim(),
    email: input.email.trim(),
    bio: bio || null,
  }
}

const codeOf = (error: unknown): string | null =>
  typeof error === 'object' && error !== null && 'code' in error && typeof error.code === 'string' ? error.code : null

export interface ZoomWaitOptions {
  retryMs?: number
  maxMs?: number
  signal?: AbortSignal
}

/**
 * Le lien Zoom d'un intervenant tout juste inscrit. La réunion peut ne pas être
 * prête (409 `LIVE_MEETING_NOT_READY`) : on réessaie, au plus une minute. Nul
 * quand l'attente s'épuise ou que Zoom ne répond pas (503) : le courriel prendra
 * le relais, ce n'est pas une erreur. Tout autre refus remonte tel quel.
 */
export async function awaitZoomLink<T>(fetchLink: () => Promise<T>, options: ZoomWaitOptions = {}): Promise<T | null> {
  const retryMs = options.retryMs ?? ZOOM_RETRY_MS
  const maxMs = options.maxMs ?? ZOOM_WAIT_MAX_MS
  const started = Date.now()
  for (;;) {
    if (options.signal?.aborted) return null
    try {
      return await fetchLink()
    } catch (error) {
      const code = codeOf(error)
      if (code === 'LIVE_ZOOM_UNAVAILABLE') return null
      if (code !== 'LIVE_MEETING_NOT_READY') throw error
      if (Date.now() - started + retryMs > maxMs) return null
      await new Promise<void>((resolve) => setTimeout(resolve, retryMs))
    }
  }
}

/** « 12 novembre », pour « le 12 novembre à 11:30 » : l'année est celle de l'édition. */
export function dayMonth(value: string, timeZone: string, locale: string): string {
  return new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'long', timeZone }).format(new Date(value))
}

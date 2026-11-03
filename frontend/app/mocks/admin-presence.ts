/**
 * La carte « En ce moment », hors ligne : un après-midi de direct. Les heures
 * sont recalées sur maintenant, la courbe monte vers le début de la séance
 * puis se tasse.
 */

import type { AdminPresence, PresenceSample, PresenceZoneCount } from '~/types/admin-presence'

const ZONES: PresenceZoneCount[] = [
  { zone: 'live', count: 187 },
  { zone: 'programme', count: 97 },
  { zone: 'day', count: 61 },
  { zone: 'activity', count: 36 },
  { zone: 'organization', count: 13 },
  { zone: 'admin', count: 9 },
  { zone: 'other', count: 9 },
]

const CURVE = [
  120, 124, 131, 138, 142, 150, 158, 171, 186, 204, 226, 251, 280, 312, 348, 377, 401, 418, 427, 431,
  429, 424, 420, 417, 415, 413, 410, 409, 412, 414, 411, 408, 405, 406, 409, 411, 413, 410, 407, 404,
  402, 405, 407, 409, 411, 410, 412, 414, 413, 411, 409, 410, 412, 413, 411, 410, 409, 411, 412, 412,
]

export function adminPresence(now: number = Date.now()): AdminPresence {
  const minute = Math.floor(now / 60_000) * 60_000
  const last_hour: PresenceSample[] = CURVE.map((total, i) => ({
    at: new Date(minute - (CURVE.length - 1 - i) * 60_000).toISOString(),
    total,
  }))
  const peak = last_hour.reduce<PresenceSample | null>((best, s) => (!best || s.total >= best.total ? s : best), null)
  const total = ZONES.reduce((sum, z) => sum + z.count, 0)
  return {
    total,
    signed_in: 58,
    visitors: total - 58,
    zones: ZONES,
    last_hour,
    peak,
    observed_at: new Date(now).toISOString(),
  }
}

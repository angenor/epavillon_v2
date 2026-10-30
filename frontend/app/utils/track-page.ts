import type { TrackPageSettings, TrackPagePayload } from '~/types/event/track-page'
import type { AttachedImage, AttachedMedia, AttachmentAssignment } from '~/types/media'
import type { AssetId, I18nText, OrganizationId, Url, VenueId } from '~/types/shared'

/**
 * Le brouillon de la page d'un fil au back-office : les champs de
 * `PUT /admin/tracks/{id}/page` et les médias du fil, enregistrés d'un geste.
 * Les parties, moments et placements ont leurs propres routes et n'y sont pas.
 */

export interface TrackPartnerDraft {
  organization_id: OrganizationId
  name: string
  acronym: string | null
  logo: AttachedImage | null
  role_code: string
}

export interface TrackDocumentDraft {
  asset_id: AssetId
  filename: string
  url: Url | null
  /** Libellé du lien sur la page publique (« Résumé (PDF) ») : légende du rattachement. */
  caption: I18nText | null
}

export interface TrackPageDraft {
  venue_id: VenueId | null
  tagline: I18nText | null
  /** Une ligne par public ; `fr` vide tant que la saisie n'est pas faite. */
  target_audiences: I18nText[]
  support_note: I18nText | null
  feature_kicker: I18nText | null
  feature_title: I18nText | null
  feature_body: I18nText | null
  feature_link_label: I18nText | null
  feature_link_url: string
  partners: TrackPartnerDraft[]
  cover: AttachedImage | null
  cover_asset_id: AssetId | null
  visual: AttachedImage | null
  visual_asset_id: AssetId | null
  documents: TrackDocumentDraft[]
}

const OWNER = { owner_schema: 'event', owner_table: 'programme_tracks' } as const

export function trackMediaOwner(trackId: string) {
  return { schema: OWNER.owner_schema, table: OWNER.owner_table, id: trackId }
}

function asImage(media: AttachedMedia | undefined): AttachedImage | null {
  if (!media) return null
  return {
    asset_id: media.asset_id,
    url: media.url,
    width: media.width,
    height: media.height,
    alt_text: media.alt_text ?? { fr: '' },
    caption: media.caption,
    credit: media.credit,
    sources: media.sources,
  }
}

/** Le nom du fichier, lu sur son adresse : `GET /media/attachments` ne le rend pas. */
function filenameOf(url: string): string {
  const last = url.split('?')[0]?.split('/').pop() ?? url
  try {
    return decodeURIComponent(last)
  } catch {
    return last
  }
}

export function draftFromSettings(settings: TrackPageSettings, media: AttachedMedia[]): TrackPageDraft {
  const cover = asImage(media.find((m) => m.role === 'cover'))
  const visual = asImage(media.find((m) => m.role === 'thumbnail'))
  return {
    venue_id: settings.venue_id,
    tagline: settings.tagline,
    target_audiences: settings.target_audiences.map((a) => ({ ...a })),
    support_note: settings.support_note,
    feature_kicker: settings.feature_kicker,
    feature_title: settings.feature_title,
    feature_body: settings.feature_body,
    feature_link_label: settings.feature_link_label,
    feature_link_url: settings.feature_link_url ?? '',
    partners: [...settings.partners]
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((p) => ({
        organization_id: p.organization_id,
        name: p.name,
        acronym: p.acronym,
        logo: p.logo,
        role_code: p.role_code,
      })),
    cover,
    cover_asset_id: cover?.asset_id ?? null,
    visual,
    visual_asset_id: visual?.asset_id ?? null,
    documents: media
      .filter((m) => m.role === 'document')
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((m) => ({ asset_id: m.asset_id, filename: filenameOf(m.url), url: m.url, caption: m.caption })),
  }
}

/** Un texte vide n'est pas une valeur : la base garderait `{"fr": ""}`. */
function filled(text: I18nText | null): I18nText | null {
  return text && text.fr.trim() ? text : null
}

export function payloadFromDraft(draft: TrackPageDraft): TrackPagePayload {
  const url = draft.feature_link_url.trim()
  return {
    venue_id: draft.venue_id,
    tagline: filled(draft.tagline),
    target_audiences: draft.target_audiences.filter((a) => a.fr.trim() || a.en?.trim()),
    support_note: filled(draft.support_note),
    feature_kicker: filled(draft.feature_kicker),
    feature_title: filled(draft.feature_title),
    feature_body: filled(draft.feature_body),
    feature_link_label: filled(draft.feature_link_label),
    feature_link_url: url || null,
    partners: draft.partners.map((p) => ({ organization_id: p.organization_id, role_code: p.role_code })),
  }
}

/** Les trois rôles du fil, nommés à chaque envoi : chacun est vidé puis regarni. */
export function mediaAssignments(draft: TrackPageDraft): AttachmentAssignment[] {
  const documents: AttachmentAssignment[] = draft.documents.length
    ? draft.documents.map((d) => ({ role: 'document', asset_id: d.asset_id, caption_override: filled(d.caption) }))
    : [{ role: 'document', asset_id: null }]
  return [
    { role: 'cover', asset_id: draft.cover_asset_id },
    { role: 'thumbnail', asset_id: draft.visual_asset_id },
    ...documents,
  ]
}

export function mediaSignature(draft: TrackPageDraft): string {
  return JSON.stringify(mediaAssignments(draft))
}

export function pageSignature(draft: TrackPageDraft): string {
  return JSON.stringify(payloadFromDraft(draft))
}

/** Échange deux éléments voisins ; rend une nouvelle liste. */
export function moved<T>(list: readonly T[], index: number, delta: -1 | 1): T[] {
  const target = index + delta
  if (target < 0 || target >= list.length) return [...list]
  const next = [...list]
  const [item] = next.splice(index, 1)
  next.splice(target, 0, item as T)
  return next
}

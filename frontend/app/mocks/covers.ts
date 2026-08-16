/**
 * Images de couverture — objets `media.assets` et rattachements
 * `media.attachments`, plus la reconstitution de `media.attached_image()`.
 *
 * POURQUOI CE FICHIER EXISTE. Les activités de la programmation portent une
 * image, et le modèle le permet : `programme.proposals` déclarait son rôle
 * `cover` depuis l'origine, `programme.sessions` l'a reçu le 16/08. La vue
 * `v_public_schedule` expose désormais la couverture résolue. Ces mocks
 * reproduisent cette chaîne de bout en bout.
 *
 * LE REPLI EST LA RÈGLE. Une organisation joint son image AU DÉPÔT ; personne ne
 * revient en téléverser une seconde après l'acceptation. La base résout donc :
 * couverture de la séance, à défaut celle de la proposition d'origine. Les
 * données ci-dessous couvrent volontairement les TROIS cas :
 *   · une séance qui a sa propre couverture, différente de celle du dossier ;
 *   · des séances sans couverture propre, qui héritent de celle du dossier ;
 *   · des séances sans aucune image — le cas le plus fréquent, et celui qui doit
 *     rester présentable.
 *
 * LES VISUELS SONT DES ESPACES RÉSERVÉS, servis depuis `public/mocks/covers/`.
 * Ce sont des aplats de charte avec une trame géométrique : ni dégradé, ni forme
 * organique, ni texte — la direction artistique les écarte, et un visuel neutre
 * ne détourne pas l'attention du composant qu'on éprouve. Les URL pointent vers
 * le serveur de développement plutôt que vers le stockage objet, qui ne répond
 * pas en local ; en production, l'API compose l'adresse par `media.object_url()`.
 *
 * LE TEXTE ALTERNATIF EST OBLIGATOIRE, et pas par politesse :
 * `ck_assets_alt_text_required` interdit à une image d'atteindre l'état `ready`
 * sans lui, et seul `ready` est servi. Chaque image ci-dessous en porte un, en
 * français et en anglais.
 */

import type { Asset, AttachedImage, Attachment } from '~/types/media'
import type { I18nText, Uuid } from '~/types/shared'
import { ASSET, ORG, PERSON, PROPOSAL, SESSION } from './ids'

/** Rattachements média : `media.attachments`, numérotés à part des objets. */
const ATTACHMENT = (n: number): Uuid => `0198c1a0-0000-7044-8000-${String(n).padStart(12, '0')}`

interface CoverFields {
  /** Nom du fichier dans `public/mocks/covers/`, sans extension. */
  slug: string
  alt: I18nText
  credit: string
  owner: string
  organization: string
  createdAt: string
}

/**
 * Un objet image prêt à servir. `status: 'ready'` et `scan_verdict: 'clean'` :
 * ce sont les deux conditions que la base impose avant qu'une image soit rendue.
 */
function cover_asset(n: number, fields: CoverFields): Asset {
  return {
    id: ASSET(n),
    bucket: 'epavillon',
    object_key: `2026/couvertures/${fields.slug}.svg`,
    checksum_sha256: `sha256-couverture-${String(n).padStart(4, '0')}`,
    mime_type: 'image/svg+xml',
    byte_size: 4_200,
    original_filename: `${fields.slug}.svg`,
    width: 1280,
    height: 720,
    duration_seconds: null,
    owner_person_id: fields.owner,
    owner_organization_id: fields.organization,
    // Une couverture de programmation est vue sans être connecté.
    visibility: 'public',
    status: 'ready',
    scan_verdict: 'clean',
    scan_engine: 'clamav',
    scanned_at: fields.createdAt,
    scan_details: null,
    alt_text: fields.alt,
    caption: null,
    credit: fields.credit,
    license_code: 'ifdd_internal',
    deleted_at: null,
    deleted_by: null,
    purge_after: null,
    purged_at: null,
    created_at: fields.createdAt,
    updated_at: fields.createdAt,
  }
}

export const coverAssets: Asset[] = [
  cover_asset(101, {
    slug: 'adaptation-cotiere',
    alt: {
      fr: "Digue de protection et cordon dunaire sur le littoral de Saint-Louis, au Sénégal",
      en: 'Protective dyke and dune ridge along the coastline of Saint-Louis, Senegal',
    },
    credit: 'ROAC',
    owner: PERSON.sowFall,
    organization: ORG.roac,
    createdAt: '2026-06-04T11:30:00Z',
  }),
  cover_asset(102, {
    slug: 'finance-climatique',
    alt: {
      fr: "Séance de travail du comité de financement climatique, vue de la salle",
      en: 'Working session of the climate finance committee, view of the room',
    },
    credit: 'IFDD',
    owner: PERSON.kabore,
    organization: ORG.osed,
    createdAt: '2026-06-13T09:50:00Z',
  }),
  cover_asset(103, {
    slug: 'pastoralisme-sahel',
    alt: {
      fr: "Troupeau de zébus sur un parcours pastoral du Sahel burkinabè en saison sèche",
      en: 'Herd of zebu on a pastoral route in the Burkinabè Sahel during the dry season',
    },
    credit: 'OSED',
    owner: PERSON.kabore,
    organization: ORG.osed,
    createdAt: '2026-06-15T14:10:00Z',
  }),
  cover_asset(104, {
    slug: 'mangroves',
    alt: {
      fr: "Plants de palétuviers en pépinière avant replantation dans le delta du Saloum",
      en: 'Mangrove seedlings in a nursery before replanting in the Saloum delta',
    },
    credit: 'ROAC',
    owner: PERSON.sowFall,
    organization: ORG.roac,
    createdAt: '2026-06-18T08:20:00Z',
  }),
  cover_asset(105, {
    slug: 'jeunesse-francophone',
    alt: {
      fr: "Jeunes délégués réunis en cercle lors d'un atelier de préparation à la COP",
      en: 'Young delegates gathered in a circle during a COP preparation workshop',
    },
    credit: 'IFDD',
    owner: PERSON.josephPierre,
    organization: ORG.fhrc,
    createdAt: '2026-06-20T16:00:00Z',
  }),
  cover_asset(106, {
    slug: 'article-six',
    alt: {
      fr: "Tableau de suivi des transferts de résultats d'atténuation, affiché en salle de négociation",
      en: 'Tracking board for mitigation outcome transfers, displayed in a negotiation room',
    },
    credit: 'IFDD',
    owner: PERSON.josephPierre,
    organization: ORG.fhrc,
    createdAt: '2026-06-22T10:45:00Z',
  }),
]

/**
 * Rattachements. Trois cas de figure, choisis pour éprouver le repli de la vue :
 *
 *   · `mangroves` porte une couverture SUR LA SÉANCE, différente de celle de son
 *     dossier : c'est le cas où l'IFDD remplace le visuel à la publication.
 *   · les autres portent la couverture sur la PROPOSITION seulement : la séance
 *     en hérite par le `COALESCE` de `v_public_schedule`.
 *   · les vingt-quatre autres séances n'ont aucune image, et c'est voulu.
 */
export const coverAttachments: Attachment[] = [
  attachment(1, 'programme', 'proposals', PROPOSAL.adaptationCotiere, ASSET(101)),
  attachment(2, 'programme', 'proposals', PROPOSAL.accesFondsVert, ASSET(102)),
  attachment(3, 'programme', 'proposals', PROPOSAL.pastoralisme, ASSET(103)),
  attachment(4, 'programme', 'proposals', PROPOSAL.mangroves, ASSET(104)),
  attachment(5, 'programme', 'proposals', PROPOSAL.releveJeunesse, ASSET(105)),
  attachment(6, 'programme', 'proposals', PROPOSAL.article6, ASSET(106)),
  // La séance « mangroves » est publiée avec un autre visuel que son dossier :
  // elle emprunte celui de la journée jeunesse, à laquelle elle est rattachée.
  attachment(7, 'programme', 'sessions', SESSION.mangroves, ASSET(105)),
]

function attachment(
  n: number,
  ownerSchema: string,
  ownerTable: string,
  ownerId: Uuid,
  assetId: Uuid,
): Attachment {
  return {
    id: ATTACHMENT(n),
    owner_schema: ownerSchema,
    owner_table: ownerTable,
    owner_id: ownerId,
    asset_id: assetId,
    role: 'cover',
    sort_order: 0,
    alt_text_override: null,
    // Le trigger le pose : le rôle `cover` n'accepte qu'un objet.
    is_exclusive: true,
    created_by: null,
    created_at: '2026-06-25T09:00:00Z',
  }
}

const assetById = new Map(coverAssets.map((asset) => [asset.id, asset]))

/**
 * Reconstitution de `media.attached_image(schéma, table, id, rôle)`.
 *
 * Reproduit fidèlement ce que fait la fonction SQL, y compris ses filtres : seul
 * un objet `ready` et non supprimé est servi, la surcharge du rattachement prime
 * sur le texte de l'objet, et l'`url` porte toujours l'original — les variantes
 * sont générées par le worker, qui n'a rien produit ici.
 */
export function attachedImage(
  ownerSchema: string,
  ownerTable: string,
  ownerId: Uuid | null | undefined,
  role = 'cover',
): AttachedImage | null {
  if (!ownerId) return null

  const link = coverAttachments.find(
    (candidate) =>
      candidate.owner_schema === ownerSchema &&
      candidate.owner_table === ownerTable &&
      candidate.owner_id === ownerId &&
      candidate.role === role,
  )
  if (!link) return null

  const asset = assetById.get(link.asset_id)
  if (!asset || asset.deleted_at !== null || asset.status !== 'ready') return null

  return {
    asset_id: asset.id,
    // En local, le stockage objet ne répond pas : on sert les espaces réservés
    // du serveur de développement. En production, l'adresse est composée en base.
    url: `/mocks/covers/${asset.object_key.split('/').pop()}`,
    width: asset.width,
    height: asset.height,
    alt_text: link.alt_text_override ?? asset.alt_text ?? { fr: '' },
    caption: asset.caption,
    credit: asset.credit,
    // Aucune variante : le worker n'a pas tourné. Le composant se rabat sur `url`.
    sources: {},
  }
}

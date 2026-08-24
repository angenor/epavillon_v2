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
 *
 * AJOUT DU PROMPT A15 — LES MÉDIAS DE LA VITRINE. Le module `content` déclare
 * trois rôles pour `content.highlights` (`banner`, `video`, `cover`) et le rôle
 * `video` a été ajouté à `media.attachment_role` pour lui. Ces objets vivent
 * dans `public/mocks/showcase/`, y compris UNE VRAIE BOUCLE `.mp4` : le bandeau
 * d'accueil décide de rendre `<video>` ou `<img>` d'après le média reçu, et une
 * vidéo qu'on ne pourrait pas lire ne prouverait rien. Voir la section « Les
 * médias de la vitrine » plus bas.
 */

import type { Asset, AssetStatus, AttachedImage, Attachment } from '~/types/media'
import type { AttachmentRole } from '~/types/media'
import type { HighlightMediaRole } from '~/types/content'
import type { I18nText, Uuid } from '~/types/shared'
import {
  ASSET,
  EVENT,
  HIGHLIGHT,
  ORG,
  PERSON,
  PROPOSAL,
  SESSION,
  SHOWCASE_ASSET,
  SHOWCASE_ATTACHMENT,
} from './ids'

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
  /** Dimensions du fichier. 16:9 par défaut — les bannières sont plus larges. */
  width?: number
  height?: number
  /** Poids réel du fichier servi depuis `public/mocks/covers/`. */
  bytes?: number
}

/**
 * Un objet image prêt à servir. `status: 'ready'` et `scan_verdict: 'clean'` :
 * ce sont les deux conditions que la base impose avant qu'une image soit rendue.
 */
function cover_asset(n: number, fields: CoverFields): Asset {
  return {
    id: ASSET(n),
    bucket: 'epavillon',
    object_key: `2026/couvertures/${fields.slug}.jpg`,
    checksum_sha256: `sha256-couverture-${String(n).padStart(4, '0')}`,
    mime_type: 'image/jpeg',
    byte_size: fields.bytes ?? 120_000,
    original_filename: `${fields.slug}.jpg`,
    width: fields.width ?? 1280,
    height: fields.height ?? 720,
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
    // En local le stockage objet ne répond pas : l'adresse pointe vers
    // `public/mocks/`. En production, `media.object_url()` la compose en base.
    url: `/mocks/covers/${fields.slug}.jpg`,
    sources: {},
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
    bytes: 107_180,
    width: 1280,
    height: 624,
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
    bytes: 73_609,
    width: 1280,
    height: 854,
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
    bytes: 111_585,
    width: 1280,
    height: 854,
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
    bytes: 150_948,
    width: 1280,
    height: 478,
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
    bytes: 112_008,
    width: 1280,
    height: 854,
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
    bytes: 128_283,
    width: 1280,
    height: 720,
  }),

  // --- Les trois déclinaisons d'une édition (19/08) ------------------------
  // TROIS RECADRAGES PAR ÉDITION, ET NON UN FICHIER REDIMENSIONNÉ TROIS FOIS.
  // `media.attachable_roles` déclare la forme exigée par chaque rôle et le
  // trigger la vérifie : un 16:9 posé en `banner` est REFUSÉ, pas rogné. Les
  // dimensions ci-dessous sont donc exactes, relevées par `ffprobe` sur les
  // fichiers réels — arrondir une hauteur ferait passer ici un jeu simulé que
  // la base rejetterait.
  cover_asset(201, {
    slug: 'pavillon-cop31-32-9',
    alt: {
      fr: "Vue du pavillon de la Francophonie : estrade, écran de projection et rangées de sièges — bandeau panoramique 32:9",
      en: 'View of the Francophonie pavilion: stage, projection screen and rows of seats — panoramic banner 32:9',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-07-28T09:00:00Z',
    width: 1920,
    height: 540,
    bytes: 99_494,
  }),
  cover_asset(202, {
    slug: 'pavillon-cop31-16-9',
    alt: {
      fr: "Vue du pavillon de la Francophonie : estrade, écran de projection et rangées de sièges — couverture 16:9",
      en: 'View of the Francophonie pavilion: stage, projection screen and rows of seats — cover 16:9',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-07-28T09:00:00Z',
    width: 1280,
    height: 720,
    bytes: 123_679,
  }),
  cover_asset(203, {
    slug: 'pavillon-cop31-1-1',
    alt: {
      fr: "Vue du pavillon de la Francophonie : estrade, écran de projection et rangées de sièges — vignette carrée 1:1",
      en: 'View of the Francophonie pavilion: stage, projection screen and rows of seats — square thumbnail 1:1',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-07-28T09:00:00Z',
    width: 800,
    height: 800,
    bytes: 90_249,
  }),
  cover_asset(204, {
    slug: 'pavillon-cop30-32-9',
    alt: {
      fr: "Salle du pavillon de la Francophonie pendant une séance plénière, à Belém — bandeau panoramique 32:9",
      en: 'Francophonie pavilion room during a plenary session in Belém — panoramic banner 32:9',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2025-10-02T09:00:00Z',
    width: 1920,
    height: 540,
    bytes: 103_775,
  }),
  cover_asset(205, {
    slug: 'pavillon-cop30-16-9',
    alt: {
      fr: "Salle du pavillon de la Francophonie pendant une séance plénière, à Belém — couverture 16:9",
      en: 'Francophonie pavilion room during a plenary session in Belém — cover 16:9',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2025-10-02T09:00:00Z',
    width: 1280,
    height: 720,
    bytes: 101_752,
  }),
  cover_asset(206, {
    slug: 'pavillon-cop30-1-1',
    alt: {
      fr: "Salle du pavillon de la Francophonie pendant une séance plénière, à Belém — vignette carrée 1:1",
      en: 'Francophonie pavilion room during a plenary session in Belém — square thumbnail 1:1',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2025-10-02T09:00:00Z',
    width: 800,
    height: 800,
    bytes: 64_986,
  }),
  cover_asset(207, {
    slug: 'cycle-paco-32-9',
    alt: {
      fr: "Grille de vignettes de participants lors d'un webinaire du cycle PACO — bandeau panoramique 32:9",
      en: 'Grid of participant thumbnails during a PACO series webinar — panoramic banner 32:9',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-01-16T09:00:00Z',
    width: 1920,
    height: 540,
    bytes: 138_115,
  }),
  cover_asset(208, {
    slug: 'cycle-paco-16-9',
    alt: {
      fr: "Grille de vignettes de participants lors d'un webinaire du cycle PACO — couverture 16:9",
      en: 'Grid of participant thumbnails during a PACO series webinar — cover 16:9',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-01-16T09:00:00Z',
    width: 1280,
    height: 720,
    bytes: 163_087,
  }),
  cover_asset(209, {
    slug: 'cycle-paco-1-1',
    alt: {
      fr: "Grille de vignettes de participants lors d'un webinaire du cycle PACO — vignette carrée 1:1",
      en: 'Grid of participant thumbnails during a PACO series webinar — square thumbnail 1:1',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-01-16T09:00:00Z',
    width: 800,
    height: 800,
    bytes: 111_369,
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

  // LES TROIS DÉCLINAISONS, POUR TROIS ÉDITIONS SUR CINQ. La COP29 et PACO 2027
  // n'en ont AUCUNE, et c'est délibéré : le bandeau de la fiche, la carte de
  // l'accueil et la vignette de liste doivent rester entiers sans visuel, et
  // c'est le seul moyen de le vérifier à chaque rendu plutôt qu'à la mise en
  // ligne.
  attachment(8, 'event', 'events', EVENT.cop31, ASSET(201), 'banner'),
  attachment(9, 'event', 'events', EVENT.cop31, ASSET(202), 'cover'),
  attachment(10, 'event', 'events', EVENT.cop31, ASSET(203), 'thumbnail'),
  attachment(11, 'event', 'events', EVENT.cop30, ASSET(204), 'banner'),
  attachment(12, 'event', 'events', EVENT.cop30, ASSET(205), 'cover'),
  attachment(13, 'event', 'events', EVENT.cop30, ASSET(206), 'thumbnail'),
  attachment(14, 'event', 'events', EVENT.paco2026, ASSET(207), 'banner'),
  attachment(15, 'event', 'events', EVENT.paco2026, ASSET(208), 'cover'),
  attachment(16, 'event', 'events', EVENT.paco2026, ASSET(209), 'thumbnail'),
]

function attachment(
  n: number,
  ownerSchema: string,
  ownerTable: string,
  ownerId: Uuid,
  assetId: Uuid,
  role: AttachmentRole = 'cover',
): Attachment {
  return {
    id: ATTACHMENT(n),
    owner_schema: ownerSchema,
    owner_table: ownerTable,
    owner_id: ownerId,
    asset_id: assetId,
    role,
    sort_order: 0,
    alt_text_override: null,
    // Le trigger le pose : le rôle `cover` n'accepte qu'un objet.
    is_exclusive: true,
    created_by: null,
    created_at: '2026-06-25T09:00:00Z',
  }
}

// ---------------------------------------------------------------------------
// LES MÉDIAS DE LA VITRINE (A15) — fonds, vignettes et fond vidéo
//
// `content.highlights` déclare TROIS rôles dans `media.attachable_roles`
// (`115_content.sql` § 5) : `banner` (fond photographique, 15 Mio), `video`
// (fond vidéo, 200 Mio) et `cover` (vignette du rail, 5 Mio — elle sert aussi
// d'affiche à la vidéo). La v1 stockait `photo_url`, `video_url` et
// `thumbnail_url` en texte libre, et `thumbnail_url` était presque toujours nul.
//
// DEUX CAS DE VIDÉO, ET C'EST DÉLIBÉRÉ :
//   · `parole-negociateur` porte une boucle PRÊTE — le bandeau rend `<video>` ;
//   · `innovation-mesure-carbone` porte une boucle ENCORE EN TRAITEMENT. Elle
//     n'est pas servie (seul `ready` l'est), `background_video` sort donc nul, et
//     le bandeau se rabat sur `background_image`. Ce n'est pas un cas d'erreur :
//     c'est ce qui se passe pendant les minutes qui suivent tout téléversement,
//     et le front doit le tenir sans trou.
//
// Deux diapositives n'ont AUCUN média — `chiffre-cle-pavillon` et l'épingle
// « organisations inscrites » : elles n'ont que `background_color_hex`. Le
// bandeau doit rester lisible sur un aplat, c'est le dernier repli.
// ---------------------------------------------------------------------------

interface ShowcaseAssetFields extends CoverFields {
  /** Extension du fichier servi depuis `public/mocks/showcase/`. */
  ext: 'jpg' | 'mp4'
  mime: string
  bytes: number
  /** Seul `ready` est servi. `processing` reproduit l'attente du worker. */
  status?: AssetStatus
  /** En TEXTE, comme l'API la fait traverser : un flottant perdrait les
   *  millisecondes sans le dire. */
  durationSeconds?: string
}

/** Un objet de la vitrine. Même fabrique que les couvertures, autre dossier. */
function showcase_asset(n: number, fields: ShowcaseAssetFields): Asset {
  const status = fields.status ?? 'ready'
  return {
    id: SHOWCASE_ASSET(n),
    bucket: 'epavillon',
    object_key: `2026/vitrine/${fields.slug}.${fields.ext}`,
    checksum_sha256: `sha256-vitrine-${String(n).padStart(4, '0')}`,
    mime_type: fields.mime,
    byte_size: fields.bytes,
    original_filename: `${fields.slug}.${fields.ext}`,
    width: fields.width ?? 1920,
    height: fields.height ?? 1080,
    duration_seconds: fields.durationSeconds ?? null,
    owner_person_id: fields.owner,
    owner_organization_id: fields.organization,
    visibility: 'public',
    status,
    // Un objet encore en traitement n'a pas fini d'être analysé : dire
    // « clean » alors qu'il n'est pas `ready` inventerait un verdict.
    scan_verdict: status === 'ready' ? 'clean' : 'pending',
    scan_engine: status === 'ready' ? 'clamav' : null,
    scanned_at: status === 'ready' ? fields.createdAt : null,
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
    url: `/mocks/showcase/${fields.slug}.${fields.ext}`,
    sources: {},
  }
}

export const showcaseAssets: Asset[] = [
  // --- Fonds photographiques (rôle `banner`) -------------------------------
  showcase_asset(1, {
    slug: 'temoignage-koivogui',
    alt: {
      fr: "Biligua Koivogui lors d'une session de formation à la négociation climatique",
      en: 'Biligua Koivogui during a climate negotiation training session',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2025-11-24T10:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 266_775,
    width: 1920,
    height: 1280,
  }),
  showcase_asset(2, {
    slug: 'temoignage-genevee',
    alt: {
      fr: "Constance Genevée au pavillon de la Francophonie pendant la CdP30",
      en: 'Constance Genevée at the Francophonie pavilion during COP30',
    },
    credit: 'IFDD',
    owner: PERSON.ngoBassong,
    organization: ORG.cofemac,
    createdAt: '2026-07-14T09:20:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 144_119,
    width: 1920,
    height: 1484,
  }),
  showcase_asset(3, {
    slug: 'temoignage-faye',
    alt: {
      fr: "Antoine Faye au pavillon de la Francophonie, entre deux séances de négociation",
      en: 'Antoine Faye at the Francophonie pavilion, between two negotiation sessions',
    },
    credit: 'IFDD',
    owner: PERSON.zinsou,
    organization: ORG.anteb,
    createdAt: '2026-07-22T15:40:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 108_423,
    width: 1920,
    height: 1080,
  }),
  showcase_asset(4, {
    slug: 'bilan-carbone',
    alt: {
      fr: "Relevé de terrain pour un bilan carbone, carnet et instrument de mesure",
      en: 'Field survey for a carbon assessment, notebook and measuring instrument',
    },
    credit: 'IFDD',
    owner: PERSON.moreau,
    organization: ORG.verdeo,
    createdAt: '2026-08-04T08:10:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 153_577,
    width: 1920,
    height: 1280,
  }),
  showcase_asset(5, {
    slug: 'formation-negociateurs',
    alt: {
      fr: "Session de formation des négociateurs francophones, vue de la salle",
      en: 'Training session for Francophone negotiators, view of the room',
    },
    credit: 'IFDD',
    owner: PERSON.kabore,
    organization: ORG.osed,
    createdAt: '2026-07-30T11:05:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 213_898,
    width: 1920,
    height: 1280,
  }),
  showcase_asset(6, {
    slug: 'cop-pavillon',
    alt: {
      fr: "Le pavillon de la Francophonie sur le site d'une Conférence des Parties",
      en: 'The Francophonie pavilion on the site of a Conference of the Parties',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-08-05T13:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 303_771,
    width: 1920,
    height: 1440,
  }),
  showcase_asset(7, {
    slug: 'temoignage-tirouvi',
    alt: {
      fr: "Kaully Tirouvi lors d'une simulation de négociation",
      en: 'Kaully Tirouvi during a negotiation simulation',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2024-11-25T10:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 290_523,
    width: 1920,
    height: 1440,
  }),
  showcase_asset(8, {
    slug: 'paco-banniere',
    alt: {
      fr: "Bannière du cycle de webinaires PACO — préparation à l'action climatique",
      en: 'Banner of the PACO webinar series — preparing for climate action',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-04-02T12:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 388_556,
    width: 1920,
    height: 1280,
  }),

  // --- Vignettes du rail (rôle `cover`, 640 × 360) -------------------------
  showcase_asset(21, {
    slug: 'temoignage-koivogui-vignette',
    alt: {
      fr: "Vignette : Biligua Koivogui en formation",
      en: 'Thumbnail: Biligua Koivogui in training',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2025-11-24T10:05:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 32_762,
    width: 480,
    height: 320,
  }),
  showcase_asset(22, {
    slug: 'temoignage-genevee-vignette',
    alt: {
      fr: "Vignette : Constance Genevée au pavillon",
      en: 'Thumbnail: Constance Genevée at the pavilion',
    },
    credit: 'IFDD',
    owner: PERSON.ngoBassong,
    organization: ORG.cofemac,
    createdAt: '2026-07-14T09:25:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 17_969,
    width: 480,
    height: 372,
  }),
  showcase_asset(23, {
    slug: 'pavillon-boucle-vignette',
    alt: {
      fr: "Vignette : le pavillon de la Francophonie en activité",
      en: 'Thumbnail: the Francophonie pavilion in operation',
    },
    credit: 'IFDD',
    owner: PERSON.zinsou,
    organization: ORG.anteb,
    createdAt: '2026-07-22T15:45:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 18_420,
    width: 480,
    height: 270,
  }),
  showcase_asset(24, {
    slug: 'bilan-carbone-vignette',
    alt: {
      fr: "Vignette : relevé de terrain pour un bilan carbone",
      en: 'Thumbnail: field survey for a carbon assessment',
    },
    credit: 'IFDD',
    owner: PERSON.moreau,
    organization: ORG.verdeo,
    createdAt: '2026-08-04T08:15:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 18_240,
    width: 480,
    height: 320,
  }),
  showcase_asset(26, {
    slug: 'cop-pavillon-vignette',
    alt: {
      fr: "Vignette : le pavillon sur le site d'une CdP",
      en: 'Thumbnail: the pavilion on a COP site',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-08-05T13:05:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 26_138,
    width: 480,
    height: 360,
  }),
  showcase_asset(28, {
    slug: 'paco-banniere-vignette',
    alt: {
      fr: "Vignette : bannière du cycle PACO",
      en: 'Thumbnail: PACO series banner',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-04-02T12:05:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 24_491,
    width: 480,
    height: 320,
  }),
  showcase_asset(30, {
    slug: 'pavillon-accueil-vignette',
    alt: {
      fr: "Vignette : entrée du pavillon de la Francophonie",
      en: 'Thumbnail: entrance to the Francophonie pavilion',
    },
    credit: 'IFDD',
    owner: PERSON.bakayoko,
    organization: ORG.ifdd,
    createdAt: '2026-08-06T09:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 23_841,
    width: 480,
    height: 270,
  }),
  showcase_asset(31, {
    slug: 'paco-session-vignette',
    alt: {
      fr: "Vignette : séance du cycle de webinaires PACO",
      en: 'Thumbnail: PACO webinar session',
    },
    credit: 'IFDD',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-07-09T10:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 12_063,
    width: 480,
    height: 320,
  }),
  showcase_asset(32, {
    slug: 'seminaire-chypre-vignette',
    alt: {
      fr: "Vignette : séminaire de Larnaka et Nicosie, Chypre",
      en: 'Thumbnail: Larnaca and Nicosia seminar, Cyprus',
    },
    credit: 'IFDD',
    owner: PERSON.nkoDiop,
    organization: ORG.ifdd,
    createdAt: '2026-08-12T14:00:00Z',
    ext: 'jpg',
    mime: 'image/jpeg',
    bytes: 24_671,
    width: 480,
    height: 180,
  }),

  // --- Fonds vidéo (rôle `video`) ------------------------------------------
  showcase_asset(41, {
    slug: 'pavillon-boucle',
    alt: {
      fr: "Boucle muette : le pavillon de la Francophonie en activité",
      en: 'Silent loop: the Francophonie pavilion in operation',
    },
    credit: 'IFDD',
    owner: PERSON.zinsou,
    organization: ORG.anteb,
    createdAt: '2026-07-22T15:50:00Z',
    ext: 'mp4',
    mime: 'video/mp4',
    bytes: 483_577,
    width: 1280,
    height: 720,
    durationSeconds: '14.000',
  }),
  showcase_asset(42, {
    // ENCORE EN TRAITEMENT, et c'est sa raison d'être : la vue ne la sert pas,
    // le bandeau se rabat sur l'image de fond. Rien n'est cassé, rien n'est
    // vide — c'est l'état normal des minutes qui suivent un téléversement.
    slug: 'bilan-carbone-boucle',
    alt: {
      fr: "Boucle muette : relevé de terrain filmé au ras du sol",
      en: 'Silent loop: field survey filmed at ground level',
    },
    credit: 'Verdeo Solutions',
    owner: PERSON.moreau,
    organization: ORG.verdeo,
    createdAt: '2026-08-18T17:30:00Z',
    ext: 'mp4',
    mime: 'video/mp4',
    bytes: 24_800_000,
    width: 1920,
    height: 1080,
    durationSeconds: '18.000',
    status: 'processing',
  }),
]


/**
 * Rattachements de la vitrine. Ce que chaque diapositive porte, et surtout ce
 * qu'elle NE PORTE PAS :
 *
 *   · `bonnePratiquePastoralisme` n'a pas de vignette — le rail se rabat sur le
 *     fond photographique ;
 *   · `chiffreClePavillon` et `asideChiffreOrganisations` n'ont aucun média —
 *     l'aplat `background_color_hex` est le seul fond ;
 *   · `temoignageArchiveCop29` et `annonceWebinairePaco` en portent, bien
 *     qu'elles ne sortent jamais : une diapositive archivée reste complète, et
 *     c'est ce qui permet de la remettre en avant l'année suivante.
 */
export const showcaseAttachments: Attachment[] = [
  showcaseAttachment(1, HIGHLIGHT.temoignageNegociatrice, SHOWCASE_ASSET(1), 'banner'),
  showcaseAttachment(2, HIGHLIGHT.temoignageNegociatrice, SHOWCASE_ASSET(21), 'cover'),
  showcaseAttachment(3, HIGHLIGHT.temoignageCooperatives, SHOWCASE_ASSET(2), 'banner'),
  showcaseAttachment(4, HIGHLIGHT.temoignageCooperatives, SHOWCASE_ASSET(22), 'cover'),
  showcaseAttachment(5, HIGHLIGHT.paroleNegociateur, SHOWCASE_ASSET(3), 'banner'),
  showcaseAttachment(6, HIGHLIGHT.paroleNegociateur, SHOWCASE_ASSET(23), 'cover'),
  showcaseAttachment(7, HIGHLIGHT.paroleNegociateur, SHOWCASE_ASSET(41), 'video'),
  showcaseAttachment(8, HIGHLIGHT.innovationMesureCarbone, SHOWCASE_ASSET(4), 'banner'),
  showcaseAttachment(9, HIGHLIGHT.innovationMesureCarbone, SHOWCASE_ASSET(24), 'cover'),
  // Objet `processing` : rattaché, donc visible au back-office, mais NON SERVI.
  showcaseAttachment(10, HIGHLIGHT.innovationMesureCarbone, SHOWCASE_ASSET(42), 'video'),
  showcaseAttachment(11, HIGHLIGHT.bonnePratiquePastoralisme, SHOWCASE_ASSET(5), 'banner'),
  showcaseAttachment(12, HIGHLIGHT.annonceJourneeJeunesse, SHOWCASE_ASSET(6), 'banner'),
  showcaseAttachment(13, HIGHLIGHT.annonceJourneeJeunesse, SHOWCASE_ASSET(26), 'cover'),
  showcaseAttachment(14, HIGHLIGHT.temoignageArchiveCop29, SHOWCASE_ASSET(7), 'banner'),
  showcaseAttachment(15, HIGHLIGHT.annonceWebinairePaco, SHOWCASE_ASSET(8), 'banner'),
  showcaseAttachment(16, HIGHLIGHT.annonceWebinairePaco, SHOWCASE_ASSET(28), 'cover'),
  showcaseAttachment(17, HIGHLIGHT.asideAppelCop31, SHOWCASE_ASSET(30), 'cover'),
  showcaseAttachment(18, HIGHLIGHT.asideRediffusionsPaco, SHOWCASE_ASSET(31), 'cover'),
  showcaseAttachment(19, HIGHLIGHT.asideGuidePavillon, SHOWCASE_ASSET(32), 'cover'),
]

function showcaseAttachment(
  n: number,
  highlightId: Uuid,
  assetId: Uuid,
  role: HighlightMediaRole,
): Attachment {
  return {
    id: SHOWCASE_ATTACHMENT(n),
    owner_schema: 'content',
    owner_table: 'highlights',
    owner_id: highlightId,
    asset_id: assetId,
    role,
    sort_order: 0,
    alt_text_override: null,
    // Les trois rôles de `content.highlights` sont exclusifs (`is_multiple`
    // faux) : un seul objet par rôle, l'index unique partiel l'impose.
    is_exclusive: true,
    created_by: null,
    created_at: '2026-08-06T09:00:00Z',
  }
}

const assetById = new Map(
  [...coverAssets, ...showcaseAssets].map((asset) => [asset.id, asset]),
)

/**
 * L'adresse servie par le serveur de développement, déduite du deuxième segment
 * de `object_key` : `2026/couvertures/…` → `/mocks/covers/…`, `2026/vitrine/…`
 * → `/mocks/showcase/…`. En production, l'adresse est composée EN BASE par
 * `media.object_url()` — aucune URL n'est stockée (ADR-08).
 */
const PUBLIC_FOLDER: Record<string, string> = {
  couvertures: 'covers',
  vitrine: 'showcase',
}

function placeholderUrl(objectKey: string): string {
  const segments = objectKey.split('/')
  const folder = PUBLIC_FOLDER[segments[1] ?? ''] ?? 'covers'
  return `/mocks/${folder}/${segments[segments.length - 1]}`
}

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

  const link = attachmentOf(ownerSchema, ownerTable, ownerId, role)
  if (!link) return null

  const asset = assetById.get(link.asset_id)
  if (!asset || asset.deleted_at !== null || asset.status !== 'ready') return null

  return {
    asset_id: asset.id,
    // En local, le stockage objet ne répond pas : on sert les espaces réservés
    // du serveur de développement. En production, l'adresse est composée en base.
    url: placeholderUrl(asset.object_key),
    width: asset.width,
    height: asset.height,
    alt_text: link.alt_text_override ?? asset.alt_text ?? { fr: '' },
    caption: asset.caption,
    credit: asset.credit,
    // Aucune variante : le worker n'a pas tourné. Le composant se rabat sur `url`.
    sources: {},
  }
}

/**
 * Le rattachement d'un rôle, PRÊT OU NON.
 *
 * `attachedImage()` n'en rend rien tant que l'objet n'est pas `ready`, et c'est
 * juste : le public ne doit voir que ce qui est servable. Le BACK-OFFICE, lui,
 * doit savoir qu'un objet existe et qu'il arrive — d'où cette lecture séparée,
 * qui alimente `ShowcaseMediaSlot.is_pending`. Sans elle, l'éditeur voit un
 * emplacement vide et téléverse une seconde fois.
 */
export function attachmentOf(
  ownerSchema: string,
  ownerTable: string,
  ownerId: Uuid | null | undefined,
  role = 'cover',
): Attachment | null {
  if (!ownerId) return null
  return (
    [...coverAttachments, ...showcaseAttachments].find(
      (candidate) =>
        candidate.owner_schema === ownerSchema &&
        candidate.owner_table === ownerTable &&
        candidate.owner_id === ownerId &&
        candidate.role === role,
    ) ?? null
  )
}

/** L'objet visé par un rattachement, quel que soit son état de traitement. */
export function assetOf(attachment: Attachment | null): Asset | null {
  return attachment ? (assetById.get(attachment.asset_id) ?? null) : null
}

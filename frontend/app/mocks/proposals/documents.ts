/**
 * Données simulées de `programme.proposal_documents` et des objets `media.assets`
 * qu'ils désignent.
 *
 * LA BASE NE STOCKE JAMAIS D'URL. Un objet est décrit par son couple
 * `(bucket, object_key)` ; l'adresse publique est composée à la lecture par
 * l'API (`media.object_url()`). Un composant reçoit donc une URL déjà
 * construite et n'en fabrique aucune — c'est ce qui permet de changer de
 * fournisseur de stockage sans toucher aux données.
 *
 * Deux pièces sont INTERNES au dossier (`is_public: false`) : le budget
 * prévisionnel et une lettre de soutien. Elles ne doivent apparaître ni sur la
 * page publique de l'activité, ni dans l'espace de l'organisation partenaire.
 *
 * Un objet est en QUARANTAINE (`scan_verdict: 'infected'`) : seul l'état `ready`
 * est servi. L'écran doit le dire au déposant plutôt que d'afficher un lien mort.
 */

import type { Asset } from '~/types/media'
import type { ProposalDocument } from '~/types/programme/proposal'
import { ASSET, ORG, PERSON, PROPOSAL, PROPOSAL_DOCUMENT } from '../ids'

// ---------------------------------------------------------------------------
// Objets stockés
// ---------------------------------------------------------------------------

interface AssetFields {
  key: string
  mime: string
  bytes: number
  filename: string
  owner: string
  organization: string
  createdAt: string
  status?: Asset['status']
  verdict?: Asset['scan_verdict']
}

function document_asset(n: number, fields: AssetFields): Asset {
  return {
    id: ASSET(n),
    bucket: 'epavillon',
    object_key: fields.key,
    checksum_sha256: `sha256-simule-${String(n).padStart(4, '0')}`,
    mime_type: fields.mime,
    byte_size: fields.bytes,
    original_filename: fields.filename,
    width: null,
    height: null,
    duration_seconds: null,
    owner_person_id: fields.owner,
    owner_organization_id: fields.organization,
    visibility: 'authenticated',
    status: fields.status ?? 'ready',
    scan_verdict: fields.verdict ?? 'clean',
    scan_engine: 'clamav',
    scanned_at: fields.createdAt,
    scan_details: null,
    // Un document n'est pas une image : `alt_text` reste nul, la contrainte
    // `ck_assets_alt_text_required` ne vise que les images servies.
    alt_text: null,
    caption: null,
    credit: null,
    license_code: 'ifdd_internal',
    deleted_at: null,
    deleted_by: null,
    purge_after: null,
    purged_at: null,
    created_at: fields.createdAt,
    updated_at: fields.createdAt,
  }
}

export const proposalAssets = [
  document_asset(1, {
    key: '2026/06/adaptation-cotiere/note-de-cadrage.pdf',
    mime: 'application/pdf',
    bytes: 842_113,
    filename: 'Note de cadrage — adaptation côtière.pdf',
    owner: PERSON.sowFall,
    organization: ORG.roac,
    createdAt: '2026-06-04T11:20:00Z',
  }),
  document_asset(2, {
    key: '2026/06/adaptation-cotiere/budget-previsionnel.xlsx',
    mime: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    bytes: 61_440,
    filename: 'Budget prévisionnel.xlsx',
    owner: PERSON.sowFall,
    organization: ORG.roac,
    createdAt: '2026-06-04T11:25:00Z',
  }),
  document_asset(3, {
    key: '2026/06/pertes-prejudices/lettre-de-soutien-ifdd.pdf',
    mime: 'application/pdf',
    bytes: 214_998,
    filename: 'Lettre de soutien IFDD.pdf',
    owner: PERSON.josephPierre,
    organization: ORG.fhrc,
    createdAt: '2026-06-12T10:05:00Z',
  }),
  document_asset(4, {
    key: '2026/06/mini-reseaux/resultats-exploitation.pdf',
    mime: 'application/pdf',
    bytes: 1_337_204,
    filename: 'Résultats d’exploitation — 40 mini-réseaux.pdf',
    owner: PERSON.kabore,
    organization: ORG.osed,
    createdAt: '2026-06-13T09:40:00Z',
  }),
  document_asset(5, {
    key: '2026/07/article-6/note-technique-autorisations.pdf',
    mime: 'application/pdf',
    bytes: 553_002,
    filename: 'Note technique — autorisations article 6.pdf',
    owner: PERSON.lemoine,
    organization: ORG.cudcm,
    createdAt: '2026-07-01T16:10:00Z',
  }),
  document_asset(6, {
    key: '2026/07/rapports-biennaux/trame-atelier.pptx',
    mime: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    bytes: 4_119_552,
    filename: 'Trame de l’atelier.pptx',
    owner: PERSON.tremblay,
    organization: ORG.ifdd,
    createdAt: '2026-06-21T08:30:00Z',
  }),
  document_asset(7, {
    // EN QUARANTAINE : le fichier a été refusé par l'analyse antivirus. L'écran
    // doit l'annoncer au déposant, pas proposer un lien.
    key: '2026/08/dechets-plastiques/annexe-photos.zip',
    mime: 'application/zip',
    bytes: 18_446_294,
    filename: 'Annexe photos.zip',
    owner: PERSON.koffi,
    organization: ORG.ujfc,
    createdAt: '2026-07-21T12:00:00Z',
    status: 'quarantined',
    verdict: 'infected',
  }),
] satisfies Asset[]

// ---------------------------------------------------------------------------
// Pièces jointes aux dossiers
// ---------------------------------------------------------------------------

export const proposalDocuments = [
  {
    id: PROPOSAL_DOCUMENT(1),
    proposal_id: PROPOSAL.adaptationCotiere,
    asset_id: ASSET(1),
    title: { fr: 'Note de cadrage', en: 'Scoping note' },
    document_type_code: 'technical_note',
    is_public: true,
    uploaded_by: PERSON.sowFall,
    uploaded_at: '2026-06-04T11:20:00Z',
    sort_order: 10,
  },
  {
    // INTERNE : le budget ne s'affiche pas sur la page publique de l'activité.
    id: PROPOSAL_DOCUMENT(2),
    proposal_id: PROPOSAL.adaptationCotiere,
    asset_id: ASSET(2),
    title: { fr: 'Budget prévisionnel' },
    document_type_code: 'relevant_document',
    is_public: false,
    uploaded_by: PERSON.sowFall,
    uploaded_at: '2026-06-04T11:25:00Z',
    sort_order: 20,
  },
  {
    id: PROPOSAL_DOCUMENT(3),
    proposal_id: PROPOSAL.pertesPrejudices,
    asset_id: ASSET(3),
    title: { fr: 'Lettre de soutien' },
    document_type_code: 'relevant_document',
    is_public: false,
    uploaded_by: PERSON.josephPierre,
    uploaded_at: '2026-06-12T10:05:00Z',
    sort_order: 10,
  },
  {
    id: PROPOSAL_DOCUMENT(4),
    proposal_id: PROPOSAL.miniReseaux,
    asset_id: ASSET(4),
    title: { fr: "Résultats d'exploitation de quarante mini-réseaux" },
    document_type_code: 'report',
    is_public: true,
    uploaded_by: PERSON.kabore,
    uploaded_at: '2026-06-13T09:40:00Z',
    sort_order: 10,
  },
  {
    id: PROPOSAL_DOCUMENT(5),
    proposal_id: PROPOSAL.article6,
    asset_id: ASSET(5),
    title: {
      fr: "Note technique sur les autorisations au titre de l'article 6",
      en: 'Technical note on Article 6 authorisations',
    },
    document_type_code: 'technical_note',
    is_public: true,
    uploaded_by: PERSON.lemoine,
    uploaded_at: '2026-07-01T16:10:00Z',
    sort_order: 10,
  },
  {
    id: PROPOSAL_DOCUMENT(6),
    proposal_id: PROPOSAL.rapportsBiennaux,
    asset_id: ASSET(6),
    title: { fr: "Trame de l'atelier" },
    document_type_code: 'presentation',
    is_public: true,
    uploaded_by: PERSON.tremblay,
    uploaded_at: '2026-06-21T08:30:00Z',
    sort_order: 10,
  },
  {
    id: PROPOSAL_DOCUMENT(7),
    proposal_id: PROPOSAL.dechetsPlastiques,
    asset_id: ASSET(7),
    title: { fr: 'Annexe photographique' },
    document_type_code: 'relevant_document',
    is_public: false,
    uploaded_by: PERSON.koffi,
    uploaded_at: '2026-07-21T12:00:00Z',
    sort_order: 10,
  },
] satisfies ProposalDocument[]

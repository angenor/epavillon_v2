/**
 * LE BACK-OFFICE DES DOCUMENTS DE GUIDE NÉGO — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-negotiations.ts`, et même règle : **portée globale,
 * sans périmètre d'édition**. L'API refuse en 403 avant toute lecture ; l'écran
 * affiche alors `UiForbiddenState`, pas une liste vide.
 *
 * DEUX PERMISSIONS SE PARTAGENT CES ROUTES. Publier ouvre tout sauf les notes ;
 * corriger ouvre la lecture, l'aperçu et les notes. Les réponses portent
 * `can_publish`, `can_correct`, `can_post` et `can_withdraw` : l'écran s'en sert
 * pour montrer les gestes, jamais pour décider à la place de l'API.
 *
 * LE PDF ET LES IMAGES NE SONT PAS DU JSON : leurs méthodes rendent le chemin
 * d'API. Le dépôt du PDF, lui, passe par la garde média (`useDepotMedia`), puis
 * par `attacherLeFichier`.
 */
import type {
  AdminCorrectionNote,
  AdminCorrectionNoteList,
  AdminDocument,
  AdminDocumentInput,
  AdminDocumentList,
  AdminDocumentPreview,
  AttachFileInput,
  CorrectionNoteInput,
  ServeAsIsInput,
} from '~/types/admin-negotiation-documents'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

type Deps = Pick<ApiTransport, 'call' | 'send'>

const exemples = () => import('~/mocks/negotiation-documents')

export function createAdminNegotiationDocumentsApi({ call, send }: Deps) {
  // Hors ligne, le jeu d'exemple résout ses titres dans la langue qu'`Accept-Language` porterait.
  const { $i18n } = useNuxtApp()
  const langue = (): string => String($i18n.locale.value)

  return {
    /** Tous les documents : état, type, version, remplacement, extraction. */
    documents: (): Promise<AdminDocumentList> =>
      call('/admin/negotiation/documents', async () => (await exemples()).listeDesDocuments(langue())),

    /** La fiche, textes non résolus, avec l'état de l'extraction. */
    document: (documentId: Uuid): Promise<AdminDocument> =>
      call(`/admin/negotiation/documents/${documentId}`, async () => (await exemples()).ficheDuDocument(documentId, langue())),

    /** Crée un **brouillon** sans source : le PDF se dépose ensuite avec lui pour propriétaire. */
    creer: (entree: AdminDocumentInput): Promise<AdminDocument> =>
      send('/admin/negotiation/documents', entree, async () => (await exemples()).creerUnDocument(entree, langue())),

    /** Un champ absent ne change rien ; `null` vide un champ facultatif. */
    modifier: (documentId: Uuid, entree: AdminDocumentInput): Promise<AdminDocument> =>
      send(
        `/admin/negotiation/documents/${documentId}`,
        entree,
        async () => (await exemples()).modifierLeDocument(documentId, entree, langue()),
        'PATCH',
      ),

    /** Un brouillon jamais publié seulement ; un document publié se dépublie. */
    supprimer: (documentId: Uuid): Promise<void> =>
      send(
        `/admin/negotiation/documents/${documentId}`,
        {},
        async () => (await exemples()).supprimerLeDocument(documentId),
        'DELETE',
      ),

    /** Attache le PDF déposé, et met son extraction en file dans la même transaction. */
    attacherLeFichier: (documentId: Uuid, assetId: Uuid): Promise<AdminDocument> => {
      const corps: AttachFileInput = { asset_id: assetId }
      return send(
        `/admin/negotiation/documents/${documentId}/file`,
        corps,
        async () => (await exemples()).attacherLeFichier(documentId, assetId, langue()),
        'PUT',
      )
    },

    /** Le chemin du PDF d'origine, lu dans le bucket privé. */
    pdf: (documentId: Uuid): string => `/admin/negotiation/documents/${documentId}/file`,

    relancerLExtraction: (documentId: Uuid): Promise<void> =>
      send(`/admin/negotiation/documents/${documentId}/extraction`, {}, async () =>
        (await exemples()).relancerLExtraction(documentId),
      ),

    /** « Ouvrir tel quel » : se change sans republier. */
    ouvrirTelQuel: (documentId: Uuid, telQuel: boolean): Promise<AdminDocument> => {
      const corps: ServeAsIsInput = { serve_as_is: telQuel }
      return send(
        `/admin/negotiation/documents/${documentId}/as-is`,
        corps,
        async () => (await exemples()).ouvrirTelQuel(documentId, telQuel, langue()),
        'PUT',
      )
    },

    /** Exige une source, et pour un fichier une extraction prête. Rejouer ne change rien. */
    publier: (documentId: Uuid): Promise<AdminDocument> =>
      send(`/admin/negotiation/documents/${documentId}/publish`, {}, async () =>
        (await exemples()).publierLeDocument(documentId, langue()),
      ),

    depublier: (documentId: Uuid): Promise<AdminDocument> =>
      send(`/admin/negotiation/documents/${documentId}/unpublish`, {}, async () =>
        (await exemples()).depublierLeDocument(documentId, langue()),
      ),

    /** Le brouillon prérempli qui remplacera celui-ci ; sa version reste à saisir. */
    nouvelleVersion: (documentId: Uuid): Promise<AdminDocument> =>
      send(`/admin/negotiation/documents/${documentId}/new-version`, {}, async () =>
        (await exemples()).nouvelleVersion(documentId, langue()),
      ),

    /** Le verdict, les indicateurs, le sommaire, et chaque page : ses blocs et son image. */
    apercu: (documentId: Uuid): Promise<AdminDocumentPreview> =>
      call(`/admin/negotiation/documents/${documentId}/preview`, async () =>
        (await exemples()).apercuDuDocument(documentId),
      ),

    /** Le chemin de l'image d'une page, **brouillon compris**. */
    imageDePage: (documentId: Uuid, index: number): string =>
      `/admin/negotiation/documents/${documentId}/pages/${index}/image`,

    /** Les notes du document, vivantes **et** retirées. */
    notes: (documentId: Uuid): Promise<AdminCorrectionNoteList> =>
      call(`/admin/negotiation/documents/${documentId}/corrections`, async () =>
        (await exemples()).notesDuDocument(documentId),
      ),

    /** Pose une note par-dessus le texte, **sans le modifier**. Le français est exigé. */
    poserUneNote: (documentId: Uuid, entree: CorrectionNoteInput): Promise<AdminCorrectionNote> =>
      send(`/admin/negotiation/documents/${documentId}/corrections`, entree, async () =>
        (await exemples()).poserUneNote(documentId, entree),
      ),

    /** Idempotent. Une note ne se supprime jamais : son retrait se date. */
    retirerUneNote: (noteId: Uuid): Promise<AdminCorrectionNote> =>
      send(`/admin/negotiation/corrections/${noteId}/withdraw`, {}, async () =>
        (await exemples()).retirerUneNote(noteId),
      ),
  }
}

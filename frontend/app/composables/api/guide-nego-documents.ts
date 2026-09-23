/**
 * Les documents de Guide Négo — leur part de `useApi()`, à côté de `api/guide-nego.ts`.
 *
 * **Lecture publique, écriture gardée** : la bibliothèque, la recherche, la
 * lecture, les notes et le compteur de téléchargements se passent de compte ;
 * les favoris l'exigent. Ce que la personne voit d'un document réservé, c'est
 * l'API qui le tranche, jamais l'écran.
 *
 * LES LECTURES QUI PORTENT UNE EMPREINTE passent par `lireEtiquete` : la liste,
 * les notes et les favoris la gardent pour la relecture ; la forme lisible, pour
 * savoir si la copie gardée sur le téléphone est la bonne (`reading_etag`).
 *
 * LES IMAGES DE PAGE NE SONT PAS DU JSON : la méthode rend le chemin d'API,
 * relatif à la base — le même que `ReadingPage.image`, et la clé sous laquelle
 * le cache des copies les garde.
 *
 * Le jeu d'exemple se charge à la demande, comme les autres : branchée,
 * l'application ne l'embarque pas.
 */
import type {
  CorrectionNoteList,
  DocumentBookmarkList,
  DocumentLibrary,
  DocumentReading,
  DocumentTextHits,
} from '~/types/negotiation-documents'
import type { Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'
import type { AvecEmpreinte } from './http'
import type { ApiTransport } from './proposal-review'

type Deps = Pick<ApiTransport, 'call' | 'send'> & Pick<Primitives, 'lireEtiquete'>

const exemples = () => import('~/mocks/negotiation-documents')

export function createGuideNegoDocumentsApi({ call, send, lireEtiquete }: Deps) {
  // Hors ligne, le jeu d'exemple résout ses textes dans la langue qu'`Accept-Language` porterait.
  const { $i18n } = useNuxtApp()
  const langue = (): string => String($i18n.locale.value)

  return {
    /**
     * La bibliothèque entière, avec les libellés des seules valeurs citées. Un
     * réservé y paraît pour tous, sans résumé ni thématiques pour qui n'a pas
     * l'accès.
     */
    bibliotheque: (): Promise<AvecEmpreinte<DocumentLibrary>> =>
      lireEtiquete('/negotiation/documents', async () => (await exemples()).bibliothequeDeDocuments(langue())),

    /**
     * Chercher un mot dans le texte, sans tenir compte des accents. Un réservé
     * sans accès ne rend que son identifiant : ni page, ni extrait.
     */
    rechercherDansLeTexte: (q: string): Promise<DocumentTextHits> =>
      call('/negotiation/documents', async () => (await exemples()).rechercherDansLesDocuments(q), { q }),

    /**
     * La forme lisible entière. Réservé sans accès : `NEGOTIATION_DOCUMENT_RESTRICTED` ;
     * lien externe : `NEGOTIATION_DOCUMENT_NOT_READABLE` — l'écran l'ouvre alors
     * dans le navigateur.
     */
    lecture: (documentId: Uuid): Promise<AvecEmpreinte<DocumentReading>> =>
      lireEtiquete(`/negotiation/documents/${documentId}/reading`, async () =>
        (await exemples()).lectureDuDocument(documentId),
      ),

    /** Le chemin de l'image d'une page, `index` à partir de 1. */
    imageDePage: (documentId: Uuid, index: number): string =>
      `/negotiation/documents/${documentId}/pages/${index}/image`,

    /** Les notes vivantes de tous les documents publiés, en une lecture. */
    notesDeCorrection: (): Promise<AvecEmpreinte<CorrectionNoteList>> =>
      lireEtiquete('/negotiation/documents/corrections', async () => (await exemples()).notesDeCorrection(langue())),

    /**
     * Compter un téléchargement **réussi**. Sans compte, et rien de la personne
     * n'est gardé : l'appelant n'attend pas la réponse et ne réessaie pas.
     */
    compterUnTelechargement: (documentId: Uuid): Promise<void> =>
      send(`/negotiation/documents/${documentId}/downloads`, {}, async () =>
        (await exemples()).compterUnTelechargement(documentId),
      ),

    mesFavoris: (): Promise<AvecEmpreinte<DocumentBookmarkList>> =>
      lireEtiquete('/negotiation/me/bookmarks', async () => (await exemples()).mesFavorisDeDocuments()),

    /** Idempotent : poser deux fois ne crée rien de plus. */
    poserUnFavori: (documentId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/bookmarks/${documentId}`,
        {},
        async () => (await exemples()).poserUnFavori(documentId),
        'PUT',
      ),

    /** Idempotent, même si le favori n'existe pas. */
    retirerUnFavori: (documentId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/bookmarks/${documentId}`,
        {},
        async () => (await exemples()).retirerUnFavori(documentId),
        'DELETE',
      ),
  }
}

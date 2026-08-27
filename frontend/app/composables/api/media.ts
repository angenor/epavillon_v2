/**
 * MÉDIA (B3) — sa part de `useApi()` : ce qu'un rôle attend, et le dépôt.
 *
 * ── POURQUOI CETTE FABRIQUE N'EXISTAIT PAS ──────────────────────────────────
 *
 * Le module sert le dépôt depuis B3, et aucun écran ne l'appelait. Les trois
 * déclinaisons d'une édition se rattachaient par un IDENTIFIANT D'OBJET saisi à
 * la main — un champ que seule une personne ayant accès à la base pouvait
 * remplir, c'est-à-dire personne. Le rattachement existait, le dépôt manquait.
 *
 * ── LA FORME ATTENDUE VIENT DE LA BASE, PAS D'UNE CONSTANTE ─────────────────
 *
 * `GET /media/roles` rend `media.attachable_roles` : le rapport largeur ÷
 * hauteur, sa tolérance, les types acceptés et le poids maximal. C'est ce qui
 * permet à l'éditeur de recadrage d'IMPOSER la forme au lieu de la découvrir par
 * le refus, après que le fichier a traversé le réseau. Un jeu de ratios écrit
 * ici en serait une seconde vérité, et la v1 est morte de cela.
 *
 * ── L'ORDRE DU CORPS COMPOSITE N'EST PAS UN DÉTAIL ──────────────────────────
 *
 * La route lit le corps dans l'ordre où il a été écrit et n'accepte le fichier
 * qu'APRÈS ses métadonnées : c'est ce qui lui permet de refuser un type, un
 * poids ou un droit sans avoir lu un octet. `file` est donc ajouté en dernier,
 * toujours, et un fichier reçu avant ses métadonnées est refusé.
 */

import type { AttachableRoleRule, UploadPayload, UploadedAsset } from '~/types/media'
import type { ApiTransport } from './proposal-review'

export function createMediaApi({ call, sendForm }: Pick<ApiTransport, 'call' | 'sendForm'>) {
  return {
    /**
     * CE QUE CHAQUE RÔLE D'UNE ENTITÉ EXIGE — table blanche de `media`.
     *
     * Hors ligne, la liste vient des mocks : sans elle, l'éditeur de recadrage
     * n'aurait ni forme à imposer ni plafond à annoncer, et le travail sur
     * données d'exemple serait plus permissif que le vrai — le pire des deux.
     */
    roles: (ownerSchema: string, ownerTable: string): Promise<AttachableRoleRule[]> =>
      call<AttachableRoleRule[]>(
        '/media/roles',
        (m) => m.attachableRolesOf(ownerSchema, ownerTable),
        { owner_schema: ownerSchema, owner_table: ownerTable },
      ),

    /**
     * LE DÉPÔT. Rend l'objet, et le fait qu'il existait déjà.
     *
     * L'ENTITÉ PORTEUSE EST FACULTATIVE : à la création d'une édition, elle
     * n'existe pas encore. Renseignée, elle achète les refus précoces — type,
     * poids, droit — et le contrôle du rôle ; absente, l'objet est simplement
     * déposé, et c'est le rattachement qui tranchera plus tard.
     *
     * `byte_size` est ANNONCÉ, et il doit être exact : la route refuse un flux
     * dont le poids diffère de sa déclaration. Il vient donc du `Blob` lui-même,
     * jamais d'une estimation.
     */
    upload: (payload: UploadPayload): Promise<UploadedAsset> => {
      const form = new FormData()
      form.append('filename', payload.filename)
      form.append('mime_type', payload.mimeType)
      form.append('byte_size', String(payload.file.size))
      if (payload.ownerSchema) form.append('owner_schema', payload.ownerSchema)
      if (payload.ownerTable) form.append('owner_table', payload.ownerTable)
      if (payload.ownerId) form.append('owner_id', payload.ownerId)
      if (payload.role) form.append('role', payload.role)
      // Une DONNÉE multilingue, transmise comme telle : `platform.i18n_text` est
      // ce que la colonne porte, et une chaîne nue y perdrait l'anglais.
      form.append('alt_text', JSON.stringify(payload.altText))
      // EN DERNIER, toujours. Voir l'en-tête.
      form.append('file', payload.file, payload.filename)

      return sendForm<UploadedAsset>('/media/assets', form, () => simulerLeDepot(payload))
    },
  }
}

/**
 * LE DÉPÔT SIMULÉ — hors ligne, l'objet n'existe que dans cet onglet.
 *
 * L'adresse est celle du `Blob` en mémoire (`blob:`), et elle meurt avec la
 * page : c'est exactement ce qu'on veut d'un dépôt qui n'a rien déposé. L'écran
 * se comporte alors comme en ligne — l'aperçu apparaît, le rattachement suit —
 * sans qu'aucun octet ne quitte le navigateur.
 *
 * `deduplicated` est faux : hors ligne, rien n'a d'empreinte à comparer.
 */
function simulerLeDepot(payload: UploadPayload): UploadedAsset {
  const now = new Date().toISOString()
  return {
    id: crypto.randomUUID(),
    bucket: 'epavillon-mock',
    object_key: `mock/${payload.filename}`,
    checksum_sha256: '',
    mime_type: payload.mimeType,
    byte_size: payload.file.size,
    original_filename: payload.filename,
    width: null,
    height: null,
    duration_seconds: null,
    owner_person_id: null,
    owner_organization_id: null,
    visibility: 'public',
    status: 'ready',
    scan_verdict: 'clean',
    scan_engine: null,
    scanned_at: now,
    scan_details: null,
    alt_text: payload.altText,
    caption: null,
    credit: null,
    license_code: null,
    deleted_at: null,
    deleted_by: null,
    purge_after: null,
    purged_at: null,
    created_at: now,
    updated_at: now,
    url: import.meta.client ? URL.createObjectURL(payload.file) : '',
    // Le worker ne tourne pas hors ligne : aucune déclinaison n'est prête, et
    // l'aperçu se replie sur l'original — ce que fait déjà `UiImage`.
    sources: {},
    deduplicated: false,
  }
}

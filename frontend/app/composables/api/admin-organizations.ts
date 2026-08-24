/**
 * ORGANISATIONS ET FUSION DES DOUBLONS (A11) — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-events.ts`, `api/planner.ts` et
 * `api/proposal-review.ts` : la règle du projet est inchangée — aucune page
 * n'importe un mock, aucune page n'appelle `$fetch`. Les écrans appellent
 * `api.adminOrganizations.list(…)`. Seule la place du CODE change, pour tenir
 * `useApi.ts` sous le garde-fou de mille lignes de `CLAUDE.md`.
 *
 * ── POURQUOI `adminOrganizations` ET NON `organizations` ────────────────────
 *
 * `api.organizations` existe déjà : c'est la recherche du formulaire de
 * rattachement (A2) et du formulaire de dépôt (A4) — des lectures ouvertes à
 * toute personne connectée, qui ne prennent aucun périmètre. Les appels d'ici
 * sont ceux du back-office : ils prennent le périmètre, et ils écrivent. Les
 * mélanger sous une même clé aurait fini par faire passer une écriture
 * d'administration pour une lecture publique.
 *
 * LA RECHERCHE AUSSI EST CELLE DU BACK-OFFICE, ET C'EST UNE CORRECTION. Ce
 * fichier disait le contraire jusqu'ici : l'écran de fusion cherchait sa seconde
 * fiche par `api.organizations.similar()`. Les deux lectures interrogent bien la
 * même fonction — `org.find_similar_organizations()`, il n'y en a pas deux —,
 * mais elles n'en gardent pas la même chose :
 *   · `/organizations/similar` répond à « ce que j'ai tapé, existe-t-il déjà ? »
 *     et ne retient que les fiches dont une DÉNOMINATION ressemble au terme ;
 *   · `/admin/organizations/similar` répond à « qu'est-ce qui pourrait être la
 *     même entité ? » et ne filtre rien — une fiche entrée par le seul DOMAINE
 *     PARTAGÉ y figure.
 * La seconde est un sur-ensemble de la première : on ne perd aucun résultat, et
 * on récupère exactement les fiches que la revue des doublons vient chercher.
 *
 * ── LE PÉRIMÈTRE : UN FILTRE POUR LA LISTE, UNE PERMISSION POUR LA FUSION ───
 *
 * La liste prend `AdministeredEvents` et se filtre : une personne détachée sur
 * la COP31 voit les organisations qui y ont déposé. La FUSION ne prend pas de
 * périmètre du tout — elle exige `org.organization.merge` sur la portée GLOBALE,
 * ce que l'écran vérifie par permission et que l'API vérifiera à son tour.
 * Fusionner déplace des rattachements dans toutes les éditions, y compris celles
 * qu'on n'administre pas : il n'existe pas de fusion « limitée à une COP ».
 *
 * ── CE QUE CES ÉCRITURES PEUVENT REFUSER ────────────────────────────────────
 *
 * Peu de choses, et jamais par prudence. `ux_organization_domains_verified` (un
 * domaine vérifié pour une seule fiche) et `tg_forbid_merge_chains` (pas de
 * chaîne A → B → C) sont des invariants de DONNÉES. La confirmation par le nom,
 * elle, est un garde-fou d'interface — revérifié ici parce que masquer un bouton
 * n'a jamais empêché une requête.
 */

import type {
  DomainVerificationPayload,
  DuplicateDecisionPayload,
  DuplicateDecisionResult,
  DuplicateQueueScreen,
  MergePayload,
  MergePreview,
  MergeResult,
  NameConfirmationPayload,
  OrganizationDetail,
  OrganizationListScreen,
  OrganizationVerificationPayload,
  OrganizationWriteResult,
} from '~/types/admin-organizations'
import type { AdministeredEvents } from '~/types/identity'
import type { SimilarOrganization } from '~/types/org'
import type { OrganizationSearchQuery } from '~/types/organization-join'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createAdminOrganizationsApi({ call, send }: ApiTransport) {
  return {
    /**
     * LA LISTE ET SES FACETTES — en une réponse.
     *
     * Aucun refus ici : la liste est FILTRÉE par le périmètre, et une personne
     * sans aucun droit reçoit une liste vide. C'est l'écran qui rend « accès
     * refusé », sur la permission — un tableau vide et un accès refusé ne se
     * lisent pas pareil.
     */
    list: (scope: AdministeredEvents): Promise<OrganizationListScreen> =>
      call('/admin/organizations', (m) => m.organizationListScreen(scope)),

    /**
     * LA RECHERCHE DE LA REVUE DES DOUBLONS — sans aucun filtre.
     *
     * Elle sert à choisir la seconde fiche d'une fusion. Lui passer le SITE et
     * l'ADRESSE DE CONTACT de la fiche déjà connue n'est pas un détail : c'est
     * ce qui arme le motif `shared_domain`, celui que le module désigne
     * lui-même comme le plus fiable — deux fiches qui déclarent le même domaine
     * sont la même maison, quels que soient les libellés saisis.
     */
    similar: (query: OrganizationSearchQuery): Promise<SimilarOrganization[]> =>
      call('/admin/organizations/similar', (m) => m.findSimilarOrganizations(query), {
        name: query.name,
        country_id: query.country_id ?? undefined,
        email: query.email ?? undefined,
        website: query.website ?? undefined,
        limit: query.limit,
      }),

    /**
     * LA FILE DES DOUBLONS PRÉSUMÉS — `org.duplicate_candidates`.
     *
     * Elle n'est PAS filtrée par le périmètre, et ce n'est pas un oubli : une
     * paire de doublons ne relève d'aucune édition, et sa résolution demande de
     * toute façon la permission globale. Un administrateur détaché n'y accède
     * pas du tout, plutôt que d'en voir une part.
     */
    duplicates: (): Promise<DuplicateQueueScreen> =>
      call('/admin/organizations/duplicates', (m) => m.duplicateQueue()),

    /**
     * L'APERÇU DE FUSION POUR UN SENS DONNÉ.
     *
     * `null` quand l'une des fiches est introuvable ou déjà absorbée. Il se
     * recharge à chaque inversion du sens : le décompte n'est pas symétrique —
     * ce qui est dédoublonné dépend de ce que la CIBLE porte déjà.
     */
    mergePreview: (sourceId: Uuid, targetId: Uuid, pairId: Uuid | null): Promise<MergePreview | null> =>
      call(
        `/admin/organizations/${sourceId}/merge-preview`,
        (m) => m.mergePreview(sourceId, targetId, pairId),
        { target_id: targetId, pair_id: pairId ?? undefined },
      ),

    /**
     * FUSIONNER. Exige `org.organization.merge` sur la portée globale.
     *
     * La réponse dit ce qui s'est réellement déplacé, par `schéma.table.colonne`
     * — c'est `merge_log.rows_reassigned`, et c'est ce que l'écran annonce après
     * coup. Un « fusion effectuée » sans chiffres laisserait l'opérateur sans
     * moyen de vérifier que la manœuvre a fait ce qu'elle promettait.
     */
    merge: (payload: MergePayload, actorId: Uuid | null): Promise<MergeResult> =>
      send('/admin/organizations/merge', payload, (m) => m.mergeOrganizations(payload, actorId)),

    /**
     * « CE NE SONT PAS DES DOUBLONS », ou « pas maintenant ».
     *
     * `distinct` retire la paire de la file pour de bon ; `deferred` la remet à
     * plus tard. Le modèle porte les deux valeurs, et les confondre ferait
     * revenir une paire qu'on a délibérément écartée — ou disparaître une paire
     * qu'on voulait seulement remettre.
     */
    decideDuplicate: (
      payload: DuplicateDecisionPayload,
      actorId: Uuid | null,
    ): Promise<DuplicateDecisionResult> =>
      send(
        `/admin/organizations/duplicates/${payload.pair_id}`,
        payload,
        (m) => m.decideDuplicatePair(payload, actorId),
        'PUT',
      ),

    /** TOUTE LA FICHE. Rend `null` pour une fiche inexistante — pas pour une fiche absorbée. */
    detail: (organizationId: Uuid): Promise<OrganizationDetail | null> =>
      call(`/admin/organizations/${organizationId}`, (m) => m.organizationDetail(organizationId)),

    /**
     * POSER OU RETIRER LE SCEAU DE L'IFDD.
     *
     * Le sceau n'est pas le statut : une fiche peut être active sans être
     * vérifiée. Poser le sceau sur une fiche encore `candidate` l'admet du même
     * geste — on ne certifie pas une organisation qu'on n'a pas admise.
     */
    setVerification: (
      payload: OrganizationVerificationPayload,
      actorId: Uuid | null,
    ): Promise<OrganizationWriteResult> =>
      send(
        `/admin/organizations/${payload.organization_id}/verification`,
        payload,
        (m) => m.setOrganizationVerification(payload, actorId),
        'PUT',
      ),

    /**
     * VÉRIFIER UN DOMAINE À LA MAIN ET RÉGLER SON RATTACHEMENT AUTOMATIQUE.
     *
     * `status: 'domain_taken'` traduit `ux_organization_domains_verified` et
     * NOMME la fiche qui détient déjà le domaine : sans ce nom, le refus serait
     * incompréhensible — et c'est précisément le cas des deux fiches OSED.
     */
    setDomainVerification: (
      payload: DomainVerificationPayload,
      actorId: Uuid | null,
    ): Promise<OrganizationWriteResult> =>
      send(
        `/admin/organizations/${payload.organization_id}/domains/${payload.domain_id}`,
        payload,
        (m) => m.setDomainVerification(payload, actorId),
        'PUT',
      ),

    /**
     * CONFIRMER UNE DÉNOMINATION.
     *
     * Elle sert la recherche dans les deux cas — c'est ce qui permet de
     * retrouver une fiche par une faute d'orthographe connue. La confirmation ne
     * décide que de son AFFICHAGE.
     */
    setNameConfirmation: (
      payload: NameConfirmationPayload,
      actorId: Uuid | null,
    ): Promise<OrganizationWriteResult> =>
      send(
        `/admin/organizations/${payload.organization_id}/names/${payload.name_id}`,
        payload,
        (m) => m.setNameConfirmation(payload, actorId),
        'PUT',
      ),
  }
}

/**
 * UTILISATEURS ET RÔLES (A12) — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-organizations.ts` : les pages appellent
 * `api.adminUsers.list(…)`, n'importent aucun mock et n'appellent jamais
 * `$fetch`. Seule la place du code change, pour tenir `useApi.ts` sous le
 * garde-fou de mille lignes.
 *
 * ── POURQUOI `adminUsers` ET NON `identity` ─────────────────────────────────
 *
 * `api.identity` existe déjà et porte des lectures ÉLÉMENTAIRES — une personne,
 * ses attributions brutes, ses permissions effectives, son périmètre. Tout le
 * back-office s'en sert, à commencer par les gardes de permission de chaque
 * écran. Les appels d'ici sont autre chose : des COMPOSITIONS d'écran, qui
 * prennent un périmètre et qui écrivent. Les mélanger aurait fini par faire
 * passer une attribution de rôle pour une lecture de profil.
 *
 * ── L'ACTEUR VOYAGE AVEC SES PERMISSIONS, ET CE N'EST PAS UN DÉTAIL ─────────
 *
 * Les deux écritures de rôle reçoivent `granted` — les permissions effectives de
 * celui qui agit. Tant que l'API n'existe pas, c'est la seule façon de rejouer
 * ce que `has_permission(acteur, 'identity.role.assign', portée, cible)` décidera
 * en base. Le paramètre DISPARAÎTRA au prompt B1 : l'API lit sa propre session,
 * et un client qui déclare ses propres droits n'est pas un contrôle d'accès. Il
 * est ici pour que le comportement de l'écran soit juste, pas pour le protéger.
 *
 * ── AUCUN `assertEventInScope` ──────────────────────────────────────────────
 *
 * Une personne n'appartient à aucune édition, pas plus qu'une organisation. La
 * règle métier n° 8 se tient par les deux bouts déjà décrits en A11 : la LISTE
 * est filtrée sur les éditions administrées, et chaque ÉCRITURE exige la
 * permission sur la portée visée. La file RGPD, elle, ne se filtre pas du tout —
 * une demande d'effacement porte sur la plateforme entière : elle s'ouvre sur la
 * permission globale, ou pas du tout.
 *
 * ── LA CIBLE EST DANS LE CHEMIN, JAMAIS DANS LE CORPS ───────────────────────
 *
 * Les quatre écritures prennent leur identifiant en PREMIER ARGUMENT et ne le
 * répètent pas dans la charge utile : l'API ne lit que l'URL, et un corps qui
 * porterait un second identifiant finirait par en désigner un autre.
 */

import type {
  EffectivePermissionsView,
  GrantRolePayload,
  HandlePrivacyRequestPayload,
  PersonWriteResult,
  PrivacyQueueScreen,
  PrivacyWriteResult,
  RevokeRolePayload,
  RoleAssignmentOptions,
  RoleWriteResult,
  SetPersonStatusPayload,
  UserDetail,
  UserListScreen,
} from '~/types/admin-users'
import type { AdministeredEvents, EffectivePermission } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

export function createAdminUsersApi({ call, send }: ApiTransport) {
  return {
    /**
     * LA LISTE ET SES FACETTES — en une réponse.
     *
     * Filtrée par le périmètre, jamais refusée : c'est l'écran qui rend « accès
     * refusé » sur la permission. Une liste vide et un accès refusé ne se lisent
     * pas pareil, et les confondre a déjà coûté un contrôle d'accès silencieux
     * (voir le commentaire d'`administered_events` dans `030_identity.sql`).
     */
    list: (scope: AdministeredEvents): Promise<UserListScreen> =>
      call('/admin/users', (m) => m.userListScreen(scope)),

    /**
     * LA FICHE. `null` pour une personne inexistante — jamais pour une personne
     * hors périmètre : celle-ci se consulte en LECTURE SEULE (`in_scope: false`),
     * parce qu'il faut souvent vérifier une adresse ou comprendre un rôle sans
     * avoir à y toucher.
     */
    detail: (personId: Uuid, scope: AdministeredEvents): Promise<UserDetail | null> =>
      call(`/admin/users/${personId}`, (m) => m.userDetail(personId, scope)),

    /**
     * CE QUE LE PANNEAU D'ATTRIBUTION A LE DROIT D'OFFRIR.
     *
     * Croise le catalogue des rôles (`roles.allowed_scopes`) avec ce que l'acteur
     * détient, portée par portée. Sans cet appel, le panneau proposerait à
     * l'administratrice de la COP31 d'attribuer un rôle global, que l'API
     * refuserait ensuite sans qu'elle comprenne pourquoi.
     */
    roleOptions: (granted: EffectivePermission[]): Promise<RoleAssignmentOptions> =>
      call('/admin/users/role-options', (m) => m.roleAssignmentOptions(granted)),

    /** « Voici ce que cette personne peut faire, et où » — l'écran d'explication. */
    permissions: (personId: Uuid): Promise<EffectivePermissionsView> =>
      call(`/admin/users/${personId}/effective-permissions`, (m) => m.effectivePermissionsView(personId)),

    /**
     * ATTRIBUER UN RÔLE AVEC SA PORTÉE.
     *
     * Cinq réponses possibles, et chacune dit quoi corriger : `duplicate` nomme
     * l'attribution déjà en place — y compris une attribution EXPIRÉE, que
     * `ux_role_assignments_active` compte encore —, `scope_not_allowed` traduit
     * le refus du trigger, `forbidden_scope` celui de l'autorisation.
     */
    grantRole: (
      personId: Uuid,
      payload: GrantRolePayload,
      actorId: Uuid | null,
      granted: EffectivePermission[],
    ): Promise<RoleWriteResult> =>
      send(`/admin/users/${personId}/roles`, payload, (m) => m.grantRole(personId, payload, actorId, granted)),

    /**
     * RETIRER UN RÔLE, AVEC MOTIF.
     *
     * `DELETE` par la route, mais pas une suppression : la ligne reste, avec
     * `revoked_at`, `revoked_by` et `revoked_reason`. C'est l'historique que le
     * prompt demande, et c'est aussi ce que fait la base — `role_assignments` n'a
     * pas de suppression.
     */
    revokeRole: (
      assignmentId: Uuid,
      payload: RevokeRolePayload,
      actorId: Uuid | null,
      granted: EffectivePermission[],
    ): Promise<RoleWriteResult> =>
      send(
        `/admin/users/roles/${assignmentId}`,
        payload,
        (m) => m.revokeRole(assignmentId, payload, actorId, granted),
        'DELETE',
      ),

    /**
     * SUSPENDRE, BLOQUER, RÉTABLIR.
     *
     * `missing_deadline` traduit `ck_people_suspension_window` : une suspension
     * sans terme est refusée par la base, et c'est une décision du modèle — sans
     * date de fin, c'est un blocage qui n'ose pas dire son nom.
     */
    setStatus: (
      personId: Uuid,
      payload: SetPersonStatusPayload,
      actorId: Uuid | null,
      scope: AdministeredEvents,
    ): Promise<PersonWriteResult> =>
      send(
        `/admin/users/${personId}/status`,
        payload,
        (m) => m.setPersonStatus(personId, payload, actorId, scope),
        'PUT',
      ),

    /** LA FILE RGPD. Non filtrée par le périmètre : exige la portée globale. */
    privacyQueue: (): Promise<PrivacyQueueScreen> =>
      call('/admin/privacy-requests', (m) => m.privacyQueue()),

    /**
     * TRAITER UNE DEMANDE.
     *
     * `anonymize` n'est PAS un statut : c'est l'acte irréversible qu'appelle une
     * demande d'effacement, et il ne vaut que pour elle — `wrong_type` refuse de
     * l'exécuter sur une demande d'export, qui ne réclamait qu'une copie.
     */
    handlePrivacyRequest: (
      requestId: Uuid,
      payload: HandlePrivacyRequestPayload,
      actorId: Uuid | null,
    ): Promise<PrivacyWriteResult> =>
      send(
        `/admin/privacy-requests/${requestId}`,
        payload,
        (m) => m.handlePrivacyRequest(requestId, payload, actorId),
        'PUT',
      ),
  }
}

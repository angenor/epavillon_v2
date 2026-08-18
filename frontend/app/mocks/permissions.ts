/**
 * `identity.permissions`, `identity.role_permissions` et la fonction
 * `identity.effective_permissions()` — recopie fidèle du semis de
 * `docs/database/030_identity.sql` § 3.
 *
 * POURQUOI CE FICHIER APPARAÎT AU PROMPT A7. La règle du projet est explicite :
 * « l'autorisation se teste par PERMISSION, jamais par nom de rôle, et toujours
 * avec sa portée ». Jusqu'ici, le front n'avait aucun moyen de la tenir — les
 * attributions de rôles étaient simulées, mais rien ne disait ce qu'un rôle
 * PERMET. Les écrans en étaient réduits à « cette personne administre-t-elle
 * quelque chose ? », c'est-à-dire à un test de rôle déguisé. La liste des
 * propositions est le premier écran qui offre des actions inégalement ouvertes —
 * affecter un révisionniste, changer un statut — et le premier où confondre les
 * deux se verrait.
 *
 * LE MODÈLE NE PORTE PAS DE PERMISSION « affecter un révisionniste ». La
 * répartition de la charge relève de l'appel, dont elle suit la composition
 * (`event.call_reviewers`) : c'est donc `event.call.manage` qui l'ouvre.
 * `programme.proposal.decide` reste ce qu'elle dit — retenir ou rejeter —, et
 * c'est elle que porte `proposal_transitions_allowed.required_permission` sur
 * les transitions de décision. Écart consigné dans `docs/PROGRESSION.md` : si
 * l'IFDD veut un jour confier l'affectation sans la décision, il faudra une
 * permission de plus dans `030_identity.sql`, pas une règle dans un écran.
 *
 * `super_admin` détient TOUT, y compris ce qui sera ajouté plus tard : c'est ce
 * que fait le trigger `tg_permissions_grant_super_admin` en base, et c'est
 * reproduit ici par une dérivation plutôt que par une liste à tenir à jour.
 */

import type { EffectivePermission, Permission, Role, RolePermission } from '~/types/identity'
import type { PermissionCode, Uuid } from '~/types/shared'
import { roleAssignments } from './people'

/**
 * Catalogue des rôles — `identity.roles`, recopie du semis du § 6.
 *
 * AJOUTÉ AU PROMPT A12. Jusqu'ici le front connaissait les PERMISSIONS et les
 * ATTRIBUTIONS, mais pas le catalogue : les écrans affichaient un `role_code`
 * brut là où il aurait fallu un libellé, faute d'avoir de quoi le résoudre.
 * L'écran d'attribution ne peut pas s'en passer — `allowed_scopes` est ce qui
 * décide des portées offertes, et `tg_check_role_scope()` refuse tout le reste.
 *
 * `label` et `description` sont des `i18n_text` : ce sont des DONNÉES, et un
 * administrateur peut les modifier. Les recopier dans un fichier de traduction
 * serait le défaut n° 1 de la v1.
 */
export const roles: Role[] = [
  {
    code: 'super_admin',
    label: { fr: 'Super administrateur', en: 'Super administrator' },
    description: {
      fr: 'Contrôle total de la plateforme. Seul rôle sans restriction de portée.',
      en: 'Full platform control',
    },
    allowed_scopes: ['global'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'admin',
    label: { fr: 'Administrateur', en: 'Administrator' },
    description: {
      fr: 'Administration courante, globale ou limitée à un événement',
      en: 'Day-to-day administration, global or scoped to one event',
    },
    // LES DEUX PORTÉES DE CE RÔLE SONT LA RÉPONSE AU CAS PACO. Attribué
    // globalement, il administre la plateforme ; attribué sur une édition, il
    // n'en administre qu'une — même back-office, même code, périmètre restreint.
    allowed_scopes: ['global', 'event'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'reviewer',
    label: { fr: 'Révisionniste', en: 'Reviewer' },
    description: {
      fr: "Évalue et note les propositions d'activité",
      en: 'Assesses and scores activity proposals',
    },
    allowed_scopes: ['global', 'event'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'programmer',
    label: { fr: 'Programmateur', en: 'Programme manager' },
    description: {
      fr: 'Planifie les créneaux et publie la programmation',
      en: 'Schedules slots and publishes the programme',
    },
    allowed_scopes: ['global', 'event'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'org_manager',
    label: { fr: "Référent d'organisation", en: 'Organization manager' },
    description: {
      fr: 'Gère les membres et les soumissions de son organisation',
      en: 'Manages members and submissions',
    },
    allowed_scopes: ['organization'],
    is_system: false,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'org_member',
    label: { fr: "Membre d'organisation", en: 'Organization member' },
    description: null,
    allowed_scopes: ['organization'],
    is_system: false,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'negotiator',
    label: { fr: 'Négociateur', en: 'Negotiator' },
    description: {
      fr: "Accède à l'espace de négociation",
      en: 'Access to the negotiation space',
    },
    allowed_scopes: ['global', 'negotiation_space'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'trainer',
    label: { fr: 'Formateur', en: 'Trainer' },
    description: null,
    allowed_scopes: ['global'],
    is_system: false,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'editor',
    label: { fr: 'Éditeur', en: 'Editor' },
    description: {
      fr: 'Modère les publications des organisations',
      en: 'Moderates organization publications',
    },
    allowed_scopes: ['global'],
    is_system: false,
    created_at: '2026-01-12T09:00:00Z',
  },
  {
    code: 'standard',
    label: { fr: 'Utilisateur', en: 'Standard user' },
    description: null,
    allowed_scopes: ['global'],
    is_system: true,
    created_at: '2026-01-12T09:00:00Z',
  },
]

/** Catalogue des permissions — `identity.permissions`. */
export const permissions: Permission[] = [
  { code: 'identity.person.read', label: { fr: 'Consulter les utilisateurs', en: 'Read users' }, module_code: 'identity' },
  { code: 'identity.person.manage', label: { fr: 'Gérer les utilisateurs', en: 'Manage users' }, module_code: 'identity' },
  { code: 'identity.role.assign', label: { fr: 'Attribuer des rôles', en: 'Assign roles' }, module_code: 'identity' },
  { code: 'org.organization.read', label: { fr: 'Consulter les organisations', en: 'Read organizations' }, module_code: 'org' },
  { code: 'org.organization.manage', label: { fr: 'Gérer les organisations', en: 'Manage organizations' }, module_code: 'org' },
  { code: 'org.organization.merge', label: { fr: 'Fusionner des organisations', en: 'Merge organizations' }, module_code: 'org' },
  { code: 'event.event.manage', label: { fr: 'Gérer les événements', en: 'Manage events' }, module_code: 'event' },
  { code: 'event.call.manage', label: { fr: 'Gérer les appels à propositions', en: 'Manage calls for proposals' }, module_code: 'event' },
  { code: 'programme.proposal.submit', label: { fr: 'Soumettre une proposition', en: 'Submit a proposal' }, module_code: 'programme' },
  { code: 'programme.proposal.read_all', label: { fr: 'Consulter toutes les propositions', en: 'Read all proposals' }, module_code: 'programme' },
  { code: 'programme.review.write', label: { fr: 'Noter et commenter une proposition', en: 'Score and comment a proposal' }, module_code: 'programme' },
  { code: 'programme.proposal.decide', label: { fr: 'Retenir ou rejeter une proposition', en: 'Accept or reject a proposal' }, module_code: 'programme' },
  { code: 'programme.session.schedule', label: { fr: 'Planifier les sessions', en: 'Schedule sessions' }, module_code: 'programme' },
  { code: 'programme.registration.manage', label: { fr: 'Gérer les inscriptions', en: 'Manage registrations' }, module_code: 'programme' },
  { code: 'live.meeting.manage', label: { fr: 'Gérer les réunions et diffusions', en: 'Manage meetings and streams' }, module_code: 'live' },
  { code: 'live.incident.publish', label: { fr: "Publier un message d'incident", en: 'Publish an incident message' }, module_code: 'live' },
  { code: 'publication.article.write', label: { fr: 'Rédiger un article', en: 'Write an article' }, module_code: 'publication' },
  { code: 'publication.article.moderate', label: { fr: 'Modérer les articles', en: 'Moderate articles' }, module_code: 'publication' },
  { code: 'publication.quota.manage', label: { fr: 'Définir les quotas de publication', en: 'Set publication quotas' }, module_code: 'publication' },
  { code: 'negotiation.space.access', label: { fr: "Accéder à l'espace de négociation", en: 'Access the negotiation space' }, module_code: 'negotiation' },
  { code: 'negotiation.content.manage', label: { fr: 'Gérer les contenus de négociation', en: 'Manage negotiation content' }, module_code: 'negotiation' },
  { code: 'engagement.campaign.send', label: { fr: 'Envoyer des campagnes', en: 'Send campaigns' }, module_code: 'engagement' },
  { code: 'tool.survey.manage', label: { fr: 'Gérer les sondages', en: 'Manage surveys' }, module_code: 'tool' },
  { code: 'analytics.dashboard.read', label: { fr: 'Consulter les tableaux de bord', en: 'Read dashboards' }, module_code: 'analytics' },
]

/** Ce que chaque rôle permet, hors `super_admin` — dérivé juste après. */
const GRANTS: Record<string, PermissionCode[]> = {
  admin: [
    // AJOUTÉE AU MODÈLE AU PROMPT A12 : `identity.role.assign` n'était détenue
    // que par `super_admin`, par le seul effet du trigger. Le back-office des
    // rôles n'aurait servi qu'au compte technique pivot.
    'identity.role.assign',
    'identity.person.read', 'identity.person.manage',
    'org.organization.read', 'org.organization.manage', 'org.organization.merge',
    'event.event.manage', 'event.call.manage',
    'programme.proposal.read_all', 'programme.proposal.decide',
    'programme.session.schedule', 'programme.registration.manage',
    'live.meeting.manage', 'live.incident.publish',
    'publication.article.moderate', 'publication.quota.manage',
    'engagement.campaign.send', 'analytics.dashboard.read',
  ],
  reviewer: [
    'programme.proposal.read_all', 'programme.review.write',
    'org.organization.read', 'analytics.dashboard.read',
  ],
  programmer: [
    'programme.proposal.read_all', 'programme.session.schedule',
    'live.meeting.manage', 'live.incident.publish',
  ],
  org_manager: ['programme.proposal.submit', 'publication.article.write'],
  org_member: ['programme.proposal.submit'],
  negotiator: ['negotiation.space.access'],
  editor: ['publication.article.moderate'],
  standard: ['org.organization.read'],
}

export const rolePermissions: RolePermission[] = [
  // Le super administrateur détient toute permission existante : dérivé du
  // catalogue, comme le fait le trigger, et non recopié.
  ...permissions.map((p) => ({ role_code: 'super_admin', permission_code: p.code })),
  ...Object.entries(GRANTS).flatMap(([role_code, codes]) =>
    codes.map((permission_code) => ({ role_code, permission_code })),
  ),
]

const permissionsOfRole = new Map<string, PermissionCode[]>()
for (const link of rolePermissions) {
  permissionsOfRole.set(link.role_code, [
    ...(permissionsOfRole.get(link.role_code) ?? []),
    link.permission_code,
  ])
}

/**
 * `identity.effective_permissions(person_id)` — ce que cette personne peut
 * faire, ET SUR QUELLE PORTÉE. Une même permission peut apparaître deux fois
 * avec deux portées : globale pour un administrateur de la plateforme, limitée
 * à une édition pour un compte détaché.
 *
 * LES ATTRIBUTIONS EXPIRÉES ET RÉVOQUÉES SONT ÉCARTÉES, comme le fait la
 * fonction en base : le jeu de données contient exprès une attribution arrivée
 * à son terme et une autre révoquée — un écran qui les oublie laisse entrer
 * quelqu'un qui ne siège plus.
 */
export function effectivePermissions(personId: Uuid): EffectivePermission[] {
  const now = Date.now()

  return roleAssignments
    .filter(
      (assignment) =>
        assignment.person_id === personId &&
        assignment.revoked_at === null &&
        Date.parse(assignment.valid_from) <= now &&
        (assignment.valid_until === null || Date.parse(assignment.valid_until) > now),
    )
    .flatMap((assignment) =>
      (permissionsOfRole.get(assignment.role_code) ?? []).map((permission_code) => ({
        permission_code,
        scope_type: assignment.scope_type,
        scope_id: assignment.scope_id,
      })),
    )
}

/**
 * TESTER UNE PERMISSION, JAMAIS UN RÔLE — la règle de projet, rendue exécutable.
 *
 * `identity.effective_permissions()` rend des lignes (permission, portée,
 * identifiant de portée). Une permission accordée GLOBALEMENT vaut partout ;
 * accordée sur une édition, elle ne vaut que là. La confusion est facile et
 * coûteuse : ignorer la portée fait apparaître une action de décision à un
 * administrateur détaché sur la COP31 quand il consulte une autre édition —
 * exactement la règle métier n° 8, prise par l'autre bout.
 *
 * Fonctions PURES : ni réseau, ni traduction. Le front s'en sert pour montrer ou
 * masquer une action ; l'API la vérifie à nouveau, et c'est elle qui fait foi.
 * Une action masquée reste refusée si l'URL est forgée.
 */

import type { EffectivePermission } from '~/types/identity'
import type { PermissionCode, Uuid } from '~/types/shared'

/**
 * Cette personne détient-elle la permission, sur cette édition ?
 *
 * `eventId` absent : seule une attribution GLOBALE répond oui. C'est le cas des
 * écrans qui ne portent pas d'édition — la fusion des organisations, la gestion
 * des comptes.
 */
export function hasPermission(
  granted: EffectivePermission[] | null | undefined,
  code: PermissionCode,
  eventId?: Uuid | null,
): boolean {
  if (!granted?.length) return false

  return granted.some((entry) => {
    if (entry.permission_code !== code) return false
    if (entry.scope_type === 'global') return true
    if (!eventId) return false
    return entry.scope_type === 'event' && entry.scope_id === eventId
  })
}

/** Les permissions qui valent sur cette édition, portée globale comprise. */
export function permissionsForEvent(
  granted: EffectivePermission[] | null | undefined,
  eventId: Uuid | null,
): PermissionCode[] {
  if (!granted?.length) return []

  return [
    ...new Set(
      granted
        .filter(
          (entry) =>
            entry.scope_type === 'global' ||
            (entry.scope_type === 'event' && eventId !== null && entry.scope_id === eventId),
        )
        .map((entry) => entry.permission_code),
    ),
  ]
}

/**
 * Cette personne détient-elle la permission, SUR QUELQUE PORTÉE QUE CE SOIT ?
 *
 * AJOUTÉE AU PROMPT A11, pour une entité qui n'appartient à aucune édition. Les
 * organisations sont transverses : `hasPermission(granted, 'org.organization.read')`
 * sans identifiant d'édition n'accepte qu'une attribution GLOBALE, et fermerait
 * la liste à une coordonnatrice détachée sur la COP31 — qui a pourtant à
 * connaître des organisations ayant déposé chez elle.
 *
 * La règle métier n° 8 se tient alors par l'autre bout : la permission ouvre
 * l'écran, et c'est la LISTE qui est filtrée sur les éditions administrées
 * (`api.adminOrganizations.list(scope)`). Le droit d'entrer et l'étendue de ce
 * qu'on voit sont deux questions distinctes, et les confondre donne soit un écran
 * interdit à tort, soit une liste trop large.
 *
 * À N'EMPLOYER QUE POUR LES ÉCRANS SANS ÉDITION. Une action qui porte sur une
 * édition doit passer par `hasPermission(granted, code, eventId)` : accepter ici
 * une permission accordée sur une AUTRE édition serait exactement la faille que
 * la règle n° 8 ferme.
 */
export function hasPermissionOnAnyScope(
  granted: EffectivePermission[] | null | undefined,
  code: PermissionCode,
): boolean {
  return Boolean(granted?.some((entry) => entry.permission_code === code))
}

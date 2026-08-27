/**
 * `media.attachable_roles` — la table blanche, telle que `GET /media/roles` la rend.
 *
 * POURQUOI CES LIGNES EXISTENT HORS LIGNE. L'éditeur de recadrage IMPOSE la
 * forme du fichier au lieu de la découvrir par le refus : il lui faut donc le
 * rapport attendu, sa tolérance et le poids maximal, y compris quand l'API n'est
 * pas configurée. Sans elles, le travail sur données d'exemple serait plus
 * permissif que le vrai — et l'écart ne se verrait qu'en production.
 *
 * ELLES RECOPIENT LE SEMIS DE `docs/database/050_media.sql` § 4, à la ligne près.
 * C'est une copie assumée et bornée : la vérité reste en base, ces lignes ne
 * servent qu'à faire tourner l'écran sans elle. Toute modification du semis se
 * répercute ici.
 *
 * LE RAPPORT TRAVERSE EN TEXTE, comme dans la réponse réelle : `numeric(6,4)`
 * n'a pas de représentant flottant exact, et un rapport affiché doit l'être tel
 * qu'il est déclaré.
 */

import type { AttachableRoleRule } from '~/types/media'

const MIO = 1024 * 1024

export const attachableRoles: AttachableRoleRule[] = [
  {
    owner_schema: 'event',
    owner_table: 'events',
    role: 'banner',
    label: { fr: 'Bandeau panoramique', en: 'Panoramic banner' },
    is_multiple: false,
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 15 * MIO,
    expected_aspect_ratio: '3.5556',
    aspect_ratio_tolerance: '0.02',
    is_active: true,
  },
  {
    owner_schema: 'event',
    owner_table: 'events',
    role: 'cover',
    label: { fr: 'Image de couverture', en: 'Cover image' },
    is_multiple: false,
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 10 * MIO,
    expected_aspect_ratio: '1.7778',
    aspect_ratio_tolerance: '0.02',
    is_active: true,
  },
  {
    owner_schema: 'event',
    owner_table: 'events',
    role: 'thumbnail',
    label: { fr: 'Vignette carrée', en: 'Square thumbnail' },
    is_multiple: false,
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 5 * MIO,
    expected_aspect_ratio: '1.0000',
    aspect_ratio_tolerance: '0.02',
    is_active: true,
  },
  {
    owner_schema: 'event',
    owner_table: 'events',
    role: 'gallery',
    label: { fr: 'Galerie', en: 'Gallery' },
    is_multiple: true,
    allowed_mime_prefixes: ['image/*'],
    max_byte_size: 15 * MIO,
    expected_aspect_ratio: null,
    aspect_ratio_tolerance: '0.02',
    is_active: true,
  },
]

/**
 * Les rôles d'une entité. Une entité inconnue rend une liste VIDE, jamais tous
 * les rôles : la table est blanche, et une combinaison non déclarée est refusée.
 */
export function attachableRolesOf(ownerSchema: string, ownerTable: string): AttachableRoleRule[] {
  return attachableRoles.filter(
    (rule) => rule.owner_schema === ownerSchema && rule.owner_table === ownerTable,
  )
}

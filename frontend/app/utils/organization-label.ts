import type { PublicScheduleRow } from '~/types/views'

export interface OrganizationNaming {
  name?: string | null
  acronym?: string | null
  /** Case « Sigle bien connu » de la fiche, posée par l'équipe. */
  acronymWellKnown?: boolean | null
}

/**
 * Le nom d'une organisation là où la place manque (panneau de l'accueil, carte
 * du calendrier, parties d'une séance commune) : le sigle seul s'il est bien
 * connu du public (OIF), sinon le nom complet, qu'aucun sigle obscur (RF-EFH)
 * ne remplace. L'écran le tient sur une ligne, coupé par des points de
 * suspension. Les vignettes de repli gardent le sigle : un visuel, pas une étiquette.
 */
export function organizationShortLabel({ name, acronym, acronymWellKnown }: OrganizationNaming): string {
  const sigle = acronym?.trim() || null
  const nom = name?.trim() || null
  if (sigle && (acronymWellKnown || !nom)) return sigle
  return nom ?? sigle ?? ''
}

/** L'organisation porteuse d'une ligne du programme public. */
export function scheduleOrganizationLabel(session: PublicScheduleRow): string {
  return organizationShortLabel({
    name: session.organization_name,
    acronym: session.organization_acronym,
    acronymWellKnown: session.organization_acronym_well_known,
  })
}

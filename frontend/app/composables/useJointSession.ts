import type { JointProposal, PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'
import type { OrganizationId, ProposalId } from '~/types/shared'
import type { UiSlide } from '~/types/ui'

/** Un dossier d'une séance commune, prêt à afficher. */
export interface JointPart {
  id: ProposalId
  /** À partir de 1, dans l'ordre d'affichage. */
  number: number
  title: string
  cover: AttachedImage | null
  organizationId: OrganizationId
  name: string
  acronym: string | null
  /** Là où la place manque : `organizationShortLabel`. */
  label: string
  logo: AttachedImage | null
  /** Le pays ne se dit que d'une institution publique nationale, comme pour une séance ordinaire ; vide sinon. */
  nationalCountry: string
  countryCode: string | null
}

/** Séance commune (plusieurs dossiers dans un créneau) : partagé par la liste, la semaine et la fiche. */
export function useJointSession() {
  const { tr } = useI18nText()
  const { locale, t, te } = useI18n()

  /** Vide pour une séance ordinaire : un seul dossier n'en fait pas une séance commune. */
  function jointParts(proposals: JointProposal[] | undefined | null): JointPart[] {
    if (!proposals || proposals.length < 2) return []
    return [...proposals]
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((proposal, index) => ({
        id: proposal.proposal_id,
        number: index + 1,
        title: tr(proposal.title),
        cover: proposal.cover,
        organizationId: proposal.organization.id,
        name: proposal.organization.name,
        acronym: proposal.organization.acronym,
        label: organizationShortLabel({
          name: proposal.organization.name,
          acronym: proposal.organization.acronym,
          acronymWellKnown: proposal.organization.acronym_well_known,
        }),
        logo: proposal.organization.logo,
        nationalCountry:
          proposal.organization.type_code === 'public_national_institution' && proposal.organization.country
            ? tr(proposal.organization.country)
            : '',
        countryCode: proposal.organization.country_code,
      }))
  }

  const partsOf = (session: PublicScheduleRow): JointPart[] => jointParts(session.proposals)

  /** « AIRF et AIMF », « AIRF, AIMF et OIF » : la conjonction suit la langue. */
  const namesOf = (parts: JointPart[]): string =>
    new Intl.ListFormat(locale.value, { type: 'conjunction' }).format(parts.map((part) => part.label))

  /** « Deux activités dans ce créneau » : le nombre en lettres tant que la langue en a, en chiffres au-delà. */
  function countLabel(count: number): string {
    const spelled = `programme.joint.spelled.${count}`
    return t('programme.joint.count', { count: te(spelled) ? t(spelled) : count }, count)
  }

  /**
   * Les couvertures qui se relaient dans une vignette : celles des dossiers, un
   * dossier sans couverture sauté ; à défaut, celle de la séance ; sinon rien.
   */
  function slidesOf(parts: JointPart[], cover: AttachedImage | null, key: string): UiSlide[] {
    const covers = parts.flatMap((part) => (part.cover ? [{ key: part.id, image: part.cover }] : []))
    if (covers.length) return covers
    return cover ? [{ key, image: cover }] : []
  }

  return { jointParts, partsOf, namesOf, countLabel, slidesOf }
}

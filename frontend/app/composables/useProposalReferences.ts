import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { Country, Locale, TaxonomyTerm } from '~/types/reference'
import type { Uuid } from '~/types/shared'

/** Ce que les étapes du dossier lisent pour une édition : son appel et ses référentiels. */
export interface ProposalReferences {
  call: CallForProposals | null
  edition: EventEdition | null
  themes: TaxonomyTerm[]
  categories: TaxonomyTerm[]
  documentTypes: TaxonomyTerm[]
  locales: Locale[]
  countries: Country[]
}

/**
 * Partagé par le formulaire de dépôt et la fiche du dossier : deux chargements
 * recopiés divergeraient au premier référentiel ajouté à une étape.
 */
export async function loadProposalReferences(api: ReturnType<typeof useApi>, eventId: Uuid): Promise<ProposalReferences> {
  const [call, edition, themes, categories, documentTypes, locales, countries] = await Promise.all([
    api.events.call(eventId),
    // `publicList()` et non `list()` : cette dernière filtre par périmètre
    // d'ADMINISTRATION, et une organisation qui dépose n'en a aucun.
    api.events.publicList().then((list) => list.find((event) => event.id === eventId) ?? null),
    api.reference.terms('activity_theme'),
    api.reference.terms('activity_category'),
    api.reference.terms('document_type'),
    api.reference.locales(),
    api.reference.countries(),
  ])
  return {
    call,
    edition,
    themes,
    categories,
    documentTypes,
    locales: locales.filter((entry) => entry.is_active),
    countries,
  }
}

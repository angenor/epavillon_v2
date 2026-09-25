/**
 * Le savoir de Guide Négo — FAQ, parcours, lexique — : sa part de `useApi()`.
 *
 * Un seul paquet, public, relu par différence : avec le `served_at` de la lecture
 * gardée, l'API ne rend que ce qui a changé ; avec son empreinte, un `304` si rien
 * n'a bougé. Les termes favoris, eux, demandent une session.
 */
import type { KnowledgeBundle, MyGlossaryFavorites } from '~/types/negotiation-savoir'
import type { IsoDateTime, Uuid } from '~/types/shared'
import type { Primitives } from './guide-nego'
import type { AvecEmpreinte } from './http'

type Deps = Pick<Primitives, 'lireEtiquete' | 'send'>

export interface DepuisLaGarde {
  since: IsoDateTime
  empreinte: string | null
}

const exemples = () => import('~/mocks/negotiation-savoir')

const parametres = (depuis: DepuisLaGarde | null): string =>
  depuis ? `?since=${encodeURIComponent(depuis.since)}` : ''

export function createGuideNegoSavoirApi({ lireEtiquete, send }: Deps) {
  const { $i18n } = useNuxtApp()
  const langue = (): string => String($i18n.locale.value)

  return {
    /** Sans garde : tout le publié. Avec : la différence depuis sa lecture, ou `304`. */
    paquet: (depuis: DepuisLaGarde | null) =>
      lireEtiquete<KnowledgeBundle>(
        '/negotiation/knowledge' + parametres(depuis),
        async () => (await exemples()).paquetDuSavoir(langue(), depuis?.since ?? null),
        depuis?.empreinte ?? null,
      ),

    mesTermesFavoris: (): Promise<AvecEmpreinte<MyGlossaryFavorites>> =>
      lireEtiquete('/negotiation/me/glossary-favorites', async () => (await exemples()).mesTermesFavoris()),

    /** Idempotent. Entrée inconnue ou en brouillon : 404. */
    poserUnTermeFavori: (entryId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/glossary-favorites/${entryId}`,
        {},
        async () => (await exemples()).poserUnTermeFavori(entryId),
        'PUT',
      ),

    /** Compteur des « plus lues », sans auteur ; hors file : une lecture perdue ne coûte rien (R12). */
    lireUneEntreeDeFaq: (entryId: Uuid): Promise<void> =>
      send(`/negotiation/faq/${entryId}/read`, {}, () => undefined),

    /** Idempotent, même si le favori n'existe pas. */
    retirerUnTermeFavori: (entryId: Uuid): Promise<void> =>
      send(
        `/negotiation/me/glossary-favorites/${entryId}`,
        {},
        async () => (await exemples()).retirerUnTermeFavori(entryId),
        'DELETE',
      ),
  }
}

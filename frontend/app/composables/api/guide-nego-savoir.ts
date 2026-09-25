/**
 * Le savoir de Guide Négo — FAQ, parcours, lexique — : sa part de `useApi()`.
 *
 * Un seul paquet, public, relu par différence : avec le `served_at` de la lecture
 * gardée, l'API ne rend que ce qui a changé ; avec son empreinte, un `304` si rien
 * n'a bougé. Chaque écriture s'ajoute ici avec la tâche qui livre sa route.
 */
import type { KnowledgeBundle } from '~/types/negotiation-savoir'
import type { IsoDateTime } from '~/types/shared'
import type { Primitives } from './guide-nego'

type Deps = Pick<Primitives, 'lireEtiquete'>

export interface DepuisLaGarde {
  since: IsoDateTime
  empreinte: string | null
}

const exemples = () => import('~/mocks/negotiation-savoir')

const parametres = (depuis: DepuisLaGarde | null): string =>
  depuis ? `?since=${encodeURIComponent(depuis.since)}` : ''

export function createGuideNegoSavoirApi({ lireEtiquete }: Deps) {
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
  }
}

import { defineStore } from 'pinia'
import type { ResolvedFeatureFlag } from '~/types/platform'
import type { FeatureFlagKey } from '~/types/shared'
import type { LoadFailure } from '~/utils/api-error'

/**
 * Les drapeaux de `platform.feature_flags`, chargés une seule fois.
 *
 * POURQUOI UN STORE. Le middleware global `feature-flag` s'exécute à CHAQUE
 * navigation : sans cache, la traversée du site déclencherait un appel par page
 * visitée, pour une table qui change une fois par trimestre. Le store charge à
 * la première navigation et se tait ensuite, exactement comme celui des
 * rattachements.
 *
 * CE N'EST PAS UN CONTRÔLE DE SÉCURITÉ — la remarque vaut ici comme pour les
 * autres gardes. Un module fermé dont on forge l'adresse ne s'ouvre pas parce
 * que le middleware serait contourné : l'API refuse de toute façon de servir un
 * module éteint. Ici on évite un écran qui n'existe pas encore.
 *
 * LA VALEUR SÛRE EST « FERMÉ », et elle vaut aussi en cas de panne. Si l'appel
 * échoue, `isEnabled` rend `false` : on annonce un espace en maintenance plutôt
 * que d'ouvrir la porte d'un module que le jalon n'a pas construit. C'est le
 * même défaut que celui du `COALESCE` de `platform.is_feature_enabled()`.
 */
export const useFeatureStore = defineStore('features', () => {
  const api = useApi()

  const flags = ref<ResolvedFeatureFlag[]>([])
  const isLoading = ref(false)
  const loadError = ref<LoadFailure | null>(null)
  const isLoaded = ref(false)

  /**
   * Le drapeau est-il ouvert POUR LA PERSONNE QUI REGARDE ?
   *
   * Le déploiement progressif n'est pas arbitré ici, et ne l'a jamais été : il
   * dépend de qui regarde, et la fonction qui sait le calculer est en base.
   * C'est désormais l'API qui rend le booléen résolu — le test du pourcentage
   * qui vivait ici en était une seconde version, condamnée à diverger.
   */
  function isEnabled(key: FeatureFlagKey): boolean {
    return flags.value.find((entry) => entry.key === key)?.is_enabled === true
  }

  /** Charge la table une fois. Idempotent : le middleware l'appelle sans se coordonner. */
  async function ensureLoaded(): Promise<void> {
    if (isLoaded.value || isLoading.value) return

    isLoading.value = true
    loadError.value = null
    try {
      flags.value = await api.platform.featureFlags()
      isLoaded.value = true
    } catch (error) {
      loadError.value = toLoadFailure(error)
      flags.value = []
      // `isLoaded` reste FAUX : la prochaine navigation vers un module fermé
      // retentera. Le poser ici figerait « tout est fermé » pour la durée de la
      // session, y compris après le retour de la plateforme.
    } finally {
      isLoading.value = false
    }
  }

  return { flags, isLoading, isLoaded, loadError, isEnabled, ensureLoaded }
})

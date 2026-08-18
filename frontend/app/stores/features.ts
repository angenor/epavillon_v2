import { defineStore } from 'pinia'
import type { FeatureFlag } from '~/types/platform'
import type { FeatureFlagKey } from '~/types/shared'

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

  const flags = ref<FeatureFlag[]>([])
  const isLoading = ref(false)
  const loadError = ref<Error | null>(null)
  const isLoaded = ref(false)

  /**
   * Un drapeau ouvert pour tout le monde.
   *
   * Le déploiement progressif — `rollout_percent` entre 1 et 99, `enabled_for`
   * nommant des personnes — n'est PAS arbitré ici : il dépend de qui regarde, et
   * la fonction qui sait le calculer est en base. Aucun drapeau du semis ne s'en
   * sert ; le jour où l'un le fera, c'est l'API qui rendra le booléen résolu.
   */
  function isEnabled(key: FeatureFlagKey): boolean {
    const flag = flags.value.find((entry) => entry.key === key)
    return flag !== undefined && flag.is_enabled && flag.rollout_percent === 100
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
      loadError.value = error instanceof Error ? error : new Error(String(error))
      flags.value = []
    } finally {
      isLoading.value = false
    }
  }

  return { flags, isLoading, isLoaded, loadError, isEnabled, ensureLoaded }
})

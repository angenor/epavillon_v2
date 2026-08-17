import { defineStore } from 'pinia'
import type { Membership, Organization } from '~/types/org'

/**
 * Les rattachements de la personne connectée.
 *
 * POURQUOI UN STORE ET PAS UN APPEL PAR ÉCRAN. La question « cette personne
 * a-t-elle une organisation ? » se pose désormais à trois endroits qui ne se
 * connaissent pas : l'écran de rattachement, la garde qui l'impose avant
 * certaines actions (`middleware/requires-organization.ts`), et la barre de
 * navigation. Trois appels indépendants, c'est trois réponses possiblement
 * différentes dans une même navigation — et la garde qui laisse passer quelqu'un
 * que l'écran suivant refuse.
 *
 * CE N'EST PAS UN CONTRÔLE DE SÉCURITÉ, exactement comme le store de session.
 * Le droit de déposer au nom d'une organisation vient de `org.memberships` ET du
 * rôle `org_member`, vérifiés par l'API (`identity.has_permission()`). Ici on
 * évite un écran vide et un aller-retour, rien de plus.
 *
 * ADHÉSION ACTIVE ET DEMANDE EN ATTENTE NE SE CONFONDENT PAS. `status = 'pending'`
 * signifie qu'un référent n'a pas encore accepté : la personne n'a pas le droit
 * d'agir au nom de l'organisation, et lui dire « c'est bon » serait lui promettre
 * un dépôt qui échouera. Les deux sont donc exposés séparément — l'un ouvre les
 * portes, l'autre explique pourquoi elles restent fermées.
 */
export const useMembershipStore = defineStore('membership', () => {
  const api = useApi()
  const auth = useAuthStore()

  const entries = ref<{ membership: Membership; organization: Organization }[]>([])
  const isLoading = ref(false)
  const loadError = ref<Error | null>(null)
  /** Identifiant de la personne pour laquelle les données ont été chargées. */
  const loadedFor = ref<string | null>(null)

  const active = computed(() => entries.value.filter((e) => e.membership.status === 'active'))
  const pending = computed(() => entries.value.filter((e) => e.membership.status === 'pending'))

  /** Peut-elle agir au nom d'une organisation ? La seule question qui ouvre une porte. */
  const hasActiveOrganization = computed(() => active.value.length > 0)
  /** A-t-elle une demande en cours ? Ce qui explique une porte encore fermée. */
  const hasPendingRequest = computed(() => pending.value.length > 0)

  /**
   * Charge les rattachements une fois par personne. Idempotent : la garde, la
   * barre et l'écran l'appellent sans se coordonner.
   */
  async function ensureLoaded(): Promise<void> {
    await auth.ensureLoaded()
    const person = auth.person
    if (!person) {
      entries.value = []
      loadedFor.value = null
      return
    }
    if (loadedFor.value === person.id || isLoading.value) return

    isLoading.value = true
    loadError.value = null
    try {
      const memberships = await api.organizations.membershipsOf(person.id)
      const loaded = await Promise.all(
        memberships.map(async (membership) => {
          const organization = await api.organizations.byId(membership.organization_id)
          return organization ? { membership, organization } : null
        }),
      )
      entries.value = loaded.filter((entry) => entry !== null)
      loadedFor.value = person.id
    } catch (error) {
      loadError.value = error instanceof Error ? error : new Error(String(error))
    } finally {
      isLoading.value = false
    }
  }

  /** Après un rattachement ou une création : la prochaine lecture repart de l'API. */
  async function refresh(): Promise<void> {
    loadedFor.value = null
    await ensureLoaded()
  }

  return {
    entries,
    active,
    pending,
    hasActiveOrganization,
    hasPendingRequest,
    isLoading,
    loadError,
    ensureLoaded,
    refresh,
  }
})

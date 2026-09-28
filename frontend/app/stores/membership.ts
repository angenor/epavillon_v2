import { defineStore } from 'pinia'
import type { Membership, Organization } from '~/types/org'
import type { LoadFailure } from '~/utils/api-error'

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
 * REJOINDRE NE BLOQUE PAS LE DÉPÔT (arbitré le 28/09). Une demande en attente du
 * référent suffit pour déposer et suivre ses propres dossiers ; ceux des
 * collègues restent fermés jusqu'à la validation. Une invitation en attente, elle,
 * n'ouvre rien : la personne n'a pas encore accepté.
 */
export const useMembershipStore = defineStore('membership', () => {
  const api = useApi()
  const auth = useAuthStore()

  const entries = ref<{ membership: Membership; organization: Organization }[]>([])
  const isLoading = ref(false)
  const loadError = ref<LoadFailure | null>(null)
  /** Identifiant de la personne pour laquelle les données ont été chargées. */
  const loadedFor = ref<string | null>(null)

  const active = computed(() => entries.value.filter((e) => e.membership.status === 'active'))
  /** Les organisations au nom desquelles elle peut déposer : actives, ou demandées. */
  const submittable = computed(() =>
    entries.value.filter(
      (e) =>
        e.membership.status === 'active' ||
        (e.membership.status === 'pending' && e.membership.invited_at === null),
    ),
  )

  const hasSubmittableOrganization = computed(() => submittable.value.length > 0)

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
      loadError.value = toLoadFailure(error)
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
    submittable,
    hasSubmittableOrganization,
    isLoading,
    loadError,
    ensureLoaded,
    refresh,
  }
})

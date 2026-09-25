import type { OfficialSession } from '~/types/negotiation-sessions'
import { rappelsDus } from '~/utils/guide-nego/agenda'
import { cleRappelVu, lireCle, poserCle } from '~/utils/guide-nego/stockage'

/**
 * « Me rappeler 15 minutes avant » (FR-034) : un bandeau dans l'application ouverte,
 * rien d'autre — ni son, ni vibration, ni notification. Il se calcule depuis la garde,
 * donc sans réseau aussi.
 *
 * Montré une fois par session et par début : déplacée, la session se rappelle à sa
 * nouvelle heure (FR-035).
 */
export function useGnRappel() {
  const compte = useGnSession()
  const agenda = useGnAgenda()
  const lecture = useGnSessions()

  const maintenant = shallowRef(new Date())
  const affiche = ref<OfficialSession | null>(null)

  function verifier(): void {
    if (!compte.connectee.value) return void (affiche.value = null)
    const dus = rappelsDus(agenda.agenda.value, lecture.sessions.value, maintenant.value)
    const courant = affiche.value ? dus.find((s) => s.id === affiche.value?.id) : undefined
    if (courant) return void (affiche.value = courant)
    const suivant = dus.find((s) => lireCle(cleRappelVu(s.id)) !== s.start_at) ?? null
    if (suivant) poserCle(cleRappelVu(suivant.id), suivant.start_at)
    affiche.value = suivant
  }

  function auPremierPlan(): void {
    if (document.visibilityState !== 'visible') return
    maintenant.value = new Date()
    agenda.assurer()
  }

  // La liste n'est lue que si un rappel est armé : sans cela, chaque ouverture la relirait.
  watch(
    () => agenda.agenda.value.entries.some((e) => e.remind),
    (arme) => {
      if (arme && !lecture.etat.value.pret && !lecture.etat.value.enCours) void lecture.rafraichir()
    },
  )
  watch([maintenant, () => agenda.agenda.value, () => lecture.sessions.value, compte.connectee], verifier)

  let horloge: ReturnType<typeof setInterval> | undefined
  onMounted(async () => {
    horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
    document.addEventListener('visibilitychange', auPremierPlan)
    await compte.assurer()
    agenda.assurer()
  })
  onBeforeUnmount(() => {
    clearInterval(horloge)
    document.removeEventListener('visibilitychange', auPremierPlan)
  })

  return {
    rappel: computed(() => affiche.value),
    fuseau: computed(() => lecture.fuseau.value ?? 'UTC'),
    ville: lecture.ville,
    fermer: () => (affiche.value = null),
  }
}

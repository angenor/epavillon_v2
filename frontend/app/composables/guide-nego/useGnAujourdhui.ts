import { memeJour } from '~/utils/guide-nego/journee'

/**
 * Le jour de « Ma journée » — **recalculé au retour au premier plan**.
 *
 * Une application restée en mémoire ne se rouvre pas : sans ce recalcul, elle
 * ouvrirait le lendemain sur la journée de la veille. C'est le cadre que 3a
 * remplira de ses sessions.
 */
export function useGnAujourdhui() {
  const aujourdhui = shallowRef(new Date())

  function relire(): void {
    if (document.visibilityState !== 'visible') return
    const maintenant = new Date()
    if (!memeJour(maintenant, aujourdhui.value)) aujourdhui.value = maintenant
  }

  onMounted(() => document.addEventListener('visibilitychange', relire))
  onScopeDispose(() => {
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', relire)
  })

  return { aujourdhui: readonly(aujourdhui) }
}

import type { MaybeRefOrGetter } from 'vue'
import type { BirdCursorOptions } from '~/types/bird-cursor'

const SCRIPT_ID = 'bird-cursor-script'

// Au niveau du module : une page quittée puis rouverte pendant le chargement du
// script doit retrouver son oiseau, ce qu'un état par instance ne garantit pas.
let wanted: BirdCursorOptions | null = null

/**
 * L'oiseau qui suit le curseur (`docs/bird-cursor/`), monté le temps de la page
 * appelante. Il se pose sur les `data-bird-perch` et parle au survol des
 * `data-bird-say`. Le script respecte de lui-même « moins d'animations ».
 */
export function useBirdCursor(source: MaybeRefOrGetter<BirdCursorOptions> = {}): void {
  onMounted(() => {
    const options = toValue(source)
    // Souris seulement : sur un écran tactile, il volerait au-dessus du texte qu'on fait défiler.
    if (!window.matchMedia('(hover: hover) and (pointer: fine)').matches) return
    wanted = options
    if (window.BirdCursor) {
      window.BirdCursor.mount(options)
      return
    }
    if (document.getElementById(SCRIPT_ID)) return

    const script = document.createElement('script')
    script.id = SCRIPT_ID
    script.src = assetUrl('/bird-cursor.js')
    // Le script se monte seul à son chargement, avec ses réglages par défaut : on reprend la main.
    script.addEventListener('load', () => {
      if (wanted) window.BirdCursor?.mount(wanted)
      else window.BirdCursor?.destroy()
    })
    document.body.appendChild(script)
  })

  // Ses phrases changent avec la langue : on le remonte, s'il est déjà là.
  watch(
    () => toValue(source),
    (options) => {
      if (!wanted || !window.BirdCursor) return
      wanted = options
      window.BirdCursor.mount(options)
    },
    { deep: true },
  )

  onBeforeUnmount(() => {
    wanted = null
    window.BirdCursor?.destroy()
  })
}

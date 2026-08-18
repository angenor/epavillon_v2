import type { ApexOptions } from 'apexcharts'

/**
 * LES GRAPHIQUES PRENNENT LEURS COULEURS DANS LES JETONS DE DESIGN, PAS DANS UNE
 * PALETTE DE BIBLIOTHÈQUE.
 *
 * ApexCharts attend des couleurs littérales — il dessine du SVG qu'il colore
 * lui-même, et `var(--color-accent)` n'y produit rien. Le réflexe serait d'écrire
 * les hexadécimaux dans les options : ce serait exactement la faute que la charte
 * interdit, et le thème sombre s'en apercevrait le premier — un cyan calculé pour
 * le fond blanc devient illisible sur fond noir, et personne ne le verrait avant
 * la capture d'écran d'un utilisateur.
 *
 * ON LIT DONC LES JETONS DANS LE DOM, à l'exécution, par un élément témoin. Le
 * navigateur a déjà résolu la cascade : `--color-accent` y vaut la nuance du
 * thème ACTIF, thème sombre et préférence système comprises. Une seule source de
 * vérité, celle de `design-tokens.css`, et rien à tenir d'accord.
 *
 * PAS DE VALEUR DE REPLI. La palette vaut `null` tant que le DOM n'a pas parlé,
 * et les graphiques attendent : une couleur de secours écrite ici serait une
 * couleur en dur de plus, qui ne se verrait que le jour où la lecture échoue.
 *
 * ELLE SE RELIT À CHAQUE BASCULE DE THÈME — choix explicite comme préférence
 * système. Sans cela, les graphiques garderaient les couleurs du thème dans
 * lequel la page a été ouverte.
 */

export interface ChartPalette {
  accent: string
  accentSoft: string
  success: string
  warning: string
  danger: string
  postponed: string
  neutral: string
  text: string
  /**
   * Encre POSÉE SUR UN APLAT — l'inverse de `text`. Les deux forment toujours une
   * paire clair/sombre, quel que soit le thème : c'est ce qui permet de choisir
   * une couleur de texte lisible sur un aplat dont la teinte vient de la base
   * (une thématique et son `color_hex`), sans écrire ni noir ni blanc en dur.
   */
  textInverse: string
  textMuted: string
  textSubtle: string
  border: string
  borderSubtle: string
  surface: string
  surfaceSunken: string
}

/** Rôle de couleur d'une série, tel que les écrans le demandent. */
export type ChartTone = 'accent' | 'success' | 'warning' | 'danger' | 'postponed' | 'neutral'

/**
 * Une série ApexCharts, réduite à ce que la plateforme en utilise. Le typage du
 * paquet est volontairement permissif (`any`) ; celui-ci nous fait attraper une
 * série mal formée à la compilation.
 */
export interface ChartSeries {
  name?: string
  type?: 'bar' | 'line' | 'area'
  data: ChartDatum[]
}

/** Un point : valeur seule, couple (instant, valeur), ou point nommé. */
export type ChartDatum =
  | number
  | null
  | [number, number | null]
  | { x: string | number; y: number | null }

const TOKENS: Record<keyof ChartPalette, string> = {
  accent: '--color-accent-solid',
  accentSoft: '--color-accent-border',
  success: '--color-success-solid',
  warning: '--color-warning-solid',
  danger: '--color-danger-solid',
  postponed: '--color-postponed',
  neutral: '--color-neutral-solid',
  text: '--color-text',
  textInverse: '--color-text-inverse',
  textMuted: '--color-text-muted',
  textSubtle: '--color-text-subtle',
  border: '--color-border',
  borderSubtle: '--color-border-subtle',
  surface: '--color-surface-raised',
  surfaceSunken: '--color-surface-sunken',
}

/**
 * Lecture par ÉLÉMENT TÉMOIN, et non par `getPropertyValue`.
 *
 * Les deux fonctionnent le plus souvent, mais `getPropertyValue` rend la valeur
 * telle qu'elle est déclarée — parfois un `var()` de plus, que la bibliothèque de
 * graphiques ne saurait pas résoudre. Poser la couleur sur un élément et relire
 * sa valeur calculée donne toujours un `rgb()` littéral.
 */
function readPalette(): ChartPalette {
  const probe = document.createElement('span')
  probe.setAttribute('aria-hidden', 'true')
  probe.style.position = 'absolute'
  probe.style.opacity = '0'
  probe.style.pointerEvents = 'none'
  document.body.appendChild(probe)

  const entries = Object.entries(TOKENS).map(([role, token]) => {
    probe.style.color = `var(${token})`
    return [role, getComputedStyle(probe).color]
  })

  probe.remove()
  return Object.fromEntries(entries) as ChartPalette
}

export function useChartTheme() {
  const preferences = usePreferencesStore()

  /** Partagée par tous les graphiques de la page : ils basculent ensemble. */
  const palette = useState<ChartPalette | null>('chart-palette', () => null)
  const fontFamily = useState<string>('chart-font', () => '')
  const isDark = useState<boolean>('chart-dark', () => false)

  function refresh(): void {
    if (import.meta.server) return
    palette.value = readPalette()
    fontFamily.value = getComputedStyle(document.body).fontFamily
    // Les deux seules gardes de `design-tokens.css` : l'attribut explicite, et
    // la préférence du système quand il n'y en a pas.
    isDark.value =
      preferences.theme === 'dark' ||
      (preferences.theme === 'system' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches)
  }

  onMounted(() => {
    refresh()

    /*
     * ON OBSERVE LE DOM, ON NE DEVINE PAS LE MOMENT.
     *
     * Le thème est posé par `app.vue` sur l'attribut `data-theme` du `<html>`, et
     * cette pose est ASYNCHRONE. Relire la palette en réaction au magasin de
     * préférences — même avec `nextTick` — lisait le DOM avant qu'il n'ait reçu le
     * nouvel attribut : les graphiques gardaient un thème de retard, et l'encre
     * claire du thème sombre se retrouvait sur le fond blanc du thème clair, où
     * elle est illisible. Mesuré en basculant clair → sombre → clair sans
     * recharger la page.
     *
     * Un observateur d'attribut ne peut pas se tromper de moment : il se déclenche
     * APRÈS la mutation, et `getComputedStyle` rend alors la cascade à jour. Il
     * couvre aussi le passage au mode « système », où l'attribut est retiré.
     */
    const observer = new MutationObserver(() => refresh())
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme'],
    })

    // Mode « système » : c'est la préférence du navigateur qui décide, et elle
    // change sans qu'aucun attribut ne bouge.
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    media.addEventListener('change', refresh)

    onScopeDispose(() => {
      observer.disconnect()
      media.removeEventListener('change', refresh)
    })
  })

  const toneColor = (tone: ChartTone): string => palette.value?.[tone] ?? 'transparent'

  /**
   * LES OPTIONS COMMUNES À TOUS LES GRAPHIQUES DE LA PLATEFORME.
   *
   * Ce qu'elles ÉTEIGNENT est le sujet : la barre d'outils (télécharger un SVG
   * n'est pas un geste de tableau de bord), les dégradés et les ombres portées,
   * les étiquettes posées sur chaque point. La direction artistique du projet
   * proscrit les dégradés et les halos ; les valeurs par défaut d'ApexCharts en
   * mettent partout.
   *
   * Chaque graphique étale cet objet et redéfinit les sections dont il a besoin.
   * On ne fusionne pas en profondeur : une fusion silencieuse cache ce qui gagne.
   */
  function baseOptions(): ApexOptions {
    return {
      chart: {
        fontFamily: fontFamily.value,
        foreColor: palette.value?.textMuted,
        background: 'transparent',
        toolbar: { show: false },
        zoom: { enabled: false },
        parentHeightOffset: 0,
        animations: { enabled: true, speed: 320 },
        dropShadow: { enabled: false },
      },
      grid: {
        borderColor: palette.value?.borderSubtle,
        strokeDashArray: 0,
        xaxis: { lines: { show: false } },
        yaxis: { lines: { show: true } },
        padding: { top: 0, right: 0, bottom: 0, left: 4 },
      },
      dataLabels: { enabled: false },
      legend: { show: false },
      fill: { type: 'solid', opacity: 1 },
      states: {
        hover: { filter: { type: 'lighten' } },
        active: { filter: { type: 'none' } },
      },
      tooltip: {
        theme: isDark.value ? 'dark' : 'light',
        style: { fontFamily: fontFamily.value },
        marker: { show: false },
      },
      stroke: { curve: 'smooth', lineCap: 'round' },
    }
  }

  return { palette, isDark, fontFamily, toneColor, baseOptions, refresh }
}

import type { CoverDesignOptions } from '~/types/programme/cover-design'
import type { CoverScene, CoverTemplate } from './cover-templates/draw.ts'
import { COVER_HEIGHT, COVER_WIDTH } from './cover-templates/draw.ts'
import { faces, fullPhoto, splitPhoto, typographic } from './cover-templates/classic.ts'
import { revue } from './cover-templates/revue.ts'
import { ephemeride } from './cover-templates/ephemeride.ts'
import { diagonale } from './cover-templates/diagonale.ts'
import { passePartout } from './cover-templates/passe-partout.ts'
import { duotone } from './cover-templates/duotone.ts'
import { plateau } from './cover-templates/plateau.ts'
import { billet } from './cover-templates/billet.ts'
import { carton } from './cover-templates/carton.ts'
import { grandsChiffres } from './cover-templates/grands-chiffres.ts'
import { encart } from './cover-templates/encart.ts'
import { ficheSeance } from './cover-templates/fiche-seance.ts'
import { hublot } from './cover-templates/hublot.ts'

/**
 * L'ATELIER DE L'IMAGE DE COUVERTURE (29/10, option A du commanditaire) : les
 * modèles vivent dans le code du site (`cover-templates/`, un fichier par
 * modèle) ; la base ne garde que leur code et les choix de l'organisation. Un
 * modèle est une fonction de dessin sur un canvas de 1600 × 900 : l'aperçu et
 * l'image enregistrée sortent du même dessin, au pixel près. L'image sert aussi
 * d'aperçu quand on partage le lien.
 */

export type { CoverColors, CoverDate, CoverDrawOptions, CoverScene, CoverSpeaker, CoverTemplate, TextMeasurer } from './cover-templates/draw.ts'
export {
  COVER_ACCENTS,
  COVER_HEIGHT,
  COVER_WIDTH,
  DEFAULT_COVER_OPTIONS,
  fitText,
  withAlpha,
  wrapLines,
} from './cover-templates/draw.ts'
export type { CoverAccent } from './cover-templates/draw.ts'

/** Ordre du catalogue de l'atelier. */
export const COVER_TEMPLATES: CoverTemplate[] = [
  fullPhoto,
  splitPhoto,
  faces,
  typographic,
  revue,
  ephemeride,
  diagonale,
  passePartout,
  duotone,
  plateau,
  billet,
  carton,
  grandsChiffres,
  encart,
  ficheSeance,
  hublot,
]

export function findCoverTemplate(code: string | null | undefined): CoverTemplate {
  return COVER_TEMPLATES.find((template) => template.code === code) ?? fullPhoto
}

export function drawCover(ctx: CanvasRenderingContext2D, scene: CoverScene, templateCode: string, options: CoverDesignOptions): void {
  const platformLogo = options.show_partner_logos ? scene.platformLogo : null
  const organizationLogo = options.show_org_logo ? scene.organizationLogo : null
  ctx.clearRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
  ctx.save()
  findCoverTemplate(templateCode).draw(
    ctx,
    { ...scene, platformLogo, organizationLogo },
    {
      show_date: options.show_date,
      show_speakers: options.show_speakers,
      // Un bandeau vide n'a rien à porter : il suit les logos réellement présents.
      show_logos: Boolean(platformLogo ?? organizationLogo),
      show_event_banner: options.show_event_banner,
      show_event_icon: options.show_event_icon,
    },
  )
  ctx.restore()
}

/** Les sorties de l'image montrées à l'organisation : l'originale et son aperçu de partage.
 *  Les courriels prennent leurs visuels aux gabarits du back-office (31/10). */
export type CoverFormat = 'cover' | 'share'

/** Une couleur libre : `#rrggbb` en minuscules, la forme que la base accepte. */
export const CUSTOM_ACCENT = /^#[0-9a-f]{6}$/

/** Contraste du texte blanc sur une couleur (WCAG) : sous 4,5, les titres de l'image se lisent mal. */
export function whiteContrast(hex: string): number {
  const channel = (index: number) => {
    const value = Number.parseInt(hex.slice(index, index + 2), 16) / 255
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  }
  const luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
  return 1.05 / (luminance + 0.05)
}

/** Teinte, intensité, clarté vers `#rrggbb`. */
export function hslToHex(hue: number, saturation: number, lightness: number): string {
  const s = saturation / 100
  const l = lightness / 100
  const k = (n: number) => (n + hue / 30) % 12
  const a = s * Math.min(l, 1 - l)
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)))
  const hex = (x: number) =>
    Math.round(x * 255)
      .toString(16)
      .padStart(2, '0')
  return `#${hex(f(0))}${hex(f(8))}${hex(f(4))}`
}

export function hexToHsl(hex: string): { hue: number; saturation: number; lightness: number } {
  const [r, g, b] = [1, 3, 5].map((index) => Number.parseInt(hex.slice(index, index + 2), 16) / 255) as [number, number, number]
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  const lightness = (max + min) / 2
  let hue = 0
  let saturation = 0
  if (max !== min) {
    const d = max - min
    saturation = lightness > 0.5 ? d / (2 - max - min) : d / (max + min)
    hue = (max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4) * 60
  }
  return { hue: Math.round(hue), saturation: Math.round(saturation * 100), lightness: Math.round(lightness * 100) }
}

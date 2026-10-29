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

export type { CoverColors, CoverDate, CoverScene, CoverSpeaker, CoverTemplate, TextMeasurer } from './cover-templates/draw.ts'
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
  ctx.clearRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
  ctx.save()
  findCoverTemplate(templateCode).draw(ctx, scene, options)
  ctx.restore()
}

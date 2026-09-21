/**
 * Les 57 symboles de `assets/guide-nego/pictogrammes.svg`, en type.
 *
 * Sans cette union, `nom` était un texte libre : un nom fautif ne donnait ni erreur ni
 * avertissement, seulement un carré vide à l'écran — et un pictogramme manquant sur une
 * marque d'état ôte la moitié de ce qui la rend lisible.
 */
export const NOMS_DE_PICTO = [
  'home', 'nego', 'franco', 'res', 'wifi-off', 'search', 'bell', 'user', 'filter', 'back',
  'chevron', 'chev-down', 'chev-up', 'toc', 'text-size', 'bookmark', 'external', 'close',
  'more', 'flag', 'calendar', 'download', 'share', 'send', 'lock', 'unlock', 'doc', 'play',
  'quiz', 'clock', 'pin', 'sync', 'info', 'warn', 'star', 'check', 'live', 'moved',
  'diamond', 'circle', 'check-circle', 'x-circle', 'refresh', 'shield-check', 'shield',
  'translate', 'sun', 'eye', 'plus', 'minus', 'chat', 'moon', 'logout', 'paperclip',
  'reply', 'copy', 'pause',
] as const

export type NomDePicto = (typeof NOMS_DE_PICTO)[number]

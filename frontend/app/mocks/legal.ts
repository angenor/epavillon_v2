/**
 * Les textes qui engagent, sans API — **tels que l'API les sert aujourd'hui** :
 * en attente du texte de l'IFDD, sans corps, version `2026-01`. Aucun texte n'est
 * écrit ici : un jeu d'exemple qui en inventerait un le ferait passer pour vrai.
 */
import type { LegalText, LegalTextKey } from '~/types/platform'

export function texteJuridique(cle: LegalTextKey): LegalText {
  return {
    key: cle,
    locale: 'fr',
    status: 'pending',
    version: '2026-01',
    effective_date: null,
    body: null,
  }
}

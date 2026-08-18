/**
 * Schéma `platform` — la part que les écrans consomment.
 * Dérivé de `docs/database/010_platform.sql`.
 *
 * UN SEUL TYPE POUR L'INSTANT, et il n'est pas décoratif. L'écran des
 * permissions effectives (A12) montre ce qu'une personne peut faire, groupé PAR
 * MODULE : vingt-quatre permissions à plat ne se lisent pas, alors que
 * « Programmation : 4 · Organisations : 2 » se lit d'un coup d'œil. Le nom d'un
 * module est une donnée — `platform.modules.display_name`, en `i18n_text` — et
 * pas une chaîne d'interface : le recopier dans un fichier i18n serait
 * exactement le défaut n° 1 de la v1, appliqué au découpage technique.
 *
 * Le reste de `platform` — outbox, travaux, audit, drapeaux — n'a pas d'écran :
 * ses types viendront avec eux.
 */

import type { I18nText, IsoDateTime, Url } from './shared'

/** Mode de déploiement d'un module — ENUM `platform.module_deployment`. */
export type ModuleDeployment = 'embedded' | 'external'

/** Table `platform.modules` — `010_platform.sql` § 1. */
export interface PlatformModule {
  /** Clé primaire, ex. `programme`. C'est le préfixe des codes de permission. */
  code: string
  schema_name: string
  display_name: I18nText
  deployment: ModuleDeployment
  /** Renseignée seulement pour un module extrait en service distant. */
  base_url: Url | null
  depends_on: string[]
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

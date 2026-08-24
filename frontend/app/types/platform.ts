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
 * DEPUIS A14, `FeatureFlag` s'y ajoute : la page « En cours de maintenance »
 * n'est pas posée écran par écran mais SERVIE PAR LE ROUTAGE, qui doit donc
 * savoir lire `platform.feature_flags`.
 *
 * Le reste de `platform` — outbox, travaux, audit — n'a pas d'écran : ses types
 * viendront avec eux.
 */

import type { FeatureFlagKey, I18nText, IsoDateTime, Url, Uuid } from './shared'

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

/**
 * Table `platform.feature_flags` — `010_platform.sql` § 5.
 *
 * ELLE N'EST PAS RENDUE PAR L'API, et ne doit pas l'être : `enabled_for` est une
 * liste d'identifiants de personnes et `description` est écrite pour
 * l'exploitant. Ce type décrit la LIGNE, pour les données simulées qui
 * reproduisent la base ; ce que le site reçoit est `ResolvedFeatureFlag`.
 *
 * DEUX NATURES DE DRAPEAUX, que `900_seed.sql` § 2 distingue explicitement et
 * qu'il ne faut pas confondre : `<module>.enabled` ferme l'interface d'un module
 * ENTIER — c'est celui que le routage lit pour servir la page « En cours de
 * maintenance » ; les drapeaux plus fins (`negotiation.channels`,
 * `tools.ai_assistant`) commandent une fonctionnalité À L'INTÉRIEUR d'un module
 * déjà ouvert et ne peuvent pas en tenir lieu.
 *
 */
export interface FeatureFlag {
  key: FeatureFlagKey
  /** À quoi sert le drapeau, en français. Écrit pour l'exploitant, pas affiché. */
  description: string
  is_enabled: boolean
  /** Déploiement progressif : 100 pour tout le monde, 0 pour personne. */
  rollout_percent: number
  /** Personnes explicitement ouvertes, quel que soit le pourcentage. */
  enabled_for: Uuid[]
  updated_at: IsoDateTime
}

/**
 * Ce que le site reçoit — `ResolvedFeatureFlag`, rendu par
 * `GET /platform/feature-flags`.
 *
 * DEUX CHAMPS, ET C'EST TOUT. Le déploiement progressif — un pourcentage, une
 * liste de personnes explicitement ouvertes — est tranché par
 * `platform.is_feature_enabled()`, en base : le refaire côté site en donnerait
 * une seconde version, qui divergerait au premier ajustement.
 */
export interface ResolvedFeatureFlag {
  key: FeatureFlagKey
  /** Le verdict POUR L'APPELANT. Hors session, seul un déploiement complet ouvre. */
  is_enabled: boolean
}

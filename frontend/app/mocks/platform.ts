/**
 * `platform.modules` — recopie du semis de `010_platform.sql` § 1.
 *
 * DOUZE LIGNES, ET UNE SEULE RAISON DE LES AVOIR ICI : l'écran des permissions
 * effectives (A12) groupe par module. Vingt-quatre permissions à plat ne se
 * lisent pas ; « Programmation : 4 · Organisations : 3 » se lit d'un coup d'œil,
 * et c'est aussi la façon dont le modèle lui-même est organisé — un module, un
 * schéma, un crate.
 *
 * `display_name` EST UNE DONNÉE, pas une chaîne d'interface. Le préfixe d'un code
 * de permission (`programme.proposal.decide`) est un code technique ; le nom
 * affiché du module vient de la base. Le recopier dans un fichier i18n serait
 * exactement le défaut n° 1 de la v1, appliqué au découpage technique.
 */

import type { PlatformModule } from '~/types/platform'

function platformModule(
  code: string,
  display_name: PlatformModule['display_name'],
  depends_on: string[],
): PlatformModule {
  return {
    code,
    schema_name: code,
    display_name,
    deployment: 'embedded',
    base_url: null,
    depends_on,
    created_at: '2026-01-10T08:00:00Z',
    updated_at: '2026-01-10T08:00:00Z',
  }
}

export const platformModules = [
  platformModule('identity', { fr: 'Identité', en: 'Identity' }, []),
  platformModule('org', { fr: 'Organisations', en: 'Organizations' }, ['identity']),
  platformModule('event', { fr: 'Événements', en: 'Events' }, ['org', 'identity']),
  platformModule('programme', { fr: 'Programmation', en: 'Programme' }, ['event', 'org', 'identity']),
  platformModule('live', { fr: 'Direct', en: 'Live' }, ['programme']),
  // AJOUTÉ AU MODÈLE AU PROMPT A15 (`115_content.sql` § 0) : la vitrine
  // éditoriale porte sa permission, donc son groupe dans l'écran A12.
  platformModule('content', { fr: 'Contenu éditorial', en: 'Editorial content' }, [
    'identity',
    'org',
    'media',
    'event',
    'programme',
  ]),
  platformModule('publication', { fr: 'Publications', en: 'Publications' }, ['org', 'identity']),
  platformModule('negotiation', { fr: 'Négociations', en: 'Negotiations' }, ['identity']),
  platformModule('engagement', { fr: 'Engagement', en: 'Engagement' }, ['identity']),
  platformModule('media', { fr: 'Médiathèque', en: 'Media' }, ['identity']),
  platformModule('tool', { fr: 'Outils', en: 'Tools' }, ['identity']),
  platformModule('analytics', { fr: 'Analytique', en: 'Analytics' }, []),
] satisfies PlatformModule[]

/**
 * Ordre d'affichage des modules dans l'écran des permissions.
 *
 * Ni alphabétique ni celui du semis : celui du JALON. Ce qu'un administrateur de
 * l'IFDD manipule tous les jours vient d'abord — la programmation, les
 * événements, les organisations — et les modules encore en maintenance ferment
 * la liste. Un tri alphabétique mettrait « Analytique » en tête et
 * « Programmation » en huitième position.
 */
const MODULE_ORDER = [
  'programme',
  'event',
  'org',
  'identity',
  'live',
  'content',
  'analytics',
  'engagement',
  'media',
  'publication',
  'negotiation',
  'tool',
]

export function moduleRank(code: string): number {
  const rank = MODULE_ORDER.indexOf(code)
  return rank === -1 ? MODULE_ORDER.length : rank
}

export function moduleByCode(code: string): PlatformModule | null {
  return platformModules.find((entry) => entry.code === code) ?? null
}

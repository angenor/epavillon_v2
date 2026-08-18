/**
 * `platform.feature_flags` — recopie du semis de `900_seed.sql` § 2.
 *
 * POURQUOI CES DONNÉES EXISTENT ICI. La page « En cours de maintenance » (A14)
 * n'est pas posée écran par écran : c'est le ROUTAGE qui la sert quand le
 * drapeau du module est éteint. Le routage a donc besoin de lire les drapeaux,
 * et il les lit comme tout le reste — par `useApi()`, jamais en dur.
 *
 * LES VALEURS SONT CELLES DU SEMIS, sans arrangement. Douze drapeaux, dont deux
 * seulement sont ouverts : la liste d'attente et le rattachement automatique par
 * domaine. Tout ce qui touche aux modules hors périmètre est éteint — c'est
 * exactement ce que le jalon décrit, et c'est ce qui rend les six pages de
 * maintenance visibles en développement sans qu'on ait rien à truquer.
 *
 * `enabled_for` est vide partout : le semis ne nomme personne, et une liste
 * inventée ici ouvrirait un module à un compte de démonstration sans que rien,
 * dans le modèle, ne le justifie.
 */

import type { FeatureFlag } from '~/types/platform'

function flag(
  key: string,
  description: string,
  is_enabled: boolean,
  rollout_percent: number,
): FeatureFlag {
  return {
    key,
    description,
    is_enabled,
    rollout_percent,
    enabled_for: [],
    updated_at: '2026-01-10T08:00:00Z',
  }
}

export const featureFlags = [
  flag('publications.enabled', 'Espace Publications ouvert aux organisations.', false, 0),
  flag('negotiation.enabled', 'Espace Négociations, réservé aux négociateurs.', false, 0),
  flag(
    'negotiation.channels',
    "Canaux d'échange temps réel, à l'intérieur de l'espace Négociations.",
    false,
    0,
  ),
  flag(
    'training.enabled',
    'Espace Formations : catalogue, chapitres, quiz, attestations.',
    false,
    0,
  ),
  flag(
    'messaging.enabled',
    'Messagerie directe et mise en relation entre membres (tables du module engagement).',
    false,
    0,
  ),
  flag(
    'directory.enabled',
    "Annuaire des organisations et des personnes, et profils publics — l'espace Communauté.",
    false,
    0,
  ),
  flag('tools.enabled', 'Espace Outils.', false, 0),
  flag(
    'tools.ai_assistant',
    'Assistant IA et recherche documentaire (RAG), à l\'intérieur des Outils.',
    false,
    0,
  ),
  flag("tools.surveys", "Outil de sondages, à l'intérieur des Outils.", false, 0),
  flag(
    'calendar.external_sync',
    'Synchronisation Google Agenda / Apple Calendar (phase ultérieure).',
    false,
    0,
  ),
  flag("newsletter.campaigns", "Campagnes d'infolettre (hors périmètre du jalon 1).", false, 0),
  flag('programme.waitlist', "Liste d'attente sur les sessions à jauge limitée.", true, 100),
  flag(
    'org.auto_join_by_domain',
    "Rattachement automatique à une organisation par domaine de courriel vérifié.",
    true,
    100,
  ),
] satisfies FeatureFlag[]

/**
 * `platform.is_feature_enabled(clé, personne)` rejouée — mêmes règles, même
 * ordre, même défaut.
 *
 * TROIS CONDITIONS, ET LE DÉFAUT EST « FERMÉ ». Un drapeau absent de la table
 * rend `false` : c'est le comportement du `COALESCE` de la fonction SQL, et
 * c'est la valeur sûre — une clé mal orthographiée ferme un espace au lieu d'en
 * ouvrir un par accident.
 *
 * LE TIRAGE PAR HACHAGE N'EST PAS REJOUÉ. La fonction SQL calcule
 * `md5(clé || personne) % 100 < pourcentage` pour qu'une même personne voie
 * toujours la même chose. Le reproduire ici demanderait un MD5 dans le
 * navigateur pour un cas qu'aucun drapeau du semis n'utilise — les douze sont à
 * 0 ou à 100. Le déploiement progressif se jouera côté API, qui a la fonction ;
 * ici on traite les deux bornes, et 0 < pourcentage < 100 est traité comme
 * ouvert pour la personne nommée, fermé sinon.
 */
export function isFeatureEnabled(key: string, personId: string | null = null): boolean {
  const entry = featureFlags.find((f) => f.key === key)
  if (!entry || !entry.is_enabled) return false
  if (entry.rollout_percent === 100) return true
  if (personId !== null && entry.enabled_for.includes(personId)) return true
  return false
}

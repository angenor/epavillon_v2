# Modèle de données — Guide Négo, coquille (0a)

## En base : une ligne, aucune structure

`docs/database/900_seed.sql` § 2, dans l'`INSERT` existant de `platform.feature_flags (key, description, is_enabled, rollout_percent)` :

| `key` | `is_enabled` | `rollout_percent` | Rôle |
|---|---|---|---|
| `guide_nego.enabled` | `false` | `0` | **Ajoutée.** Ouvre ou ferme l'application entière |
| `negotiation.channels` | `false` | `0` | **Existante, inchangée.** Allumée, la barre passe à cinq onglets |

« Allumé » = `is_enabled = true` **et** `rollout_percent = 100` : sans session, `platform.is_feature_enabled()` n'ouvre qu'à 100 %. Pas de déploiement progressif pour ces deux drapeaux tant que l'application s'ouvre sans compte.

Ni table, ni colonne, ni type, ni fonction. Le changement se consigne dans `docs/progression/modele.md`.

## Ce que l'API rend

`GET /platform/feature-flags` → `ResolvedFeatureFlag[]` (`frontend/app/types/platform.ts`) : `{ key: string; is_enabled: boolean }`. Inchangé.

## Sur le téléphone

### IndexedDB — base `guide-nego`, magasin `lectures` (clé : `cle`)

| Champ | Type | Sens |
|---|---|---|
| `cle` | texte | Ce qui a été lu. À cette étape : `drapeaux` |
| `valeur` | objet simple | La réponse gardée. Pour `drapeaux` : `{ 'guide_nego.enabled': boolean, 'negotiation.channels': boolean }` |
| `lu_a` | instant ISO | Heure de la lecture réussie — c'est elle qu'affichent « lu à » et « Synchronisé à » |
| `empreinte` | texte ou nul | Nul à cette étape ; l'`ETag` des listes à partir de l'étape 1 |

Règles : une lecture réussie remplace la ligne ; un échec ne touche à rien ; `valeur` ne porte que des objets simples.

### `localStorage`

| Clé | Valeurs | Défaut |
|---|---|---|
| `gn.theme` | `clair` · `sombre` · `systeme` | `systeme` |
| `gn.ouverture-vue` | `1` | absente |
| `gn.garde-annoncee` | `1` | absente — posée quand « Prête hors connexion » a été dit |

Lues avant le premier affichage. Aucune ne quitte l'appareil.

### Cache du service worker

Un cache par version, `gn-<buildId>`, rempli **en entier à l'installation** d'après la liste calculée à la construction : la page vide, les fichiers de Guide Négo, le paquet de locale `fr`, la police, le sprite, les icônes, le manifeste. Aucune réponse d'API. Deux caches coexistent le temps d'une mise à jour.

## États

**Application** : `fermée` ⇄ `ouverte`, selon la résolution de R7 (`research.md`).

**Connexion** : `en ligne` (dernière lecture réussie) ⇄ `hors connexion` (navigateur hors ligne, ou dernière lecture échouée). Entrer dans `hors connexion` remet `bandeauVu` à faux ; le bandeau affiché une fois le passe à vrai.

**Thème affiché** : `clair` ou `sombre`, fonction de `gn.theme` et, pour `systeme`, du réglage du téléphone.

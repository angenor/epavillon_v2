<!--
Sync Impact Report — 2026-08-20
Version : GABARIT (aucune) → 1.0.0
Motif du calibrage : première ratification. Le fichier ne contenait que les
placeholders du gabarit `constitution-template` (aucune valeur de projet, aucun
amendement antérieur à préserver).

Principes ajoutés (10, contre 5 emplacements au gabarit — nombre imposé par
l'entrée B0 de docs/PROMPTS_DEVELOPPEMENT.md) :
  I.    Le modèle de données fait autorité
  II.   Frontières de modules
  III.  Frontières vérifiables en base (xmod_fk_*)
  IV.   Effets de bord par l'outbox transactionnel
  V.    Autorisation par permission et par portée
  VI.   SQLx vérifié à la compilation, pas d'ORM
  VII.  Contexte d'écriture systématique (app.actor_id, app.request_id)
  VIII. Les invariants de la base ne sont pas réimplémentés
  IX.   Erreurs d'API : code stable, message français
  X.    Tests d'intégration sur base réelle

Sections ajoutées :
  [SECTION_2_NAME]  → « Contraintes techniques »
  [SECTION_3_NAME]  → « Flux de travail et portes de qualité »

Sections supprimées : aucune.

Titres structurels conservés en anglais (`## Core Principles`, `## Governance`)
pour rester lisibles par les autres compétences Spec Kit ; tout le contenu est
en français, comme le reste du dépôt.

TODO reportés : aucun. Toutes les valeurs sont renseignées.
-->

# Constitution — ePavillon v2

Plateforme numérique de l'IFDD (Institut de la Francophonie pour le développement durable, organe de
l'OIF). API Rust + Actix Web + SQLx sur PostgreSQL 17, front Nuxt 4.

Ce document gouverne le développement de l'API (phase B). Il **découle de [CLAUDE.md](../../CLAUDE.md)
et ne le contredit jamais** : en cas de désaccord apparent, `CLAUDE.md` et `docs/database/` tranchent,
et la constitution est amendée pour cesser de diverger.

## Core Principles

### I. Le modèle de données fait autorité

`docs/database/*.sql` est la source de vérité du modèle. Aucun nom de table, de colonne, de type,
d'énumération ou de fonction ne se devine ni ne s'invente : on lit le fichier concerné, repéré dans
[docs/MODELE_INDEX.md](../../docs/MODELE_INDEX.md), et on en dérive les structures Rust.

Aucune table et aucune colonne ne DOIT être créée sans avoir d'abord été ajoutée au fichier SQL
concerné. Si le modèle paraît insuffisant pour une fonctionnalité, l'ordre est : modifier le SQL,
recharger une base propre (`down -v` puis `up -d` — le schéma n'est chargé qu'au premier démarrage du
conteneur), vérifier que la chaîne passe, puis écrire le code, et noter le changement dans
`docs/progression/modele.md`.

Un champ qui existe côté application sans exister en base est une dette immédiate, et c'est ainsi que
la v1 a fini avec des libellés de thématiques figés dans le front et désynchronisés de la base.

### II. Frontières de modules

Tout le Rust vit dans `backend/`, workspace Cargo — symétrique de `frontend/`. Les emplacements sont
imposés et ne se réinventent pas :

| Chemin | Contenu |
|--------|---------|
| `backend/crates/kernel/` | contexte de requête, erreurs, i18n, accès base, bus d'événements |
| `backend/crates/contracts/` | contrats d'événements partagés entre modules — la seule chose qu'ils échangent |
| `backend/crates/api/` | binaire HTTP Actix Web · `cargo run -p api` |
| `backend/crates/worker/` | travaux différés et relais d'outbox · `cargo run -p worker` |
| `backend/crates/modules/<nom>/` | un crate par module métier |

**Un module = un schéma PostgreSQL = un crate dans `backend/crates/modules/`.** Le registre
`platform.modules` (`010_platform.sql` § 1) porte cette cartographie en base et est lu au démarrage de
l'API : un module `disabled` n'est pas monté dans le routeur, un module `external` bascule vers un
client HTTP.

Un crate de module NE DOIT JAMAIS dépendre d'un autre crate de module : uniquement de `kernel` et de
`contracts`. `api` et `worker` dépendent des modules ; l'inverse n'existe pas. Une dépendance
croisée entre deux modules est un échec de revue, jamais un compromis acceptable.

### III. Frontières vérifiables en base (`xmod_fk_*`)

Toute clé étrangère traversant deux schémas métier DOIT être nommée `xmod_fk_*`. Les schémas
`platform` et `reference` forment le noyau partagé et sont exemptés — c'est la vue
`platform.cross_module_fk_report` (`000_bootstrap.sql` § 9) qui en décide, pas une appréciation.

```sql
SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;  -- doit rester vide
```

Cette requête est déjà une assertion bloquante de `make check-db`. Elle DOIT le rester : le jour où un
module part en service autonome, la liste des liens à couper se produit mécaniquement par
`platform.generate_module_decoupling_script('<schéma>')`, sans relecture manuelle du schéma. Les
autres préfixes de nommage sont non négociables au même titre : `ck_` vérification, `ux_` unique,
`ix_` index, `ex_` exclusion.

### IV. Effets de bord par l'outbox transactionnel

Les effets de bord inter-modules passent par `platform.emit_event()`, appelée dans la **même
transaction** que le changement d'état. La fonction écrit dans `platform.outbox_events`, y agrège
l'acteur et l'identifiant de requête, et réveille le relais par `pg_notify('platform_outbox', …)` ;
le worker publie ensuite.

On appelle la fonction — on n'INSÈRE JAMAIS dans `platform.outbox_events` à la main. Le type
d'événement respecte `module.ressource.action` (contrainte `ck_outbox_event_type_format`). Tout
consommateur DOIT se garder du rejeu par `platform.inbox_events (consumer, event_id)` : un worker qui
redémarre ne produit pas deux fois ses effets.

**Jamais d'appel direct d'un module à un autre.** C'est ce qui interdit structurellement le courriel
envoyé pour une activité non validée, et la validation sans notification.

Un travail différé qui n'annonce pas un changement d'état — envoi de rappel, variante d'image,
rafraîchissement analytique — passe par `platform.jobs` et `platform.claim_jobs()`, avec un
`idempotency_key` dès qu'un doublon serait visible par un utilisateur.

### V. Autorisation par permission et par portée

L'autorisation se teste par **permission**, via `identity.has_permission(personne, permission,
type_de_portée, id_de_portée)`, jamais par nom de rôle, et **toujours avec sa portée** : un compte
détaché sur la COP31 n'agit que là. Une permission de mutation se vérifie sur la portée **visée** par
l'écriture, pas sur celle de l'appelant ; retirer un droit exige la même permission qu'attribuer, sur
la même portée. Un client qui déclare lui-même ses droits n'est pas un contrôle d'accès : l'API lit sa
propre session.

Toute liste du back-office est bornée au périmètre d'administration de l'appelant.
`identity.administered_events` est une **fonction qui retourne une table**, pas une vue : on l'appelle,
on ne la joint pas.

```sql
SELECT is_global, event_ids FROM identity.administered_events($1);
-- puis :  WHERE is_global OR event_id = ANY(event_ids)
```

Elle renvoie **toujours** exactement une ligne et jamais de NULL. Les trois cas sont distincts et DOIVENT
le rester dans le code :

| Retour | Signification | Réponse de l'API |
|--------|---------------|------------------|
| `(true, …)` | tous les événements | accès complet, `event_ids` non signifiant |
| `(false, {…})` | les éditions listées | liste filtrée sur ces éditions |
| `(false, '{}')` | aucun droit | **refus d'accès explicite**, jamais une liste vide |

Un administrateur d'événement NE DOIT atteindre aucune donnée d'une autre édition, **y compris en
forgeant une URL** : une route paramétrée par un identifiant vérifie le périmètre avant de lire.

### VI. SQLx vérifié à la compilation, pas d'ORM

Les requêtes passent par les macros SQLx à vérification à la compilation. Aucun ORM, aucun
constructeur de requêtes dynamique là où une requête statique suffit.

`DATABASE_URL` doit donc être renseignée et la base démarrée pour compiler : c'est le prix, et c'est
ce qui fait qu'un nom de colonne inventé échoue au `cargo build` et non en production.

Les vues prêtes à l'emploi du modèle (`programme.v_public_schedule`, `programme.v_proposal_dashboard`,
`content.v_showcase`, `event.v_public_editions`, `analytics.v_operational_health`) répondent à un
écran en une requête : les utiliser plutôt que recomposer la jointure à la main.

### VII. Contexte d'écriture systématique

Toute transaction en écriture positionne `app.actor_id` et `app.request_id` en début de transaction
(`SET LOCAL`), avant la première écriture. `platform.current_actor_id()` et
`platform.current_request_id()` les relisent ; `platform.tg_audit()` alimente `platform.audit_log`, et
`platform.entity_history()` en dérive l'historique champ par champ de toute entité auditée.

Une écriture sans contexte produit une trace anonyme : l'audit et l'historique de la fiche cessent
d'être exploitables, et c'est exactement le défaut de la v1, dont la table `activity_modifications`
n'était alimentée que par les écritures passant par le bon chemin.

### VIII. Les invariants de la base ne sont pas réimplémentés

Le code ne redouble pas une contrainte `CHECK`, une exclusion, un trigger ou une machine à états déjà
portée par le modèle : il **traduit l'erreur PostgreSQL** en message français exploitable par
l'interface, à partir du `SQLSTATE` et du nom de la contrainte.

Corollaires que le jalon rencontre déjà :

- Les transitions d'une proposition se lisent dans `programme.proposal_transitions_allowed` ; on ne
  recopie pas la machine à états en Rust.
- Les chevauchements de créneaux **ne sont jamais bloqués** : le système les détecte
  (`programme.detect_conflicts()`) et les signale, il ne refuse pas. Seule la publication du programme
  est conditionnée.
- Les vocabulaires ouverts vivent dans `reference.taxonomy_terms`, jamais dans un `enum` Rust.

### IX. Erreurs d'API : code stable et message français

Toute erreur rendue par l'API porte un **code stable** — identifiant machine, jamais traduit, jamais
renommé sans amendement de version — et un **message en français** destiné à l'affichage. Le front ne
compose pas un message à partir du texte de l'erreur : il branche sur le code.

Une erreur de validation désigne le champ fautif. Aucune erreur ne divulgue l'existence d'une donnée
hors du périmètre de l'appelant : un identifiant inaccessible se refuse comme tel, il ne se distingue
pas d'un identifiant inexistant par la forme de la réponse.

### X. Tests d'intégration sur base réelle

Les tests d'intégration tournent sur une base **réelle et jetable**, chargée depuis `docs/database/`
dans l'ordre de numérotation des fichiers. **Aucun mock de base de données**, aucun double en mémoire :
la moitié des invariants de ce projet vit dans la base, un mock ne les porte pas et ferait passer au
vert un code que PostgreSQL refuserait.

Chaque module livre au minimum, sur cette base :

1. le chemin nominal de chaque route ;
2. un refus d'accès par périmètre, **URL forgée comprise** (principe V) ;
3. la traduction d'au moins une erreur d'invariant de la base en réponse d'API (principe VIII) ;
4. l'écriture des événements attendus dans `platform.outbox_events`, pour chaque changement d'état qui
   doit en produire (principe IV).

## Contraintes techniques

**Pile imposée** — Rust stable, Actix Web, SQLx, PostgreSQL 17 + pgvector, Garage (S3), Valkey. Aucune
dépendance nouvelle d'ampleur (framework, runtime, couche d'accès) sans décision consignée dans
`docs/progression/decisions/`.

**Taille des fichiers** — aucun fichier de `backend/` ni de `frontend/` ne dépasse **1000 lignes**.
C'est une limite haute, pas une cible : le découpage par écran ou par entité produit naturellement des
fichiers bien plus courts. `docs/database/` en est exclu — un fichier SQL de module est un tout
cohérent qui porte sa documentation.

**Deux sortes de textes multilingues, à ne jamais confondre** — les textes d'**interface** vivent dans
les fichiers i18n ; les **données métier** multilingues vivent dans des colonnes `platform.i18n_text`
et se résolvent par `platform.t(champ, locale)`, avec repli sur le français. Thématiques, catégories,
types d'organisation, titres d'activités, noms de journées spéciales, libellés de salles, intitulés de
critères et messages d'incident relèvent de la base. La règle qui tranche : *si un administrateur peut
le modifier depuis le back-office, ce n'est pas une traduction — c'est une donnée.*

**Commentaires** — très peu, et sur le *pourquoi* : une décision non évidente, un contournement, un
piège. Jamais sur le *quoi*. Un nom juste vaut mieux qu'un commentaire.

**Périmètre du jalon** — l'API ne couvre que ce qui permet de lancer l'appel à propositions de la
COP31 : identité, organisations, événements, propositions, sessions, média et engagement. Les modules
`publication`, `negotiation`, `training`, `tool` et la messagerie **existent dans le modèle** mais leur
interface affiche « En cours de maintenance », commandée par `platform.feature_flags`. Ils ne se
développent pas sans instruction explicite. Un drapeau de module (`<module>.enabled`) ne se confond
pas avec un drapeau fin (`negotiation.channels`, `tools.surveys`, `tools.ai_assistant`).

## Flux de travail et portes de qualité

**Le cycle Spec Kit** — un module = un cycle complet : `/speckit-specify` → `/speckit-clarify` →
`/speckit-plan` → `/speckit-tasks` → `/speckit-implement`. Les commandes s'appellent avec un tiret
depuis la 0.16. `/speckit-converge` relit le code livré et complète `tasks.md` ; `/speckit-analyze`
vérifie la cohérence entre `spec.md`, `plan.md` et `tasks.md`. L'ordre des modules est imposé par leurs
dépendances : B1 socle et identité, B2 organisations, B3 événements, B4 propositions, B5 sessions,
B6 média et engagement, B7 raccordement du front.

**La porte de qualité** — `make check` (`check-db`, `check-front`, `check-back`) DOIT passer avant tout
commit important. `check-db` détruit le volume et recharge le schéma de zéro, puis assure que les 16
schémas sont présents, que `cross_module_fk_report` ne contient aucune ligne non conforme, et que les
projections analytiques se rafraîchissent. `check-back` exécute la cible du `Makefile`, qui seule
définit ce que le portail passe : `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings` et `cargo test --workspace --all-features`. Les deux
options de `clippy` ne sont pas décoratives — sans elles, ni les tests d'intégration ni le seul fichier
qui compose du SQL dynamiquement ne sont analysés. Un avertissement Clippy est une erreur ; on ne
livre pas avec un `#[allow]` non justifié en commentaire.

**Le contrat d'API sert le front existant** — le front consomme des données simulées depuis le 16/08.
Les noms de champs de `frontend/app/types/` et `frontend/app/mocks/` ne se renégocient pas au moment de
l'API. Les écarts déjà consignés dans `docs/progression/ecrans/` et les obligations d'API relevées
dans `docs/progression/api.md` se traitent, ils ne se contournent pas.

**La progression se met à jour en fin de session** — journal du jour, fichier de l'écran ou du module,
décisions prises, `docs/progression/modele.md` si le SQL a bougé, et la ligne de suivi dans
`docs/PROGRESSION.md`. Une session qui ne le fait pas oblige la suivante à tout redécouvrir. C'est une
obligation de la même force que les portes techniques.

## Governance

**Autorité** — cette constitution prévaut sur toute pratique de développement de l'API. Elle ne prévaut
pas sur `docs/database/` (principe I) ni sur `CLAUDE.md`, dont elle dérive : une divergence constatée
avec l'un des deux se corrige **ici**, par amendement, jamais en écartant la source.

**Amendement** — un amendement exige trois choses : la modification de ce fichier, une entrée datée
dans `docs/progression/decisions/<date>.md` disant ce qui a changé et pourquoi, et la mise à jour du
numéro de version et de la date ci-dessous. Un principe non négociable ne se contourne pas au cas par
cas : soit il est amendé pour tous, soit il s'applique.

**Versionnage** — sémantique, sur cette constitution seule :

- **MAJEUR** — retrait ou redéfinition incompatible d'un principe, changement d'une règle de
  gouvernance qui invalide du code déjà livré.
- **MINEUR** — ajout d'un principe ou d'une section, extension matérielle d'une règle existante.
- **CORRECTIF** — clarification, reformulation, correction typographique, précision sans effet sur ce
  qui est autorisé ou interdit.

**Contrôle de conformité** — chaque revue vérifie les dix principes sur le périmètre modifié. Les
étapes `/speckit-plan` et `/speckit-analyze` relisent ce fichier et signalent tout écart avant
l'implémentation. Trois vérifications sont mécaniques et bloquantes, et le restent :
`cross_module_fk_report` vide, `make check` au vert, et le graphe de dépendances des crates sans arête
entre deux modules. Toute complexité qui semble exiger une entorse se justifie par écrit dans
`docs/progression/decisions/` — ou se règle autrement.

**Version**: 1.0.1 | **Ratified**: 2026-08-20 | **Last Amended**: 2026-08-20

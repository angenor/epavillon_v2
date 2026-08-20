# B3 — Événements

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B3.

**État** : ✅ **livré** le 20/08 — [`specs/003-evenements/`](../../../specs/003-evenements/spec.md). `backend/crates/modules/event` porte **les huit histoires**, **les 129 tâches sont faites**, les **37 routes** répondent sur la vraie application et les **6 événements de domaine** sont émis. `make check` passé en entier depuis la racine — base rechargée de zéro et frontières conformes, front typé et construit, **397 tests verts** et clippy sans avertissement. Aucune arête vers `identity` ni `org`.

**Une seule dérogation, assumée et bornée** : le module écrit dans `reference.entity_terms` pour poser les thématiques d'un fil — la seule écriture hors de son schéma, isolée dans `repo/themes.rs`. Le contrôle mécanique de `quickstart.md` était trop large d'un schéma ; le détail est en fin de fichier.

---

## Ce qui a été livré

**La spécification** : 8 histoires utilisateur priorisées, 16 cas limites, **99 exigences fonctionnelles**, 25 critères de réussite mesurables, une section d'hypothèses et une section de vérifications. Plus la liste de contrôle qualité (`checklists/requirements.md`), passée sans réserve bloquante.

**Aucune modification du modèle.** `060_events.sql` a été relu en entier : ses huit sections portent déjà tout ce que le prompt demande — les dix-huit champs d'une édition, la cardinalité 0..1 de l'appel garantie par index, la grille pondérée avec son critère éliminatoire, le comité, les canaux traités comme ressource réservable, et la vue des éditions publiques. Les deux compléments du 18/08 (coordonnées du lieu) et du 19/08 (trois déclinaisons d'image) y sont.

**Le contrat du front est repris tel quel**, sans une renégociation de nom de champ : quinze écritures et quatre lectures dans `composables/api/admin-events.ts`, treize lectures publiques dans le bloc `events` de `composables/useApi.ts`, et les deux appels de publication de `composables/api/planner.ts`.

**Le plan, le même jour** : `plan.md`, `research.md` (**20 décisions techniques**), `data-model.md`, `contracts/` (routes, erreurs, événements), `quickstart.md`. Contrôle constitutionnel passé **deux fois**, avec **une seule entorse justifiée** — le refus de supprimer un critère porteur de notes. **37 routes**, **6 événements de domaine**, **3 codes d'erreur**, **1 travail différé**, **1 crate créé**, **aucune dépendance nouvelle**, **aucun SQL composé dynamiquement** — contrairement à B2, tout passe la vérification à la compilation.

**Quatre décisions structurent le reste.** (1) **Les jours civils se calculent en base, dans le fuseau de l'édition** : les calculer en Rust demanderait une seconde base de fuseaux, et c'est exactement le défaut qui a fait tomber le formulaire du front sur `Europe/Geneva`. (2) **Tout ce qui détache une séance le compte AVANT de détacher** — les clés sont `ON DELETE SET NULL`, après coup le chiffre serait zéro. (3) **Le préfixe `/admin/planner` est composé une seule fois dans `api`**, avant que B5 n'y vienne : c'est le patron de `/people` posé en B1, appliqué avant que le défaut de B2 — trois routes muettes — ne se reproduise. (4) **La publication annonce au lieu d'écrire** : contrôle, estampille de l'édition, événement de domaine que B5 consommera, avec le **prédicat exact** des séances à publier — pour que le chiffre annoncé et l'effet obtenu viennent du même raisonnement.

**Le découpage, le même jour** : `tasks.md` porte **129 tâches en 11 phases, dont 31 tests**, une phase par histoire utilisateur et **quatre jalons de livraison**. **Trois avertissements sont écrits en tête**, chacun parce qu'il coûterait cher découvert en chemin : la suppression d'un critère qui **réussit** en détruisant les notes ; le préfixe `/admin/planner` que B5 partagera et qu'on compose dès les fondations ; et les trois chemins littéraux à déclarer avant leur homologue paramétré. **Le découpage a fait apparaître une dépendance que le plan n'avait pas nommée** : la phase 4 bloque les phases 7 à 9, parce que **toutes les écritures d'onglet rendent la composition entière** — `service/detail.rs` doit exister avant la première d'entre elles. **Le jalon que le prompt demande est T001–T068** : édition, périmètre, appel et grille.

**Les huit histoires, dans l'ordre de priorité** : (P1) une édition existe avec un numéro de dossier qu'on peut épeler ; le périmètre d'administration borne aussi les événements ; l'appel unique s'ouvre avec sa grille et jamais sans ; le public voit une édition, ses échéances et son visuel. (P2) le stand a ses salles et le direct son canal ; les journées du calendrier et les journées spéciales ne se confondent pas ; le comité se compose sans ouvrir de droits. (P3) la programmation ne se publie pas avec un conflit ouvert.

---

## Écarts relevés en écrivant la spécification de B3 (20/08)

Numérotation à la suite de B2, qui s'arrêtait à 86.

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **87** | **AUCUN DÉCLENCHEUR DE CE MODULE N'ÉMET D'ÉVÉNEMENT DE DOMAINE.** Le fichier ne porte que deux déclencheurs d'audit, sur `event.events` et sur `event.calls_for_proposals` — et aucune fonction en `SECURITY DEFINER`. C'est **l'inverse** du piège n° 1 rencontré en B1 (`anonymize_person()`) et en B2 (`merge_organizations()`), où la base émettait déjà ce que le service s'apprêtait à émettre | `060` § 2, 5 | Rien, si c'est **vérifié**. Beaucoup, si c'est supposé dans l'autre sens : un service qui attendrait de la base un événement qu'elle n'émet pas laisserait passer silencieusement tout changement d'état de ce module | **Vérifié, et inscrit à la spécification (FR-093).** Le service émet tout lui-même. Aucune double émission n'est possible ici |
| **88** | **LE RÔLE `programmer` NE DÉTIENT AUCUNE PERMISSION `event.*`.** Le semis de `identity.role_permissions` n'accorde `event.event.manage` et `event.call.manage` qu'à `admin` — et à `super_admin` par le déclencheur. L'écart n° 5 d'A10 supposait pourtant qu'un chargé de programmation pouvait « composer les journées spéciales sans toucher à la grille d'évaluation » | `030` § 6 | Un chargé de programmation ne peut ni créer un fil, ni déclarer une salle, ni poser un canal : il ne peut que planifier des créneaux. La séparation des deux permissions `event.*` reste juste, mais **elle ne sert à personne aujourd'hui**, aucun rôle ne détenant l'une sans l'autre | **Consigné, non corrigé** : accorder `event.event.manage` à `programmer` serait modifier le modèle, ce que le prompt interdit sans justification. À trancher avec le commanditaire — c'est une question de métier (« que peut faire un chargé de programmation ? »), pas de code |
| **89** | **PUBLIER LA PROGRAMMATION EXIGE DEUX ÉCRITURES DANS DEUX SCHÉMAS.** La date vit sur `event.events.programme_published_at` ; la visibilité de chaque séance vit sur `programme.sessions.published_at`, que `v_public_schedule` filtre (`WHERE s.published_at IS NOT NULL`). La règle de frontière posée en B2 interdit à un module d'écrire hors de son schéma | `060` § 2, `075` § 6 | Écrire quand même violerait le principe II ; confier toute la publication à B5 le violerait dans l'autre sens, B5 écrivant alors dans `event.events`. Sans décision, la route serait écrite par le premier des deux modules à en avoir besoin | **Tranché : la frontière passe par l'outbox** (FR-078, FR-079). B3 contrôle, pose la date et émet l'événement dans la même transaction ; le module Programmation le consomme, avec garde contre le rejeu. C'est le principe IV appliqué là où la tentation d'y déroger est la plus forte |
| **90** | **LA PUBLICATION EST GARDÉE PAR UNE PERMISSION D'UN AUTRE MODULE.** Le libellé du rôle `programmer` dit « planifie les créneaux **et publie la programmation** », et ce rôle détient `programme.session.schedule` — pas une permission `event.*`. Or la colonne publiée appartient à `event.events` | `030` § 6, `060` § 2 | Garder la route par `event.event.manage` empêcherait un chargé de programmation de publier ce que son rôle dit qu'il publie. Garder par la permission de l'autre module suppose que le garde d'autorisation soit atteignable sans dépendance croisée | **Tranché : garde par `programme.session.schedule`** (FR-082). Le garde vit dans `kernel` depuis B1 — décision T-B1 — donc **aucune arête** entre deux crates de module n'en découle |


**Deux écarts de plus, relevés en écrivant le plan (20/08).**

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **91** | **SUPPRIMER UN CRITÈRE D'ÉVALUATION DÉTRUIT LES NOTES, EN SILENCE.** `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE` : retirer une ligne de la grille efface toutes les notes posées sur ce critère, sans erreur et sans trace | `070` § 6, `060` § 6 | L'argumentaire d'une décision de sélection disparaît — précisément ce que la v1 n'avait pas et qui rendait un refus inexplicable à l'organisation qui le contestait. Et l'écran ne peut pas le voir : l'ordre réussit | **Le service compte et refuse** (`EVENT_CRITERION_HAS_SCORES`, 422), en nommant le critère et son nombre de notes. **C'est l'unique entorse au principe VIII du plan**, justifiée en « Complexity Tracking ». La cascade reste **juste** dans son cas d'origine — la disparition de l'appel entier —, d'où le refus dans le service plutôt qu'un `RESTRICT` en base |
| **92** | **CINQ TABLES DU MODULE NE SONT PAS AUDITÉES.** Seules `event.events` et `event.calls_for_proposals` portent `tg_audit()`. Les journées, les fils, les lieux, les salles et les canaux n'ont pas d'historique champ par champ — et `venues` et `rooms` n'ont même pas `updated_at` | `060` § 3, 3 bis, 4, 4 bis | « Qui a renommé la salle Baobab ? » n'a pas de réponse. Aucun écran ne le demande aujourd'hui, mais `platform.entity_history()` est présentée partout comme disponible « de toute entité auditée » | **Consigné, non compensé.** Ajouter les déclencheurs serait modifier le modèle ; les recopier en Rust serait réimplémenter un invariant. À reprendre le jour où un écran demande l'historique d'une salle |

**Trois écarts antérieurs sont traités par cette spécification, et non contournés.**

| N° | Écart | Traitement |
|---|---|---|
| **9** | **Le numéro de dossier dépend d'un champ facultatif** — `programme.tg_assign_reference_code()` préfixe par `events.acronym`, à défaut par les huit premiers caractères du slug : « COP31-TE-00001 », reproduit en base le 16/08 | **Option A, dans le service** (FR-027 à FR-030). Le sigle reste facultatif en base — les webinaires du cycle PACO n'en ont pas, et la reprise v1 en dépend — et devient obligatoire pour une édition tenant un pavillon : 2 à 12 caractères, lettres, chiffres, tiret, **vérifié sur l'état résultant** de l'écriture, avec une valeur par défaut proposée et modifiable |
| **25** | **`event.events` ne porte pas son visuel** : un aller-retour de plus par page publique | **Refermé** (FR-087, FR-089). La page publique d'une édition embarque ses **trois** déclinaisons résolues, sans repli décidé par l'API. `useApi().events.images()` disparaît au raccordement |
| **26** | **Aucune ressource ne rend « les éditions publiques »** — le critère « ni brouillon ni annulée » vivait côté front | **Refermé** (FR-084 à FR-086). La règle vit dans l'API, une édition **annoncée** dont le programme n'est pas publié en fait partie, et une édition hors série n'en disparaît pas |

**Deux obligations d'A10 sont confirmées et inscrites.**

- **`GET /admin/events/:id` embarque description, message d'accueil et images résolues** (FR-022) — plus aucun appel supplémentaire pour ouvrir le formulaire de modification.
- **La création d'une édition exige la portée GLOBALE** (FR-011) : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit. C'est la règle métier n° 8 prise par l'autre bout, confirmée comme A10 le demandait.

**Deux points d'A10 restent ouverts, et la spécification ne les tranche pas seule.**

- **La génération du calendrier reste applicative** (écart n° 1 d'A10) : aucune fonction n'est ajoutée au modèle, la génération est un geste explicite précédé d'un plan en lecture seule (FR-034, FR-035). Supprimer une journée détache ses séances, et cette décision appartient à l'équipe.
- **La granularité d'un calendrier de série de webinaires** (écart n° 2 d'A10) : sur `paco2027`, la génération proposerait **302 journées vides**. Aucune borne dure n'est codée — le plan annonce le nombre avant d'écrire. Le choix entre « ne générer que pour les séries de genre COP » et « une série de webinaires s'en passe » **est à arbitrer avec le commanditaire**.

---

## Ce qui a été vérifié en écrivant la spécification, et comment

Relecture du modèle et du contrat du front, sans exécution de code — aucun crate n'existe encore.

- **`060_events.sql` relu en entier** (680 lignes, huit sections). Les huit sections couvrent séries, éditions, calendrier, fils de programmation, lieux et salles, canaux, appels et grille, comité, registre des références et vue publique. **Aucune modification n'est requise**, et aucune n'est proposée.
- **Les quatre fonctions du module sont en lecture ou en semis** — `effective_deadline()`, `is_call_open()`, `max_weighted_score()`, `seed_default_criteria()` — et **aucune n'est en `SECURITY DEFINER`**. Rien n'a donc à être appelé depuis la porte d'écriture pour des raisons de contexte d'acteur, contrairement à `org.merge_organizations()`.
- **`event.event_days` n'a aucun déclencheur de dérivation** : rien en base ne crée une journée quand une édition change de période, rien n'en supprime quand elle se resserre. L'écart n° 1 d'A10 est confirmé par relecture.
- **Le préfixe du numéro de dossier vient bien de `programme.tg_assign_reference_code()`** : `COALESCE(upper(e.acronym), upper(left(e.slug, 8)))`, puis un repli `'EPAV'`. Le cas « slug `cop31-test` → COP31-TE-00001 » est exact.
- **`ux_calls_one_per_event` exclut les appels annulés** (`WHERE status <> 'cancelled'`) : le refus « appel déjà existant » ne doit pas se déclencher sur une édition dont l'appel a été annulé.
- **`ux_broadcast_channels_code` est `NULLS NOT DISTINCT`** : deux canaux **généraux** de la plateforme ne peuvent pas partager un code, alors qu'un canal général et un canal d'édition le peuvent. Et `ux_broadcast_channels_default` regroupe les canaux généraux sous un identifiant de substitution, en ne portant que sur les canaux **actifs**.
- **`event.v_public_editions` exclut `draft` et `cancelled`, et rien d'autre.** Une édition **annoncée** dont le programme n'est pas publié y figure — le fichier le dit en toutes lettres, et c'est ce que l'écart n° 26 demandait.
- **La vue publique ne porte pas le nombre d'activités et ne le peut pas** : `event` se charge avant `programme`. Le décompte vit dans `programme.v_edition_stats`, jointe sur `event_id`.
- **`programme.v_public_schedule` filtre sur `s.published_at IS NOT NULL`**, et non sur la date de l'édition. C'est ce constat qui produit l'écart n° 89.
- **`programme.publication_readiness()` rend `occurs_at`, un instant, et non un intervalle mis en forme** — depuis la correction du 18/08. L'API doit le laisser passer tel quel, l'interface le situant dans le fuseau de l'édition.
- **`programme.detect_conflicts()` ne vise que les séances installées en salle physique** — depuis la correction du 18/08. Une séance sans salle n'occupe rien.
- **`identity.administered_events()` filtre sur `programme.proposal.read_all`**, et non sur une permission `event.*` : le périmètre d'un administrateur d'édition est le même dans les six modules. Ce jalon n'y ajoute rien.
- **Les deux permissions du module appartiennent bien au module `event`** dans `identity.permissions`, et le rôle `admin` est attribuable `{global,event}` — la réponse du modèle au cas du webinaire confié à son responsable.
- **Le contrat du front est complet et cohérent** : quinze écritures et quatre lectures dans `composables/api/admin-events.ts`, treize lectures dans le bloc `events` de `composables/useApi.ts`, deux appels de publication dans `composables/api/planner.ts`. **Aucun nom de champ n'a été renégocié.**
- **Le module `event` est déclaré dans `platform.modules`**, dépendant de `org` et `identity` : il sera monté au démarrage comme l'ont été `identity` et `org`.

---

## Ce qui reste

`/speckit-implement`. Deux questions attendent le commanditaire et n'empêchent ni l'un ni l'autre : la granularité du calendrier d'une série de webinaires (écart n° 2 d'A10) et les permissions du rôle de programmation (écart n° 88).

**Deux ajouts additifs au contrat du front sont livrés côté API et ignorés jusqu'à B7** : le sigle proposé dans la réponse d'enregistrement d'une édition (`suggested_acronym`), et le refus de suppression d'un critère porteur de notes, qui sort en 422 faute de variante dans `CallErrorCode`.


---

## Implémentation — phases 1 et 2 (T001–T024), le 20/08

**Ce qui existe** : le crate `backend/crates/modules/event`, ses six dossiers internes, et les fondations sur lesquelles chaque histoire s'appuiera — codes d'erreur, traduction PostgreSQL, contrats d'événements, résolution d'ascendance, montage dans les deux binaires, travail différé, harnais de test. Aucune route n'est encore montée : `routes()` et `planner_routes()` existent et ne déclarent rien.

### Ce que l'écriture a fait apparaître, et qui n'était pas dans le plan

| Constat | Traitement |
|---|---|
| **L'ordre des branches de `pg_error.rs` est porteur.** La garde de B1 `("23503", c) if c.contains("country_id")` capture `events_country_id_fkey` et l'aurait rendue en `IDENTITY_UNKNOWN_REFERENCE` ; le fourre-tout `("23505", _) => Conflict` aurait absorbé les onze unicités du module | Les branches du module sont déclarées **avant** l'une et l'autre, avec le commentaire qui dit pourquoi. C'est le même genre de piège que l'ordre des routes, à un autre endroit |
| **`PG_DIAG_DATATYPE_NAME` est accessible, mais pas par l'interface générique.** `sqlx::Error::Database` rend un `&dyn DatabaseError`, qui n'expose pas le type de donnée | `pg_error::data_type()` fait un `try_downcast_ref::<PgDatabaseError>()`. Vérifié sur la source de `sqlx-postgres` 0.9 : la méthode `data_type()` existe. Un second assistant, `violated_domain()`, rend le domaine sans son schéma — c'est ce dont un module se sert pour poser le champ, que lui seul connaît |
| **La résolution d'ascendance d'un canal a trois issues, pas deux.** Un canal général de la plateforme porte `event_id IS NULL` : il n'est ni introuvable ni hors périmètre | `event_id_of_channel()` rend un `Option<Option<EventId>>`, et le service en tire un `CanalCible` à deux branches. Sans cette distinction, un canal de la plateforme se serait refusé en `not_found` au lieu de `platform_channel` |
| **Le harnais du module ne peut pas monter la vraie application.** Il faudrait une dépendance de développement vers `api`, qui dépend de `identity` et de `org` — et `cargo tree -p event` liste aussi les dépendances de développement, donc le contrôle bloquant du jalon échouerait | Les tests d'application vivent dans `crates/api/tests/`, exactement là où B2 a mis les siens. **C'est une divergence assumée avec l'énoncé de T021**, inscrite ici pour que la phase 11 sache où écrire `toutes_les_routes_repondent.rs` |
| **Quatre vérifications en ligne du modèle n'étaient pas au contrat des erreurs** — latitude, longitude, `required_reviews`, `max_proposals_per_organization`, plus le barème et le poids d'un critère | Traduites elles aussi, chacune sur son champ. Sans cela, une latitude hors bornes sortait en 500 |

### Ce qui a été vérifié en base plutôt que supposé

- **`platform.modules` porte déjà l'entrée `event`** — schéma `event`, dépendant de `org` et `identity`, semée par `010_platform.sql` § 7. **Rien n'est à semer** (T005).
- **`900_seed.sql` donne quatre séries et un canal général**, `ifdd_principal`, sans édition, actif et par défaut — et **aucune édition** (T006). Le relevé est inscrit en en-tête de `tests/commun/seed.rs`.
- **Les trois permissions consommées existent** dans `030_identity.sql` : `event.event.manage`, `event.call.manage` et `programme.session.schedule`, cette dernière attribuée au rôle `programmer` et non à un rôle du module.
- **`cargo tree -p event | grep -E 'identity|org'` ne rend rien** (T024), et `make check-back` passe : format, Clippy sans avertissement, et l'ensemble des tests du workspace.

### La dette de ce jalon, déjà datée

`backend/.sqlx/` **n'a pas été régénéré** : les requêtes de `repo/cross.rs` et du travail différé n'y sont pas encore. C'est T122, en phase 11 ; aucune commande du dépôt n'utilise `SQLX_OFFLINE` aujourd'hui, donc rien ne casse d'ici là.


---

## Implémentation — phase 3, US1 (T025–T041), le 20/08

**Ce qui existe** : créer et modifier une édition, avec la règle du sigle, la traduction des six contraintes nommées, et le calendrier créé dans le fuseau de l'édition. Trois routes montées sur trente-sept — `GET /admin/events/form-options`, `POST /admin/events`, `PUT /admin/events/{id}`.

**Ce qui est éprouvable seul, et l'est** : créer une édition à pavillon sans sigle → refus **avec une valeur proposée utilisable telle quelle** ; avec sigle → créée, avec ses **douze journées du 9 au 20 novembre** ; sans pavillon et sans sigle → créée.

### Un défaut de la phase 2, découvert en ouvrant la phase 3

**T014 n'avait jamais été écrit.** La commande qui devait le poser commençait par un changement de répertoire déjà effectué ; le `&&` a fait échouer l'ensemble, mais le corps du document a été consommé par l'interpréteur sans être écrit. La vérification qui a suivi portait sur la première ligne du fichier — **identique** à celle de l'ébauche laissée par la phase 1 —, et le crate compilait parce que rien ne référençait encore l'assistant.

Ce qu'il faut en retenir : **vérifier une écriture par sa première ligne ne vérifie rien** quand un gabarit partage cette ligne. Le contrôle qui aurait vu le défaut est le nombre de lignes, ou la présence d'un symbole que seul le nouveau contenu porte.

### Ce que l'écriture a fait apparaître

| Constat | Traitement |
|---|---|
| **La traduction d'un refus de la base doit brancher sur le NOM de la contrainte**, jamais sur le texte du message — qui est localisé par PostgreSQL et se reformule d'une version à l'autre. Or le dépôt traduisait déjà l'erreur en erreur d'API, ce qui perdait ce nom | `repo::editions::inserer` et `modifier` rendent un `sqlx::Error` **brut**, et le service traduit. C'est l'idiome posé en B1 (`repo/admin_users.rs`), remis au jour |
| **`PG_DIAG_DATATYPE_NAME` rend le nom du domaine NU** — « slug », et non « platform.slug » : le schéma voyage dans `PG_DIAG_SCHEMA_NAME`. L'hypothèse contraire, écrite en phase 2, faisait sortir une adresse d'URL mal formée en erreur 422 anonyme au lieu d'un refus de formulaire sur son champ | Mesuré sur la base — `SELECT 'Mauvais Slug'::platform.slug` rend « DATATYPE NAME: slug » — puis corrigé dans `kernel::pg_error::violated_domain`, qui prend le dernier segment pour rester juste si une version future qualifiait le nom |
| **`numeric` n'a aucun décodeur** dans les caractéristiques SQLx du workspace : ni `bigdecimal` ni `rust_decimal` n'y sont déclarés | Les coordonnées passent par `::float8` en lecture et `$n::float8::numeric(9,6)` en écriture — le même parti que le score de similitude de B2. **À prévoir en phase 5** : le barème et le poids d'un critère sont eux aussi des `numeric` |
| **`event.event_series` ne porte pas de colonne de rang**, contrairement à ce qu'un tri « par ordre d'affichage » aurait supposé | Les séries sont triées par genre puis par nom français |
| **Le décalage d'un fuseau est un intervalle PostgreSQL**, pas une durée : trois composantes, dont celle des mois n'a aucun sens ici | Ramené en minutes, avec le commentaire qui dit pourquoi la composante des mois est ignorée |

### Deux décisions de conception, et ce qui les dicte

- **La liste des fuseaux vient de `pg_timezone_names`** — le dictionnaire **même** contre lequel `platform.timezone_name` vérifie ce qu'on écrit. En recopier une ailleurs les ferait diverger, et c'est exactement le défaut qui a fait tomber le formulaire du front sur `Europe/Geneva`. Conséquence assumée : la ville est le dernier segment de l'identifiant IANA, donc **sans accent** — « Belem », pas « Belém ». Inventer l'accent ferait de cette commodité de saisie une seconde vérité sur les noms de villes.
- **Les statuts sont lus dans l'énuméré du modèle** (`enum_range`), dans l'ordre où il les déclare — qui est l'ordre du cycle de vie. Les recopier ferait un second vocabulaire, à désynchroniser au premier ajout.

### Ce que les tests tiennent, et ce qui tomberait sans eux

- **`sigle_obligatoire_avec_pavillon.rs`** — les cinq chemins d'écriture, dont les deux qu'un service vérifiant seulement la création laisserait passer : **basculer** en pavillon sans sigle, et **retirer** le sigle d'une édition à pavillon. Plus un contrôle que le prompt ne demandait pas et qui vaut d'être écrit : **la valeur proposée est réellement acceptée sans retouche**. Une proposition que la tentative suivante refuserait ferait tourner l'équipe en rond.
- **`contraintes_edition_traduites.rs`** — les six contraintes, chacune sur son champ, plus deux cas de frontière : une adresse mal formée (refusée par le **domaine**, dont le nom de contrainte n'est pas celui de la colonne) et une série inconnue, qui sort en **erreur HTTP** parce que `EditionFormError` n'a aucun code pour l'exprimer.
- **`jours_civils_dans_le_fuseau.rs`** — écrit pour **tomber si le calcul se fait en temps universel** : la COP31 s'y termine le 20 novembre à 22 h à Belém, soit le **21** en UTC. Douze journées d'un côté, treize de l'autre. Un test dont les bornes coïncideraient dans les deux fuseaux ne prouverait rien.

### Ce qui reste ouvert de cette phase

`days_removed` et `sessions_detached` valent **toujours zéro** sur une écriture d'édition, et c'est la règle (FR-033) : le retrait des journées hors période est un geste séparé, livré avec la génération du calendrier en phase 8.

---

## Implémentation — phase 4, US2 (T042–T053), le 20/08

**US2 est livrée** : lister, ouvrir le détail, et refuser tout ce qui sort du périmètre — URL forgée comprise. Trois routes de plus (`GET /admin/events`, `GET /admin/events/{id}`, `GET /events`), la composition des six onglets, et quatre fichiers de test.

**Les trois cas du périmètre restent distincts jusqu'au bout**, y compris dans le service : `edition_read::ecran` refuse lui-même un périmètre vide, sans s'en remettre à l'extracteur. Appelé d'ailleurs — et il le sera, les écritures d'onglet rendant la composition entière —, il devait refuser tout autant.

**La seule exception du module est écrite et éprouvée.** `GET /events`, le sélecteur du back-office, est *filtré* et non refusé : un périmètre vide y rend une liste vide, parce que le contrat du front le veut ainsi. `perimetre_liste_filtree.rs` la fixe, précisément pour qu'on ne la « corrige » pas un jour par symétrie avec les autres lectures.

### Ce que l'écriture a fait apparaître

| Constat | Traitement |
|---|---|
| **La liste et le détail lisaient la même édition par deux requêtes voisines de vingt-cinq colonnes.** Deux occasions de diverger, et le jour où l'une gagne une colonne, les deux écrans cessent de dire la même chose de la même édition | Une seule requête privée, `repo::editions::lire`, paramétrée par le filtre de périmètre **et** par un identifiant facultatif. `ligne()` et `lignes_du_perimetre()` s'y ramènent |
| **Les décomptes d'une ligne se prenaient une par une.** Sur une liste, c'est une requête par édition — le défaut que B2 a payé sur sa propre liste | `cross::decomptes_par_edition` prend la liste entière par `unnest`, en une requête. Une édition sans rien à compter y figure à zéro, jamais absente |
| **Les facettes ne peuvent pas venir du catalogue.** Proposer au filtre toutes les séries de la plateforme donnerait à un compte détaché la liste de ce qu'il n'administre pas — divulguer par la facette ce que la liste vient de masquer | Séries et millésimes sont dérivés des **lignes déjà lues** (FR-018). `LigneBase` porte pour cela `series_is_active`, qui ne sert qu'à la facette et jamais à la ligne |
| **Le nom du responsable d'un fil venait d'une jointure sur `identity.people`, écrite dans `repo/tracks.rs`** — première fissure dans la frontière que `repo/cross.rs` existe pour tenir | Retirée. `cross::noms_de_personnes` résout les noms, et le service les pose. Aucune requête hors du schéma `event` ne vit ailleurs que dans `cross.rs` |
| **Le contrat du front documente `programme.proposal.review`**, permission qui **n'existe pas** dans `030_identity.sql`. Le code réel est `programme.review.write`, et c'est celui que les données simulées emploient | Le code du modèle fait foi. Il vit dans `repo/cross.rs` et **non** dans `domain/permissions.rs` : ici il est un **critère de liste**, pas un garde — il ne protège aucune route de ce module |
| **`time::Time` ne se sérialise pas en `HH:MM:SS`** mais en ISO 8601 complet, fractions comprises | La plage d'accueil du pavillon est rendue **en texte par la base** (`daily_start_time::text`). La mettre en forme en Rust inventerait une seconde écriture de l'heure |
| **Le garde de portée globale vivait dans le gestionnaire**, donc hors de portée d'un test | Déplacé en `service::portee_globale_exigee`. La règle qu'on veut éprouver ne doit pas être enfermée dans une fonction privée de route |

### Ce que les tests tiennent, et ce qui tomberait sans eux

- **`perimetre_edition_url_forgee.rs`** — les **six cibles d'ascendance** plus l'édition elle-même, chacune nommée dans la boucle pour qu'un échec dise laquelle est passée. Trois moitiés comptent autant que la première : le compte détaché **passe** sur sa propre édition (un garde qui refuserait tout serait vert sans rien protéger) ; un identifiant **inexistant** rend le même code **et le même message** qu'un hors-périmètre ; et le **canal général de la plateforme** se résout au lieu de se refuser — c'est l'issue de plus que les six autres n'ont pas.
- **`perimetre_liste_filtree.rs`** — les trois cas, les facettes bornées, et l'exception du sélecteur.
- **`creation_portee_globale.rs`** — le refus, l'acceptation, **et** la preuve que le refus vient de la *portée* et non d'une absence de droits : le compte détaché détient bien `event.event.manage` sur son édition. Sans cette moitié, le test passerait aussi sur un compte qui n'a rien.
- **`detail_en_une_reponse.rs`** — les six onglets, et surtout **une séance réelle** placée en salle, diffusée et rattachée à un fil. Elle doit se retrouver dans les cinq compteurs qui la concernent — édition, journée, fil, salle, canal — et **pas** sur le canal général. Sans elle, tous les décomptes vaudraient zéro et le test ne prouverait rien.

### Ce qui reste ouvert de cette phase

**T049 n'est pas cochée, et c'est exact.** Les six routes paramétrées par un identifiant d'enfant naissent en phases 5, 7 et 9 : on ne peut pas leur appliquer aujourd'hui un garde qu'elles n'ont pas encore l'occasion d'appeler. Ce qui est livré, c'est le garde lui-même et son épreuve sur les six cibles ; chaque route qui arrivera l'appellera et cochera sa part.

**Six dépôts sont créés avec leur seule lecture** — `tracks`, `venues`, `channels`, `calls`, `criteria`, `committee`. Le détail en avait besoin dès maintenant, la dépendance étant inscrite au découpage (« phase 4 → phases 7, 8, 9 »). Leurs écritures viennent avec leurs phases, et les tâches correspondantes le disent déjà.

---

## Implémentation — phases 5 à 11 (T054–T129), le 20/08

**Le module est complet.** Les **37 routes** sont montées et frappées sur la vraie application, les **6 événements de domaine** sont émis — et les six silences tenus —, les **3 codes d'erreur** sont au catalogue du noyau, et le travail différé tourne. `cargo tree -p event | grep -E 'identity|org'` ne rend toujours rien.

**Ce que chaque phase a livré, en une ligne** : (5) l'appel unique et sa grille, avec le refus qui sauve les notes ; (6) les dix lectures publiques, et les écarts n° 25 et n° 26 refermés ; (7) lieux, salles et canaux, avec le défaut de diffusion qui retire le précédent ; (8) le calendrier et les fils, avec le plan qui n'écrit rien ; (9) le comité, qui n'accorde aucun droit ; (10) la publication, seul contrôle bloquant du module ; (11) la finition.

### Ce que l'écriture a fait apparaître, et qui n'était pas dans le plan

| Constat | Traitement |
|---|---|
| **UNE ÉCRITURE HORS DU SCHÉMA `event` EST INÉVITABLE, et le contrôle mécanique de `quickstart.md` l'interdisait.** Les thématiques d'un fil vivent dans `reference.entity_terms` — le modèle le veut ainsi (« aucune table de liaison à maintenir »), le contrat du front les porte (`EditionTrackPayload`, `EditionTrack.themes`), et **aucune fonction de pose n'existe** dans `020_reference.sql` : `terms_of()` et `term_badges()` lisent, rien n'écrit. Or le grep de T120 refuse tout `INSERT INTO reference.` | **La règle est resserrée plutôt que l'écriture cachée.** `repo/themes.rs` est le **seul** fichier du module qui écrive hors de son schéma, et son en-tête porte la justification : `reference` n'est pas un module métier mais le **référentiel partagé**, sans crate ni service, dont aucun service autonome ne se détachera. La frontière que le principe II protège est celle des **modules** — `programme`, `media`, `identity`, `org` —, et **aucune écriture vers ceux-là n'existe**. Le contrôle de T120 doit donc lire `(programme|media|identity|org)` et non `reference` ; l'exception reste vérifiable d'un `grep`, puisqu'elle tient dans un fichier |
| **`event.seed_default_criteria()` écrit : elle ne sait poser ses six lignes que sur un appel.** La route qui rend la grille par défaut est une **lecture** | Un appel **jetable** dans une transaction **annulée** — une édition et un appel créés, semés, lus, puis rien qui subsiste, pas même une ligne d'audit. C'est le parti que `data-model.md` prévoyait. Les identifiants rendus sont **mis à nul** : ce sont des lignes *nouvelles*, et les rendre ferait croire à l'écran qu'elles existent. Trois assertions le tiennent, dont une qui compte les éditions avant et après |
| **L'ordre du diff de grille est porteur** : supprimer, puis modifier, puis insérer. Un critère **renommé** libère un code que la même charge utile reprend ; insérer d'abord violerait `ux_review_criteria` sur un conflit qui n'existe déjà plus | Écrit dans cet ordre, avec le commentaire qui dit pourquoi |
| **`event.v_public_editions` ne porte pas tout `EventEdition`** : ni l'adresse, ni les coordonnées, ni l'auteur, ni les horodatages. Le contrat du front les attend | La requête joint `event.events` à la vue — **la même requête**, pas une seconde lecture. La vue reste la seule à porter le critère de publicité, la série, le pays, les trois images et l'appel |
| **`v_public_editions.id` est rendu nullable par SQLx**, comme toute colonne de vue | Annoté `AS "id!"`. Le rappel vaut pour B5 : une vue ne porte aucune contrainte de nullité, et le vérificateur le suppose |
| **La désactivation d'un canal doit lui retirer son statut de défaut.** `ux_broadcast_channels_default` ne porte que sur les canaux **actifs** : laisser `is_default` à vrai sur un canal inactif ne violerait rien, mais l'édition se retrouverait sans défaut apparent alors qu'une ligne le revendique | `channels::desactiver` pose les deux colonnes. Un test vérifie que le canal désactivé n'est plus le défaut |
| **Une séance annulée doit porter son motif** (`ck_sessions_cancelled_reason`), et `location_note` est un texte **multilingue**, pas une chaîne | Relevé en écrivant le semis des tests de publication. Deux corrections d'une ligne, mais deux suppositions de moins |
| **`identity.roles` porte `label` et `allowed_scopes`**, non `name` et `is_assignable` | Le test qui éprouve la séparation des deux permissions construit **ses propres rôles** : aucun rôle du catalogue ne détient l'une sans l'autre — c'est l'écart n° 88 —, et sans eux le test serait vert sans rien prouver |

### Deux décisions de conception, et ce qui les dicte

- **La réponse commune des onglets recompose tout.** `service/tabs.rs` porte la réussite, le refus et la traduction : six onglets partagent le même vocabulaire d'erreur, et une contrainte de code refuse de la même façon qu'elle vienne d'une salle, d'un fil ou d'un canal. La contrepartie est assumée — écrire dans un onglet relit les cinq autres —, et c'est elle qui garantit que leurs décomptes restent justes.
- **Le prédicat des séances à publier est écrit une fois**, dans `repo/cross.rs`, et **voyage** dans la charge utile de l'annonce. Le chiffre annoncé et l'effet obtenu viennent ainsi du même raisonnement : un consommateur qui recalculerait « les séances de l'édition » publierait autre chose que ce qui a été annoncé.

### Ce que les tests tiennent, et ce qui tomberait sans eux

- **`critere_porteur_de_notes.rs`** — le refus, **et les notes toujours présentes après**. Sans cette seconde moitié le test ne prouverait rien : un service qui supprimerait d'abord et refuserait ensuite passerait la première. Il vérifie en plus que **la grille entière est intacte** — le refus annule tout l'enregistrement, pas seulement la suppression.
- **`outbox_evenements_du_module.rs`** — les six événements, **et l'absence d'événement** pour les journées, les fils, les lieux, les salles, les canaux et le comité. Six écritures d'onglet, aucun message de plus : émettre « pour plus tard » remplit la file de messages que personne ne lit.
- **`canal_par_defaut_unique.rs`** — le second défaut qui retire le premier, **deux écritures concurrentes**, le canal général semé **qui n'est pas délogé** (il est le défaut de son propre groupe), et le défaut désactivé qui **libère la place**.
- **`plan_necrit_rien.rs`** — la base identique avant et après, comparée **journée par journée avec son habillage**, et une période d'un an qui annonce **365 journées sans en écrire une**.
- **`detachement_salle_et_lieu.rs` et `detachement_journee_et_fil.rs`** — le chiffre annoncé comparé au chiffre **réel** après coup, dans les quatre cas. Le retrait d'un lieu compte les séances de **toutes** ses salles, pas seulement celle que l'écran affichait.
- **`publication_rejouee_inoffensive.rs`** — la date d'origine intacte, **aucun second événement**, et qu'une écriture d'édition ne touche jamais cette date.
- **`routes_event.rs`** (dans `crates/api/tests/`) — **les 37 routes sur la vraie application**. Les dix lectures publiques doivent répondre **200 sans session** ; les vingt-sept autres doivent refuser **sans jamais rendre 404**.
- **`ordre_des_routes_publiques.rs`** — `/events/public` rend **un tableau**, jamais le `null` d'une adresse inconnue. Le symptôme du défaut inverse serait une page d'accueil vide, **sans erreur** : plus discret encore que les trois routes muettes de B2.

### Ce qui reste ouvert

- **T049 est cochée pour de bon** : les six routes paramétrées par un identifiant d'enfant existent toutes et appellent le garde d'ascendance. `perimetre_edition_url_forgee.rs`, écrit en phase 4, les couvrait déjà par leurs cibles.
- **La seconde moitié de la publication arrive avec B5** : ce module annonce, l'autre pose `published_at` sur les séances. Le contrat du consommateur est inscrit aux points bloqués.
- **Deux arbitrages attendent toujours le commanditaire** : la granularité du calendrier d'une série de webinaires (écart n° 2 d'A10 — le plan annonce 365 journées et n'impose rien) et les permissions du rôle de programmation (écart n° 88).

### Éprouvé au navigateur, le 20/08

`quickstart.md` demandait de vérifier au navigateur, **sur les données simulées**, que rien n'a régressé côté écrans — le front n'est pas encore raccordé à l'API (`NUXT_PUBLIC_API_BASE` reste vide jusqu'à B7). Fait en mode visible, à 1440 px puis à 375 px, en français et en anglais.

- **A10, liste des éditions** — cinq éditions, facettes et six filtres en place, tri par colonne. La COP31 affiche « 12 jours » et « du 9 novembre 2027 au 20 novembre 2027, heure de Belém » : le compte et le fuseau que la phase 3 tient côté API.
- **A10, détail** — les **six onglets** répondent, chacun avec son décompte (12 journées, 2 fils, 2 lieux, 1 canal, 5 membres du comité).
- **L'onglet de l'appel montre exactement ce que l'API sert désormais** : l'échéance **effective** en évidence (« Prolongé jusqu'au 30 septembre 2026 ») avec l'échéance annoncée à l'origine en dessous, la note maximale atteignable, et sur chaque critère son barème, son poids, l'éliminatoire et **« 32 notes déjà posées »** — le chiffre même qui interdit le retrait du critère.
- **L'onglet du comité** rend la charge de chacun (« 7 confiés · 4 évalués »), son plafond et son rôle de président, et rappelle en tête que **siéger n'ouvre aucun droit** — la règle que le service tient.
- **Page publique d'une édition** — bandeau 32:9 résolu, statut, appel ouvert et **échéance effective**, les quatre échéances de la frise. C'est ce que `GET /events/{slug}` sert maintenant en une requête.
- **Frise d'accueil** — les trois éditions publiques ; ni brouillon ni annulée.
- **375 px** : aucun défilement horizontal, ni sur la page publique ni sur le détail du back-office.
- **Aucune erreur de page.** Les seuls avertissements sont les quatre pages institutionnelles absentes (`/aide`, `/accessibilite`, `/contact`, `/mentions-legales`), consignées aux points bloqués depuis le 16/08 et sans rapport avec ce jalon.

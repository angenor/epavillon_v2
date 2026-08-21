# Implementation Plan: Sessions (B5)

**Branch**: `005-sessions` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-sessions/spec.md`

---

## Summary

Étendre `backend/crates/modules/programme` — **aucun crate n'est créé** — pour transformer une décision en programme : la séance naît de l'acceptation, l'équipe l'arbitre, la publication la rend publique, et le public s'y inscrit.

L'approche tient en huit points, tous déduits du modèle, du contrat du front, ou d'un précédent livré :

1. **Le service n'émet aucun événement, parce que les DEUX déclencheurs du fichier émettent déjà.** Séance et inscription : le piège de B1, B2 et B4, en double. Vérifié dans le corps des fonctions (R2, écart n° 117).
2. **La naissance des séances s'accroche à UN SEUL point** — `service/transition.rs::tenter` —, parce que les deux chemins d'acceptation y passent, et la reprise v1 n'écrit pas l'état (R3, écart n° 57).
3. **L'écart n° 7 est traité dans son intention, pas à la lettre.** Le canal de diffusion **est** saisissable : le déclencheur ne le pose que lorsqu'il est nul. Le refuser aurait cassé une fonctionnalité livrée du planificateur (R8, écart n° 111).
4. **La journée de rattachement se remet à nul quand le créneau change**, pour que la base la redéduise. Sans cela, une séance déplacée reste rangée au mauvais jour, en silence — et déplacer est le geste le plus fréquent de l'écran (R9, écart n° 113).
5. **La publication pose la date ET fait passer « pressenti » à « programmé »**, en un seul ordre, le déclencheur filtrant lui-même les lignes inchangées. Trois preuves concordantes le demandent ; ne poser que la date laisserait un état du modèle mort (R12).
6. **Le module est le premier consommateur d'outbox du dépôt.** La machinerie du noyau n'avait jamais servi qu'à la télémétrie ; elle porte la garde de rejeu, que le module n'a donc pas à écrire (R13).
7. **La validation des réponses est dynamique et PURE**, contre le formulaire **résolu** — séance, édition, plateforme. Le déclencheur ne vérifie rien lorsque la séance ne porte pas de formulaire attaché (R15, écart n° 114).
8. **Toute écriture d'inscription prend la ligne de la séance en verrou.** Le contrôle de jauge de la base compte sans verrou : deux inscriptions simultanées la dépassent, et deux personnes reçoivent le même rang d'attente (R19, écart n° 124 — nouveau).

---

## Technical Context

**Language/Version** : Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies** : **une déclaration nouvelle, et une seule** — `regex`, pour honorer la règle `pattern` d'un champ de formulaire. Ce n'est ni un framework, ni un runtime, ni une couche d'accès, et **elle est déjà dans l'arbre de compilation** (1.13.1, dépendance transitive du workspace) : la déclarer au workspace ne fait rien entrer de neuf. Décision consignée dans `docs/progression/decisions/2026-08-21.md`, comme la constitution l'exige (R27). Tout le reste est déjà déclaré. **Aucun SQL composé dynamiquement.**

**Storage** : PostgreSQL 17 + pgvector — **schéma existant, non modifié**. Valkey toujours inutilisé. Garage hors périmètre : la couverture d'une séance est résolue **en base** par `media.attached_image()`, ce module ne dépose aucun objet.

**Testing** : `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock. Le semis ne fournit aucune séance : chaque test enchaîne édition, jours, salles, canal, appel, organisation, personne, dossier déposé et évalué — le parcours le plus long du jalon, et le seul qui traverse les cinq modules.

**Target Platform** : serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type** : service web (API) adossé à un front Nuxt existant. `api` monte les routes et **compose le préfixe `/admin/planner`** à partir de deux modules ; **`worker` est modifié** — pour la première fois autrement que par un travail différé : il enregistre un **consommateur d'outbox**.

**Performance Goals** : **aucune cible chiffrée, sauf une garantie de justesse sous concurrence.** Cent inscriptions simultanées sur une séance de dix places doivent produire exactement dix confirmées et des rangs d'attente sans doublon (SC-024) — ce n'est pas une cible de débit, c'est une correction. Deux points méritent d'être surveillés : l'écran du planificateur (sept lectures, traitées par une transaction en lecture seule sur **une** connexion — la leçon de B2), et `detect_conflicts()`, dont les cinq branches croisent les séances actives deux à deux, sur quelques dizaines de lignes par édition.

**Constraints** : `DATABASE_URL` renseignée et base démarrée pour compiler · **aucun fichier de `backend/` au-dessus de 1000 lignes — c'est le module où la marge est la plus mince**, le crate passant d'environ 17 400 à 25 000 lignes · aucun avertissement Clippy · aucune arête entre deux crates de module · les chemins et noms de champs sont **exactement** ceux que `composables/api/planner.ts` et les blocs `sessions` et `registrations` de `useApi.ts` consomment déjà · **aucune modification de `docs/database/`**, ce que ce plan ne propose nulle part.

**Scale/Scope** : **17 routes HTTP**, 0 crate créé, **0 travail différé**, **0 événement émis par le service** (quatorze types le sont par la base), **1 événement consommé** — le premier du dépôt —, **8 codes d'erreur** ajoutés au catalogue, **1 écriture hors schéma nouvelle**, **1 addition au noyau** (`ConsumerRegistry::register_all`), **1 refactorisation de préfixe** touchant du code livré, **4 formes ajoutées** au contrat du front. Volumétrie de référence : quelques dizaines de séances par édition, quelques centaines d'inscriptions par séance.

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `075_programme_sessions.sql` relu intégralement (1 137 lignes, neuf sections), plus `060` § 3, 3 bis, 4, 4 bis, 5, `070` § 2, 5, 6, `030` § 3 et 5, `020` § 4, `050` § 8, `010`. **Aucune modification proposée** | ✅ La conception a **plié devant le modèle** six fois, et l'a corrigé une : le service n'émet rien (R2) ; l'idempotence de la naissance repose sur une contrainte plutôt que sur un décompte (R6) ; le fuseau se convertit **en base** (R4) ; la bascule en liste d'attente est **laissée faire** (R20) ; le porteur de séance n'est **jamais** écrit par le service (data-model § 1) ; le repli de créneau existe parce que la colonne est `NOT NULL`. **Et la lecture a corrigé la consigne du prompt** : l'écart n° 7 se trompe sur le canal de diffusion (R8, écart n° 111) |
| II | Frontières de modules | ✅ Aucun crate créé : `programme` porte déjà tout le schéma, décision de B4 (R1). Dépendance au noyau et aux contrats, à rien d'autre | ⚠️ **Une écriture hors schéma nouvelle** — `identity.consents` —, bornée à un fichier et justifiée en « Complexity Tracking ». Les deux autres sont celles de B4, réemployées : `reference.entity_terms` (une seconde entité) et `identity.people` (inchangé). **Le noyau gagne cinq lignes** (`register_all`), ce qui est son rôle : il est partagé, pas contourné. `cargo tree -p programme` doit rester sans arête |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée — aucun DDL | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` dans la transaction, jamais d'insertion à la main | ✅ Tenu **dans les deux sens, et c'est la nouveauté du module**. En émission : le service n'émet **rien**, les deux déclencheurs émettant déjà quatorze types (R2) ; un test compte trois lignes d'outbox après une acceptation à trois séances, pas six. En consommation : **le premier consommateur du dépôt**, avec la garde de rejeu portée par le noyau, le prédicat de l'annonce appliqué tel quel, et aucune écriture sur l'édition (R13) |
| V | Autorisation par permission et par portée | ✅ Deux permissions du modèle, testées séparément par le garde du noyau | ✅ Trois décisions : toute route paramétrée **remonte à l'édition de la séance avant** de vérifier le périmètre — jamais l'édition annoncée par le client ; les trois cas du périmètre restent distincts, **aucun droit → refus explicite** ; et la liste nominative des inscrits exige une permission que le rôle de programmation **ne détient pas** (écart n° 119) — le modèle est suivi, pas contourné |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ✅ **Aucune exception.** Deux traversées ajoutées, toutes deux triviales : `tstzrange` lu en `text` — la représentation que le contrat du front déclare —, et les trois colonnes `jsonb` libres du formulaire, qui **doivent** rester libres (écart n° 6). Les colonnes des deux vues s'annotent une à une : une vue ne porte aucune contrainte de nullité (leçon de B3) |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu, **avec un acteur de fond assumé** : la publication est écrite par le relais d'outbox, dont le contexte est `background("outbox")`. L'audit porte donc une étiquette et non une personne — ce qui est exact, personne ne publie une séance à la main. Toutes les autres écritures portent l'acteur de la session |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Quatorze garanties nommées identifiées, aucune recopiée | ⚠️ **Une entorse, justifiée en « Complexity Tracking »** : le service revérifie la **présence** des réponses obligatoires que le déclencheur vérifie déjà. **Quinze autres règles vivent dans le service et n'en sont pas** : la base ne les porte pas du tout — data-model § 3 les liste une à une avec leur écart. Et **une catégorie nouvelle apparaît** : deux garanties de la base existent mais **ne tiennent pas sous concurrence** (jauge, rang d'attente). Les rendre sûres n'est pas les réimplémenter — c'est poser le verrou qui leur manque (R19) |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ **Huit codes ajoutés**, et pas plus : cinq refus d'inscription sont des membres d'union et sortent en **200** avec leur valeur, relue en base et **jamais extraite d'une phrase française**. Les deux codes PostgreSQL du déclencheur d'inscription servent à deux gestes et se distinguent **par le geste, jamais par le texte** — la règle de B4 |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations ont chacune leur test nommé, plus quatre qui ne se déduisent d'aucune : **un test qui frappe les dix-sept routes** sur la vraie application ; **un test de concurrence** à cent inscriptions ; **un test de bout en bout de la publication**, comparant l'annonce à l'effet (obligation inscrite aux points bloqués le 20/08) ; et **un balayage de charge utile** prouvant qu'aucun nom d'inscrit ne sort vers l'organisation |

**Verdict** : une entorse au principe VIII, justifiée ci-dessous ; une écriture hors schéma nouvelle, bornée et justifiée ; une addition de cinq lignes au noyau, qui est son emploi normal. Aucune autre.

**Six points ont été tranchés dans `research.md`** plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une faute **à l'exécution** : la double émission d'événements (R2), qui enverrait tout en double ; la mise à nul de la journée de rattachement (R9), sans laquelle une séance déplacée est rangée au mauvais jour ; la validation contre le formulaire **résolu** (R15), sans laquelle une inscription passe sans réponse obligatoire ; le verrou de ligne (R19), sans lequel la jauge se dépasse ; le passage en « programmé » à la publication (R12), sans lequel un état du modèle reste mort ; et le refus **partiel** des champs dérivés (R8), dont la version littérale aurait cassé une fonctionnalité livrée.

---

## Project Structure

### Documentation (this feature)

```text
specs/005-sessions/
├── spec.md              # Ce qu'il faut faire (/speckit-specify)
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 27 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les 17 routes, leur autorisation, les 4 formes ajoutées
│   ├── errors.md        #   les 8 codes stables et la traduction PostgreSQL
│   └── events.md        #   ce que la base émet, ce que le service n'émet pas, ce qu'il consomme
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
backend/
├── Cargo.toml                          # + une déclaration : regex (déjà dans l'arbre)
└── crates/
    ├── kernel/
    │   ├── error.rs                    # MODIFIÉ — + 8 variantes (SESSION_*, REGISTRATION_*)
    │   └── events.rs                   # MODIFIÉ — + ConsumerRegistry::register_all() (R13)
    ├── contracts/
    │   └── programme.rs                # INCHANGÉ — le service n'émet rien, c'est la décision
    ├── modules/
    │   ├── identity/ · org/            # INCHANGÉS
    │   ├── event/                      # INCHANGÉ — il publie et annonce déjà
    │   └── programme/                  # ÉTENDU, jamais recréé
    │       ├── domain/
    │       │   ├── ids.rs               # MODIFIÉ — + SessionId, RegistrationId, RoomId,
    │       │   │                        #   TrackId, ChannelId, EventDayId, FormId
    │       │   ├── sessions.rs          # NOUVEAU — formes de séance, états, décomptes
    │       │   ├── derived.rs           # NOUVEAU — les quatre champs dérivés et leur régime (R8)
    │       │   ├── birth.rs             # NOUVEAU — créneau, durée et repli d'une séance naissante
    │       │   ├── answers.rs           # NOUVEAU — la validation dynamique, PURE (R15, R16, R17)
    │       │   └── registration.rs      # NOUVEAU — les six issues, et ce qui les décide
    │       ├── repo/
    │       │   ├── cross/mod.rs         # MODIFIÉ — + salles, jours, fils, canaux ;
    │       │   │                        #   + starts_at et programme_published_at sur l'édition
    │       │   ├── themes.rs            # MODIFIÉ — une seconde entité, `sessions`
    │       │   ├── people.rs            # INCHANGÉ — réemployé pour l'inscrit sans compte
    │       │   ├── consents.rs          # NOUVEAU — ÉCRITURE HORS SCHÉMA n° 3, bornée (R22)
    │       │   ├── sessions.rs          # NOUVEAU — lecture et écriture des séances
    │       │   ├── session_parts.rs     # NOUVEAU — intervenants, organisations, journées
    │       │   ├── conflicts.rs         # NOUVEAU — detect_conflicts(), telle quelle
    │       │   ├── planner.rs           # NOUVEAU — les sept lectures de l'écran (R10)
    │       │   ├── public_schedule.rs   # NOUVEAU — v_public_schedule, telle quelle
    │       │   ├── forms.rs             # NOUVEAU — formulaire résolu et options de taxonomie
    │       │   └── registrations.rs     # NOUVEAU — verrou de ligne compris (R19)
    │       ├── service/
    │       │   ├── transition.rs        # MODIFIÉ — L'UNIQUE HAMEÇON de la naissance (R3)
    │       │   ├── workspace.rs         # MODIFIÉ — remplit les séances, ajoute l'action
    │       │   ├── birth.rs             # NOUVEAU — créer les séances d'un dossier retenu
    │       │   ├── planner.rs           # NOUVEAU — l'écran et les trois écritures
    │       │   ├── public_schedule.rs   # NOUVEAU — la programmation publique
    │       │   ├── registration.rs      # NOUVEAU — inscrire, annuler, promouvoir, rejoindre
    │       │   └── publication.rs       # NOUVEAU — ce que fait le consommateur
    │       ├── consumers/
    │       │   └── publication.rs       # NOUVEAU — le premier EventConsumer d'un module
    │       ├── routes/
    │       │   ├── openapi.rs           # MODIFIÉ — + 17 chemins, + 2 étiquettes
    │       │   ├── planner.rs           # NOUVEAU — GET /admin/planner, SANS le préfixe
    │       │   ├── sessions.rs          # NOUVEAU — le scope /sessions
    │       │   ├── public_schedule.rs   # NOUVEAU — /schedule et le détail d'une séance
    │       │   └── registrations.rs     # NOUVEAU — le scope /registrations
    │       ├── lib.rs                   # MODIFIÉ — + planner_routes(), + session_routes(),
    │       │                            #   + event_consumers()
    │       └── tests/                   # + 9 fichiers
    ├── api/
    │   └── lib.rs                       # MODIFIÉ — /admin/planner composé À PARTIR DE DEUX MODULES
    └── worker/
        └── main.rs                      # MODIFIÉ — enregistre le consommateur de publication
```

**Structure Decision** : le crate `programme` est **étendu**, jamais recréé (R1). Ses dossiers internes sont nommés par agrégat depuis B4 — décision prise à l'époque pour recevoir ce jalon —, et les fichiers nouveaux s'y rangent sans réorganiser un seul fichier existant. Cinq fichiers livrés sont **modifiés** : `transition.rs` (l'hameçon), `workspace.rs` (les deux blocs vides), `themes.rs` (une seconde entité), `cross/mod.rs` (quatre lectures et deux colonnes), `openapi.rs` (les chemins). Trois fichiers hors du module le sont aussi : `kernel/error.rs`, `kernel/events.rs`, `api/lib.rs`, `worker/main.rs`.

---

## Complexity Tracking

| Violation | Pourquoi elle est nécessaire | Alternative plus simple, et pourquoi elle est écartée |
|-----------|------------------------------|------------------------------------------------------|
| **Principe VIII — le service revérifie la présence des réponses obligatoires** que `tg_validate_registration()` vérifie déjà | Le déclencheur ne vérifie **rien** lorsque la séance ne porte pas de formulaire **attaché** (écart n° 114), alors que le formulaire applicable peut venir de l'édition ou de la plateforme — le cas le plus courant. Et quand il vérifie, il rend une phrase française listant des codes, d'où le contrat ne peut pas extraire le `field` qu'un formulaire doit souligner | **Laisser faire la base seule** : une inscription sans aucune réponse obligatoire passerait sur toute séance sans formulaire attaché, et l'écran n'aurait aucun champ à souligner sur les autres. **Attacher d'office le formulaire résolu à chaque séance** : ce serait écrire une donnée pour contourner un contrôle, et figer un choix que l'administrateur doit pouvoir changer en modifiant le formulaire de l'édition |
| **Principe II — écriture hors schéma n° 3 : `identity.consents`** | La preuve d'un consentement RGPD doit vivre **dans la transaction de la donnée qu'elle couvre**. Le modèle prévoit exactement cet usage : la colonne `source` documente `'registration_form'`. Aucun autre module ne peut poser ce consentement au moment où il est donné | **Un contrat d'événement consommé par le module Identité** : il n'existe aucun consommateur côté `identity`, et la preuve serait écrite **après** l'inscription — refuser faute de consentement deviendrait impossible à garantir, et un relais mort perdrait la preuve d'une donnée déjà écrite. **Ranger le consentement dans les réponses** : ce document a pour clés des codes qu'un administrateur renomme, et la preuve disparaîtrait avec l'inscription |
| **Principe II — le noyau gagne `ConsumerRegistry::register_all()`** | B5 est le **premier module consommateur** du dépôt ; B6 en aura plusieurs. Le noyau porte déjà `EventConsumer`, `ConsumerRegistry` et `claim()` depuis B1 : il leur manquait l'entrée en lot que `JobRegistry` a déjà | **Enregistrer le consommateur ligne à ligne dans `worker/main.rs`** : le fichier grossirait d'une ligne par consommateur, et l'asymétrie avec les travaux différés inviterait à la contourner. Ce n'est pas une violation du principe II — le noyau est **partagé**, et l'étendre est son emploi normal ; c'est noté ici parce que du code du noyau est modifié |

**Ce qui n'est pas une entorse, et pourrait le paraître.** Poser un verrou de ligne sur la séance (R19) ne réimplémente aucun invariant : il rend sûr, sous concurrence, un contrôle que la base **fait déjà** et **fait mal**. Promouvoir exactement le nombre de places libérées (R20) ne réimplémente rien non plus : la fonction du modèle prend le nombre en paramètre, et le service décide lequel — c'est son emploi prévu. Et faire passer une séance publiée en « programmée » (R12) n'est pas ajouter une règle : c'est écrire l'état que le modèle nomme et que personne ne posait.

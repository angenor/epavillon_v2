# Implementation Plan: Média + Engagement (B6)

**Branch**: `006-media-engagement` | **Date**: 2026-08-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/006-media-engagement/spec.md`

---

## Summary

Créer **deux** crates — `backend/crates/modules/media` et `backend/crates/modules/engagement` — pour fermer les deux trous par lesquels tout le reste fuit : un fichier qui n'arrive nulle part, et un courriel qui ne part pas.

L'approche tient en neuf points, tous déduits du modèle, du contrat du front, ou d'un précédent livré :

1. **Le service n'émet rien et n'enfile rien : les deux déclencheurs le font.** Le dépôt d'un objet met en file son traitement **et** émet ; la matérialisation des rappels met en file un travail par rappel **et** émet. Le piège n° 1 des cinq modules précédents, **deux fois dans le même jalon** (R3).
2. **Le consommateur d'inscriptions branche sur le STATUT.** `programme.registration.confirmed`, que le modèle nomme lui-même, **n'existe pas** : un consommateur écrit d'après ce commentaire ne serait jamais réveillé (R4, écart n° 126).
3. **Le dépôt passe par l'API, en flux, en un seul geste**, précédé d'une annonce facultative. L'hypothèse H1 prévoyait ce repli ; le prendre évite d'exposer le stockage en écriture au navigateur et fait vérifier l'empreinte gratuitement (R6). **La spécification est amendée, et l'amendement est daté.**
4. **Le stockage vit derrière un contrat, avec deux implémentations.** `make check-db` efface le layout de Garage : des tests qui le frapperaient échoueraient après chaque vérification complète, et l'on prendrait l'habitude de les sauter (R7).
5. **Une seule modification du modèle, et elle est bornée d'avance** : une **fonction de lecture**, parce que l'agrégat des rappels a **deux** lecteurs et que deux agrégations divergeraient en silence (R17). Ni table, ni colonne, ni type.
6. **Le droit de rattacher est le droit d'écrire sur l'entité porteuse.** Aucune permission `media.*` n'existe, et un test parcourt la table blanche pour qu'aucune ligne ne reste sans garde (R15, écart n° 127).
7. **La garde d'envoi enveloppe le contrat du noyau**, si bien que la liste de suppression et le journal s'appliquent aux courriels de B1 et B2 **sans qu'aucun module livré ne change d'une ligne** (R24, écart n° 133).
8. **Une inscription reprise réactive ses rappels**, la clé d'unicité du modèle interdisant de les recréer. Sans cela, une personne qui se désiste puis revient ne reçoit plus jamais rien — en silence (R21).
9. **Aucune écriture hors du schéma du module**, dans aucun des deux. C'est une amélioration sur B3, B4 et B5, et elle est vérifiée par un contrôle mécanique plus strict que le leur.

---

## Technical Context

**Language/Version** : Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies** : **trois déclarations nouvelles**, chacune consignée dans `docs/progression/decisions/2026-08-21.md` comme la constitution l'exige.

| Crate | Pourquoi | Ampleur |
|---|---|---|
| `actix-multipart` 0.8 | le dépôt de fichier est la seule route qui ne parle pas JSON ; c'est le composant officiel d'Actix, de la même famille que le serveur déjà déclaré | faible |
| `image` 0.25 (`default-features = false`, codecs `jpeg`, `png`, `webp`, `gif`) | décoder, mesurer et redimensionner. La seule alternative sans crate serait d'appeler des binaires système, invisibles à `cargo` et absents le jour du déploiement | **moyenne, assumée** |
| `hmac` 0.12 | la signature S3. Minuscule, RustCrypto, même famille que `sha2` déjà déclaré. **0.12 et non 0.13** : la 0.13 exige `sha2` 0.11, quand le workspace tient la 0.10 | négligeable |

**Le client S3 d'Amazon est écarté** (R8) : quarante crates de plus, payés à chaque `make check-back` d'un dépôt sans intégration continue. La signature SigV4 est écrite ici, dans un fichier, avec un **critère de bascule écrit d'avance** — si elle n'est pas au vert contre Garage en une demi-journée, prendre le SDK et le consigner. L'argument qui a fait choisir `ammonia` contre un filtre maison ne vaut pas ici : une erreur de signature est **bruyante et totale**, jamais silencieuse.

`ammonia` et `reqwest` sont **déjà déclarées** et réemployées telles quelles. **Aucun SQL composé dynamiquement.**

**Storage** : PostgreSQL 17 + pgvector — **une fonction de lecture ajoutée**, aucune table ni colonne (R17). **Garage** (compatible S3) devient un vrai dépendant du jalon, pour la première fois : les cinq variables `S3_*` existent dans `.env.example` depuis le 16/08 et n'avaient jamais servi. Valkey toujours inutilisé.

**Testing** : `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock de base. Le **stockage** des tests est le système de fichiers (R7) ; le stockage S3 réel se vérifie par le point de contrôle manuel du [quickstart](quickstart.md), comme B1 a vérifié Mailpit. Le semis ne fournit ni règle de rappel, ni modèle de message, ni objet stocké : chaque test les pose.

**Target Platform** : serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type** : service web (API) adossé à un front Nuxt existant. `api` monte les routes et **compose le préfixe `/sessions`** à partir de deux modules — pour la première fois, un préfixe livré par B5 est rouvert. `worker` enregistre **cinq travaux différés et deux consommateurs d'outbox**.

**Performance Goals** : **aucune cible chiffrée, et deux garanties de comportement**. Un fichier de 200 Mio doit traverser l'API **en flux**, sans être chargé en mémoire — c'est une propriété du code, pas un débit. Et un dépôt dédupliqué ne doit **écrire aucun octet supplémentaire** sur le disque, ce qui se mesure sur le stockage lui-même. Un point à surveiller : le redimensionnement d'une photographie bloque un fil du worker pendant quelques centaines de millisecondes ; il vit donc dans une tâche bloquante dédiée, comme le hachage de mot de passe en B1.

**Constraints** : `DATABASE_URL` renseignée et base démarrée pour compiler · **`down -v` obligatoire au premier lancement**, le schéma ayant changé · aucun fichier de `backend/` au-dessus de 1000 lignes · aucun avertissement Clippy · **aucune arête entre les deux crates ni vers un autre module** · **aucune écriture hors du schéma du module** · les noms de champs sont **exactement** ceux de `frontend/app/types/media.ts`, `types/engagement.ts` et `ReminderSlot`.

**Scale/Scope** : **33 routes HTTP**, **2 crates créés**, **5 travaux différés**, **2 consommateurs d'outbox**, **2 événements émis** (six le sont par la base), **16 codes d'erreur** ajoutés au catalogue, **1 fonction de lecture** ajoutée au modèle, **aucune addition au noyau**, **1 refactorisation de préfixe** touchant du code livré, **1 décorateur du contrat d'envoi**, **5 formes ajoutées** au contrat du front. Volumétrie de référence : quelques milliers d'objets, quelques dizaines de milliers de rappels par édition.

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `050_media.sql` (917 lignes) et `110_engagement.sql` (1 002 lignes) relus intégralement, plus `075` § 1 et 4, `060` § 5, `030` § 5, `115` § 5, `010`. Une seule modification envisagée, annoncée et bornée par la spécification | ⚠️ **Une modification, et elle est une fonction de lecture** (R17) : `engagement.session_reminder_schedule()`. Ni table, ni colonne, ni type. Justifiée en « Complexity Tracking », sur le précédent exact de `media.attached_image()`, que trois modules appellent déjà. **Et la conception a plié devant le modèle sept fois** : le service n'émet ni n'enfile (R3) ; le quota reste porté par la base (R5) ; `is_exclusive` est laissée au trigger ; le regroupement de notifications s'appuie sur l'index partiel du modèle ; `unsupported` est préféré à `clean` parce que le modèle l'a prévu (R13) ; les décalages restent une liste (R20) ; et la non-cumulation n'est pas « améliorée » |
| II | Frontières de modules | ✅ Deux crates, un par schéma. Aucune dépendance de l'un vers l'autre : le seul lien du modèle entre les deux schémas appartient à la messagerie, hors périmètre | ✅ **Tenu, et plus strictement que dans les trois modules précédents** : **aucune écriture hors du schéma du module**, ni dans `media`, ni dans `engagement` — le contrôle mécanique du quickstart porte aussi sur `reference` et `content`. **Le noyau n'est pas modifié** : `register_all` existe depuis B5, `Mailer` depuis B1, et le décorateur d'envoi est une **implémentation** du contrat, pas une extension. `cargo tree` doit rester sans arête dans les deux sens |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée : la modification est une fonction | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` dans la transaction, jamais d'insertion à la main | ✅ **Tenu dans les deux sens, et c'est le module qui en fait le plus.** En émission : **deux** événements seulement, et six sont émis par la base sans que le service les redouble ; le test compte les lignes. En consommation : **deux consommateurs**, la garde de rejeu portée par le noyau, et **trois barrières superposées** contre le double envoi — rejeu, unicité de rappel, idempotence de travail — dont [`contracts/events.md`](contracts/events.md) § 4 dit pourquoi aucune n'est redondante |
| V | Autorisation par permission et par portée | ✅ Les quatre permissions du module Engagement viennent du modèle ; **aucune permission média n'existe** (écart n° 127) | ⚠️ **Une décision structurante** : le droit de rattacher est le droit d'écrire sur l'entité porteuse, résolu par une table de gardes, avec un test qui parcourt la table blanche pour qu'aucune ligne ne reste sans garde (R15). **Trois cas du périmètre distincts** sur les trois lectures d'administration : aucun droit → refus explicite. **Et une garde sans permission** : le calendrier des rappels d'une séance s'ouvre par l'**adhésion active** de l'organisation qui l'anime — une organisation n'administre rien, c'est la règle posée par B4 |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ✅ **Aucune exception, aucun SQL dynamique.** Trois traversées ajoutées, toutes explicites : les décalages **en minutes** dans les deux sens (R19), les énumérations en texte avec cast — le patron de `identity` et de `programme` —, et `platform.i18n_text` en `jsonb`. Les colonnes de la fonction ajoutée s'annotent une à une : une fonction qui rend une table ne porte aucune contrainte de nullité, leçon de B3 |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu, **avec deux acteurs de fond assumés et exacts** : le traitement d'un objet et l'envoi d'un rappel sont écrits par le worker, dont le contexte est `background(...)`. L'audit porte donc une étiquette et non une personne — ce qui est juste : personne ne fabrique une vignette à la main. Toutes les écritures issues d'une requête portent l'acteur de la session |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Onze refus identifiés, tous destinés à la traduction | ✅ **Aucune entorse.** Le contrôle de capacité **avant** l'écriture n'en est pas une : `has_storage_capacity()` est la fonction que le modèle décrit comme « contrôle opposable au téléversement », et le refus final reste celui du trigger — les deux sortent sous **le même code** (R14). **Quinze règles vivent dans le service et n'en sont pas** : la base ne les porte pas du tout, et [`data-model.md`](data-model.md) § 4 les liste une à une avec leur écart |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ **Seize codes ajoutés**, et pas plus. Neuf situations qui ressemblent à des erreurs sortent en **200** avec leur valeur, et [`contracts/errors.md`](contracts/errors.md) dit pourquoi chacune. Les cinq refus de `tg_validate_attachment` **partagent trois `SQLSTATE`** : le service les distingue par le contrôle qu'il a lui-même fait en amont, **jamais par le texte du message** — la règle de B3 |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations ont chacune leur test, plus **huit qui ne se déduisent d'aucune** et que le quickstart énumère : le double décompte d'outbox et de file ; l'inscription créée à l'état « inscrit » ; le **balayage de charge utile** prouvant qu'aucun nom d'inscrit ne sort ; la réinscription qui réactive ; le `href` porteur d'une variable qui survit à l'assainissement ; l'objet rattaché qu'on ne peut pas supprimer ; la table blanche lue **en base** pour qu'aucune ligne ne soit sans garde ; et **les trente-trois routes frappées sur la vraie application** |

**Verdict** : **aucune entorse au principe VIII**, ce qui n'était arrivé ni en B3, ni en B5. **Aucune écriture hors schéma**, ce qui n'était arrivé ni en B3, ni en B4, ni en B5. **Une modification du modèle**, justifiée ci-dessous — et c'est la seule chose que ce plan demande qu'un plan précédent n'ait pas eue.

**Cinq points ont été tranchés dans [`research.md`](research.md)** plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une faute **à l'exécution** :

- **R3** — la double émission et la double mise en file : deux traitements par fichier, deux courriels par rappel ;
- **R4** — le branchement sur le statut : sans lui, **aucun rappel ne part**, et rien ne le dit ;
- **R21** — la réactivation des rappels d'une inscription reprise : sans elle, qui se désiste puis revient ne reçoit plus jamais rien ;
- **R26** — la politique d'URL de l'assainisseur : un `href` porteur d'une variable détruit à l'écriture donne un lien mort dans le courriel, et cela ne se voit qu'à la réception ;
- **R14** — le même code d'erreur pour les deux refus de quota : la course décrite au cas limite n° 13 les rend tous deux atteignables, et deux codes obligeraient l'écran à traiter deux fois le même cas.

---

## Project Structure

### Documentation (this feature)

```text
specs/006-media-engagement/
├── spec.md              # Ce qu'il faut faire (/speckit-specify), + section « Amendements »
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 31 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les 33 routes, leur autorisation, les 5 formes ajoutées
│   ├── errors.md        #   les 16 codes stables et la traduction PostgreSQL
│   └── events.md        #   2 émis, 6 par la base, 2 consommateurs, 5 travaux différés
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
docs/database/
└── 110_engagement.sql                  # MODIFIÉ — + engagement.session_reminder_schedule()
                                        #   UNE FONCTION DE LECTURE. Ni table, ni colonne. (R17)

backend/
├── Cargo.toml                          # + actix-multipart, image, hmac ; + les deux crates
└── crates/
    ├── kernel/
    │   ├── error.rs                    # MODIFIÉ — + 16 variantes (MEDIA_*, ENGAGEMENT_*)
    │   └── config.rs                   # MODIFIÉ — + MediaConfig, + EngagementConfig
    ├── contracts/
    │   ├── programme.rs                # MODIFIÉ — + les noms des événements ÉMIS PAR LA BASE
    │   │                               #   que ce module consomme : ils étaient absents, chacun
    │   │                               #   les aurait écrits en littéral dans son coin
    │   ├── media.rs                    # NOUVEAU — 2 constantes, 1 charge utile
    │   └── engagement.rs               # NOUVEAU — 1 constante, 1 charge utile
    ├── modules/
    │   ├── identity/ · org/ · event/   # INCHANGÉS — c'est la promesse de R24
    │   ├── programme/
    │   │   ├── lib.rs                  # MODIFIÉ — /sessions n'ouvre plus son scope (R29)
    │   │   ├── routes/sessions.rs      # MODIFIÉ — routes sans préfixe, chemins inchangés
    │   │   ├── repo/sessions.rs        # MODIFIÉ — appelle la fonction agrégée (FR-053)
    │   │   └── domain/sessions.rs      # MODIFIÉ — `reminders` cesse d'être une liste vide
    │   ├── media/                      # NOUVEAU CRATE
    │   │   ├── domain/
    │   │   │   ├── ids.rs               #   AssetId, AttachmentId, RenditionId
    │   │   │   ├── asset.rs             #   états, visibilité, formes de réponse
    │   │   │   ├── rules.rs             #   la table blanche, telle qu'elle se lit
    │   │   │   ├── guards.rs            #   LA TABLE DE GARDES (R15) — le fichier le plus
    │   │   │   │                        #   important du module, et un test le parcourt
    │   │   │   ├── keys.rs              #   la convention de clé d'objet, PURE
    │   │   │   └── variants.rs          #   le jeu de déclinaisons, depuis la configuration
    │   │   ├── storage/
    │   │   │   ├── mod.rs               #   le trait ObjectStore (R7)
    │   │   │   ├── sigv4.rs             #   LA SIGNATURE, un fichier, un critère de bascule (R8)
    │   │   │   ├── s3.rs                #   Garage
    │   │   │   └── filesystem.rs        #   les tests, et le développement hors ligne
    │   │   ├── scan/
    │   │   │   ├── mod.rs               #   le trait Scanner (R13)
    │   │   │   ├── clamd.rs             #   INSTREAM sur TCP, aucune dépendance nouvelle
    │   │   │   └── none.rs              #   un moteur DÉCLARÉ, pas une absence
    │   │   ├── repo/
    │   │   │   ├── assets.rs · renditions.rs · attachments.rs · quotas.rs
    │   │   │   └── cross.rs             #   LES SEULES LECTURES HORS SCHÉMA, réunies ici
    │   │   ├── service/
    │   │   │   ├── upload.rs            #   annonce, flux, empreinte, déduplication (R6, R10)
    │   │   │   ├── attach.rs            #   poser, remplacer, retirer, en lot (R16)
    │   │   │   ├── read.rs              #   objets, déclinaisons, avancement
    │   │   │   └── admin.rs             #   orphelins, quotas, suppression
    │   │   ├── jobs/
    │   │   │   ├── process.rs           #   mesurer, analyser, décliner, rendre servable
    │   │   │   ├── purge.rs             #   récurrente, se replanifie (patron B1)
    │   │   │   └── reconcile.rs         #   récurrente
    │   │   ├── routes/ · state.rs · lib.rs
    │   │   └── tests/                   #   9 fichiers
    │   └── engagement/                  # NOUVEAU CRATE
    │       ├── domain/
    │       │   ├── ids.rs · reminder.rs · notification.rs · template.rs
    │       │   ├── offsets.rs           #   minutes ⇄ intervalle, PUR (R19)
    │       │   ├── render.rs            #   la substitution de variables, PURE (R25)
    │       │   └── sanitize.rs          #   LA LISTE BLANCHE DU COURRIEL, et le piège du lien (R26)
    │       ├── repo/
    │       │   ├── rules.rs · reminders.rs · notifications.rs · preferences.rs
    │       │   ├── templates.rs · emails.rs · suppressions.rs
    │       │   └── cross.rs             #   LES SEULES LECTURES HORS SCHÉMA, réunies ici
    │       ├── service/
    │       │   ├── schedule.rs          #   matérialiser, réactiver, décaler, annuler (R21)
    │       │   ├── rules.rs · notifications.rs · templates.rs · deliverability.rs
    │       │   └── compose.rs           #   du type au courriel : modèle, repli, variables (R27)
    │       ├── consumers/
    │       │   ├── reminders.rs         #   BRANCHE SUR LE STATUT (R4) — écart n° 126
    │       │   └── notifications.rs     #   la correspondance est une DONNÉE (R22)
    │       ├── jobs/
    │       │   ├── send_reminder.rs
    │       │   └── partitions.rs        #   récurrente — écart n° 137
    │       ├── mail.rs                  #   LE DÉCORATEUR : garde + trace, sans toucher aux
    │       │                            #   modules livrés (R24) — écart n° 133
    │       ├── routes/ · state.rs · lib.rs
    │       └── tests/                   #   9 fichiers
    ├── api/
    │   ├── lib.rs                       # MODIFIÉ — /sessions composé À PARTIR DE DEUX MODULES
    │   ├── state.rs                     # MODIFIÉ — mailer ENVELOPPÉ, + les deux états
    │   └── openapi.rs                   # MODIFIÉ — + 2 étiquettes
    └── worker/
        └── main.rs                      # MODIFIÉ — + 5 travaux, + 2 consommateurs, mailer enveloppé

Makefile                                 # MODIFIÉ — garage-init importe une clé FIXE (R31)
.env.example                             # MODIFIÉ — + 10 variables
```

**Structure Decision** : **deux crates sont créés**, un par schéma, sur le patron interne de `org` et de `event` — `domain/`, `repo/`, `service/`, `routes/`, `jobs/`, `state.rs`. Deux dossiers leur sont propres et n'existent nulle part ailleurs : `storage/` et `scan/` dans `media`, parce qu'ils portent des contrats à deux implémentations que la configuration choisit — exactement le patron de `kernel::mail`.

**Du code livré est modifié, et il faut le savoir avant de commencer** : `programme` cesse d'ouvrir le scope `/sessions` (R29) et sa composition d'espace organisation cesse de rendre une liste de rappels vide (FR-053) ; `api` compose le préfixe et **enveloppe le mailer** ; `worker` enveloppe le mailer et enregistre sept choses de plus ; `contracts/programme.rs` gagne les noms des événements que la base émet et que ce module consomme. **Aucune route de B5 ne change de chemin**, et le test qui frappe les dix-sept routes de `programme` le vérifie.

---

## Complexity Tracking

| Violation | Pourquoi elle est nécessaire | Alternative plus simple, et pourquoi elle est écartée |
|-----------|------------------------------|------------------------------------------------------|
| **Principe I — une fonction est ajoutée à `110_engagement.sql`** : `engagement.session_reminder_schedule()` | L'agrégat des rappels a **deux** lecteurs : la lecture par séance (module Engagement) et la composition de l'espace organisation (module Programme). FR-052 exige qu'il soit écrit une seule fois. Deux agrégations séparées divergeraient au premier ajustement, et **la divergence serait silencieuse** — un nombre de destinataires faux ressemble à un nombre juste. C'est le précédent exact de `media.attached_image()`, appelée par trois modules, et dont l'en-tête du fichier média porte mot pour mot ce raisonnement. **Ni table, ni colonne, ni type** : la surface du modèle ne change pas | **Écrire l'agrégation dans chaque module** : la divergence ci-dessus, et deux endroits à corriger le jour où la consolidation d'état change. **N'exposer qu'une route et laisser le front composer** : cela contredirait FR-053, laisserait l'espace organisation rendre une liste vide, et l'écart n° 108 resterait ouvert alors que ce jalon existe pour le fermer. **La porter dans le noyau** : il n'a « aucune connaissance métier », et l'y mettre ferait du noyau un module |
| **Une dépendance d'ampleur moyenne : `image` 0.25** | Décoder, mesurer et redimensionner une photographie. Le relevé des dimensions est **exigé par le modèle** — `attachable_roles.expected_aspect_ratio` ne peut être vérifié sans lui —, et les déclinaisons sont la raison d'être du schéma des variantes | **Appeler `vipsthumbnail` ou `cwebp` en sous-processus** : une dépendance système invisible à `cargo`, qui manque le jour du déploiement et qu'aucun test ne signale. **Ne pas décliner du tout** : la plateforme servirait des originaux de dix mégaoctets à des visiteurs sur téléphone, ce qui est précisément le défaut de la v1 que le schéma des variantes corrige |
| **Une signature S3 écrite ici plutôt que déléguée à un SDK** | `aws-sdk-s3` amène une quarantaine de crates, payés à **chaque** `make check-back` d'un dépôt sans intégration continue, tenu par une seule personne sous contrainte de temps. Quatre verbes suffisent, en *path-style*, contre notre propre stockage sur le réseau interne | **Prendre le SDK** : c'est l'alternative, et **le critère de bascule est écrit d'avance** (R8) — si la signature n'est pas au vert contre Garage en une demi-journée, on prend le SDK et on le consigne. L'argument d'`ammonia` ne s'applique pas : une erreur de signature est **bruyante et totale**, jamais silencieuse, à l'inverse d'un filtre HTML dont le premier trou est une injection |
| **Le préfixe `/sessions`, livré par B5, est rouvert** | Le module Engagement y dépose deux routes, et **deux `web::scope` du même préfixe ne se complètent pas** : Actix retient le premier et rend 404 sur les routes du second. Le défaut a coûté trois routes sur vingt et une en B2 | **Poser les deux routes ailleurs**, sous `/reminders?session_id=` : le contrat du front les attend rattachées à la séance, et déplacer une ressource pour éviter une refactorisation de vingt lignes serait céder sur le contrat. **Ne rien faire** : les deux routes seraient **muettes**, et le défaut est raconté dans trois fichiers du dépôt |

**Ce qui n'est pas une entorse, et pourrait le paraître.**

- **Vérifier la capacité avant d'écrire** (R14) ne réimplémente rien : `has_storage_capacity()` est la fonction que le modèle décrit comme « contrôle opposable au téléversement », et le refus final reste celui du trigger.
- **Exiger le texte alternatif au dépôt** (R9) ne double pas `ck_assets_alt_text_required` : la contrainte s'applique au passage à l'état servable, c'est-à-dire **après** le traitement. Le service ne l'anticipe pas, il empêche d'écrire un objet qui ne pourra jamais y arriver.
- **Refuser la suppression d'un objet rattaché** (R11) n'ajoute pas de règle : c'est la définition de l'orphelin que le modèle porte déjà dans `find_orphan_assets()`, appliquée là où elle protège.
- **Envelopper le contrat d'envoi du noyau** (R24) n'étend pas le noyau : c'est une **implémentation** de `Mailer`, exactement ce que le contrat existe pour permettre. Le noyau n'est pas modifié, et aucun module livré non plus.

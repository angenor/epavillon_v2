# Implementation Plan: Guide Négo — compte et admission (étape 0b)

**Branch**: `009-guide-nego-compte-admission` | **Date**: 2026-09-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-guide-nego-compte-admission/spec.md`

## Summary

L'entrée dans Guide Négo : une personne crée un compte ou se connecte avec celui de l'ePavillon,
saisit le code d'invitation reçu sur WhatsApp, et entre — aussitôt ou après l'approbation d'un
administrateur, selon un réglage. L'IFDD tient ses codes, voit qui est entré avec lequel, retire des
accès, et durcit l'admission sans redéployer.

**L'approche, en une phrase** : le SQL d'abord — sept manques à combler dans quatre fichiers, **et un
script de migration**, parce que `identity.sessions` est une table en service —, puis un **crate
`negotiation` qui naît à cette étape**, puis huit écrans d'application bâtis sur les composants déjà
livrés en 0a et cinq écrans de back-office bâtis sur ceux du site.

Trois choix commandent tout le reste :

1. **Rien de l'authentification n'est réécrit.** Inscription, vérification, connexion, rotation et
   réinitialisation servent l'application telles quelles ; seules deux routes gagnent un objet
   `client` facultatif pour que la session sache d'où elle vient (ADR-001).
2. **Les refus d'un code sortent en 200 avec leur discriminant et leur message**, comme
   `mfa_required` le fait déjà pour la connexion. Sept refus prévus par le parcours — sur neuf issues
   en tout — ne sont pas sept pannes, et aucun écran ne doit rester sans action possible.
3. **La portée d'un code est celle du rôle `negotiator`** : `negotiation_space` ou `global`, les deux
   valeurs de ses `allowed_scopes`. Le choix tranché par le commanditaire — une COP, ou tout Guide
   Négo — n'a demandé aucune invention : il était déjà dans le modèle.
4. **On migre, on ne détruit pas.** [migration.sql](migration.sql) porte l'écart et se rejoue sans
   dégât ; `down -v`, `make check` et `make check-db` sont proscrits. Le contrôle est celui du § 13 de
   [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md) : comparer le schéma obtenu à celui que produisent les
   fichiers du modèle.
5. **Une session d'application dure 90 jours glissants**, d'office. Douze heures, la durée du site,
   déconnecteraient une négociatrice en salle, là où aucun réseau ne permet de se reconnecter.

## Technical Context

**Language/Version** : Rust stable (Actix Web, SQLx vérifié à la compilation) · TypeScript strict
(Nuxt 4, Vue 3)

**Primary Dependencies** : aucune nouvelle. Le crate `negotiation` ne dépend que de `kernel` et
`contracts` ; le front n'ajoute aucun paquet.

**Storage** : PostgreSQL 17. Schémas touchés : `negotiation` (cinq tables, deux vues, deux types),
`identity` (quatre colonnes et un type sur `sessions`), `reference` (une taxonomie), `platform`
(deux réglages). Garde locale du client : IndexedDB, déjà posée en 0a. **Le changement se livre par
migration**, jamais par rechargement : la base locale comme la production ont des données.

**Testing** : `cargo test --workspace --all-features` sur **base réelle et jetable**
(`kernel::testing::TestDb`, recopie d'une base modèle par `CREATE DATABASE … TEMPLATE`) ; tests HTTP
dans `backend/crates/api/tests/` ; `node --test` pour `frontend/tests/guide-nego/` ; scripts
`check:guide-nego` et contrastes. Porte complète : `make check-safe`, **jamais `make check`**.

**Target Platform** : navigateur mobile Android et iPhone (application installable). **Guide Négo se
mesure à 320, 360 et 390 px** — 320 parce qu'un téléphone d'entrée de gamme ou une police agrandie y
ramènent, et c'est la largeur retenue depuis 0a. Le plancher de 375 px du site ne s'y applique pas.
Back-office sur poste de travail.

**Project Type** : application web — `backend/` (API Rust) + `frontend/` (Nuxt), plus l'application
installable qui est un sous-arbre du même Nuxt.

**Performance Goals** : le parcours complet — compte, code, entrée — en moins de trois minutes
(SC-001) ; l'état d'accès lu en une requête ; le mode d'admission relu à chaque tentative, sans cache.

**Constraints** : hors connexion d'abord (principe XI) — tout écran de lecture s'ouvre sans réseau,
avec l'heure de sa lecture ; design borné à `[data-app="guide-nego"]` (principe XIII) ; aucun fichier
de plus de 1000 lignes ; aucun cookie `epavillon_*` nommé dans un fichier de Guide Négo ; préfixe
`/v2` à éprouver, pas à supposer.

**Scale/Scope** : 13 écrans (8 dans l'application, 5 au back-office), 1 composant nouveau, **16 routes
d'API** — 4 servant l'application, 12 le back-office —, 5 tables, 2 vues, et environ 500 lignes de SQL
entre `docs/database/` et le script de migration.

## Constitution Check

*GATE : passé avant la phase 0, repassé après la phase 1.*

| Principe | Comment ce plan s'y tient | Verdict |
|---|---|---|
| **I. Le modèle fait autorité** | Sept manques identifiés par lecture de `docs/database/`, écrits dans `030_identity.sql`, `020_reference.sql`, `100_negotiations.sql` et `900_seed.sql` **avant** tout code, puis **portés à la base en service par [migration.sql](migration.sql)** — rejouable, éprouvé, contrôlé par comparaison de schémas (§ 13 de `DEPLOIEMENT.md`). Changement noté dans `docs/progression/modele.md` (seule exception d'écriture hors `AppNego`, ADR-017) | ✅ |
| **II. Frontières de modules** | Crate `negotiation` nouveau, dépendances limitées à `kernel` et `contracts`. Le courriel de décision vit dans `negotiation`, pas dans `identity` — c'est précisément ce que ce principe évite | ✅ |
| **III. `xmod_fk_*`** | Toute FK vers `identity.people`, `identity.sessions` la porte. `reference` et `platform` sont le noyau partagé, exemptés par `cross_module_fk_report` | ✅ |
| **IV. Outbox transactionnel** | Six événements par `platform.emit_event()`, dans la transaction du changement d'état. Les courriels par `jobs::enqueue` dans la même transaction, comme `registration.rs` | ✅ |
| **V. Permission et portée** | `negotiation.space.access` et `negotiation.space.manage`, testées par `identity.has_permission` avec leur portée. Aucun nom de rôle nulle part. **Le back-office exige la portée `global`** (tranché le 21/09) : `Requires<SpaceManage>` et non `RequiresAnyScope`, qu'un administrateur d'événement franchirait — il porte la permission sans qu'aucun espace de négociation ne soit rattaché à son édition. URL forgée comprise | ✅ |
| **VI. SQLx vérifié, pas d'ORM** | Macros vérifiées à la compilation ; deux vues servent les écrans de back-office en une requête | ✅ |
| **VII. Contexte d'écriture** | Toute écriture par `Db::write(&ctx)`. `platform.tg_audit()` posé sur les trois tables mutables | ✅ |
| **VIII. Les invariants ne se réimplémentent pas** | Une seule demande en attente, un seul usage par personne et par code, portée cohérente, transition d'état, taxonomie du réseau : **tous portés par la base**, traduits en français par l'API | ✅ |
| **IX. Erreurs à code stable** | Cinq codes nouveaux dans `kernel/src/error.rs`. Un identifiant hors périmètre se refuse comme inexistant | ✅ |
| **X. Tests sur base réelle** | Chemin nominal de chaque route, refus par périmètre en URL forgée, traduction d'un invariant de base, écriture de l'outbox vérifiée | ✅ |
| **XI. Hors connexion d'abord** | « Mon accès » et le verrou lisent une seule route, à travers `useGnLecture` : hors connexion, ce qui a été lu, avec son heure. La saisie d'un code exige le réseau **et le dit** — elle n'est pas mise en file : un accès n'est pas un signalement, on ne peut pas l'annoncer avant de l'avoir obtenu (FR-019) | ✅ |
| **XII. Confiance** | Aucune donnée importée, aucune IA à cette étape. L'application n'annonce jamais un accès non obtenu | ✅ |
| **XIII. Design borné** | Un composant nouveau, `GnVerrou`, dans le dossier de Guide Négo, sous `[data-app="guide-nego"]`. Aucun jeton ni composant du site. Le back-office garde l'apparence de l'ePavillon et ses composants `ui/` | ✅ |
| **XIV. Une seule porte** | Aucun service annexe à cette étape | ✅ |

**Écart assumé, à inscrire** : `01-stack.md` annonçait Valkey pour les essais de code. Valkey n'est
câblé nulle part dans `backend/` et l'y introduire serait une dépendance d'ampleur exigeant une
décision écrite ; une table le fait (recherche R7). La ligne de `01-stack.md` se corrige.

**Aucune violation à justifier** : la table *Complexity Tracking* reste vide.

## Project Structure

### Documentation (this feature)

```text
specs/009-guide-nego-compte-admission/
├── plan.md              # ce fichier
├── spec.md              # la spécification, relue et tranchée
├── research.md          # quatorze décisions prises avant d'écrire
├── data-model.md        # le SQL, d'abord
├── migration.sql        # de la base en service à celle de 0b — rejouable
├── quickstart.md        # comment vérifier que ça marche
├── contracts/
│   ├── session-client.md         # les deux routes d'identity qui bougent, et la durée de session
│   ├── courriels-retour-app.md   # le lien du courriel ramène dans l'application
│   ├── api-acces.md              # l'accès, côté application
│   └── api-admin.md              # le back-office de l'admission
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks — pas créé ici
```

### Source Code (repository root)

```text
docs/database/
├── 030_identity.sql          # + identity.session_client, 4 colonnes sur sessions, 1 index
├── 020_reference.sql         # + taxonomie negotiation_network et son premier terme
├── 100_negotiations.sql      # + §§ codes, usages, réseaux, demandes, essais, 2 vues, 3 triggers
└── 900_seed.sql              # + negotiation.admission_mode, negotiation.invitation_attempts

backend/
├── Cargo.toml                             # + membre et alias du crate
├── crates/kernel/src/error.rs             # + 5 codes stables
├── crates/contracts/src/negotiation.rs    # nouveau : noms d'événements et charges
├── crates/kernel/src/config.rs            # + AUTH_SESSION_TTL_APP
├── crates/modules/identity/
│   ├── src/repo/sessions.rs               # NewSession + INSERT : client, appareil
│   ├── src/service/session.rs             # Device étendu ; durée selon le client ; la rotation recopie
│   ├── src/service/auth.rs                # passe le client reçu
│   ├── src/service/registration.rs        # retient le client avec le jeton
│   ├── src/mail.rs                        # le lien mène à Guide Négo quand la demande en vient
│   ├── src/routes/auth.rs                 # corps de login et register ; me rend la session
│   └── tests/cohabitation_sous_prefixe.rs # étendu au préfixe /v2
├── crates/modules/negotiation/            # NOUVEAU CRATE
│   ├── Cargo.toml                         # kernel + contracts, rien d'autre
│   └── src/
│       ├── lib.rs                         # routes, admin_routes, job_handlers, NegotiationState
│       ├── domain/     permissions.rs · admission.rs · code.rs · access.rs
│       ├── repo/       codes.rs · uses.rs · requests.rs · attempts.rs · networks.rs · settings.rs
│       ├── service/    redeem.rs · requests.rs · admin_codes.rs · admin_requests.rs · admission.rs
│       ├── routes/     acces.rs · admin_codes.rs · admin_requests.rs · admin_admission.rs · openapi.rs
│       ├── mail.rs                        # demande admise · demande refusée
│       └── jobs/       emails.rs · purge.rs
├── crates/api/     src/lib.rs · src/state.rs · src/modules.rs · src/openapi.rs · Cargo.toml
├── crates/api/tests/  routes_negotiation_acces.rs · routes_negotiation_admin.rs
└── crates/worker/  src/main.rs · Cargo.toml

frontend/
├── app/pages/guide-nego/
│   ├── compte.vue · connexion.vue · mot-de-passe-oublie.vue
│   ├── verification-adresse.vue · nouveau-mot-de-passe.vue   # le retour depuis un courriel
│   ├── code.vue · demande.vue
│   └── ressources/acces.vue                # « Mon accès »
├── app/pages/guide-nego/ressources/reglages.vue   # + Mon accès, + déconnexion
├── app/components/guide-nego/GnVerrou.vue         # le seul composant nouveau
├── app/components/guide-nego/planche/PlancheComposantsSurfaces.vue  # + sa vitrine
├── app/composables/guide-nego/useGnSession.ts · useGnAcces.ts   # ⚠ à ne pas confondre avec
│                     # useGnConnexion.ts (0a), qui dit l'état du RÉSEAU, pas celui du compte
├── app/composables/api/guide-nego.ts              # nouveau module d'API
├── app/composables/useApi.ts                      # + le bloc negotiation
├── app/utils/guide-nego/appareil.ts               # device_id, gardé en local
├── app/pages/admin/negociations/
│   ├── codes/index.vue · codes/nouveau.vue · codes/[id].vue
│   ├── demandes/index.vue
│   └── admission.vue
├── i18n/locales/{fr,en}/pages/guide-nego.{compte,connexion,mot-de-passe-oublie,
│                     verification-adresse,nouveau-mot-de-passe,code,demande,acces}.json
├── i18n/locales/{fr,en}/pages/admin.negociations.{codes,demandes,admission}.json
├── i18n/locales/{fr,en}/components/gn-verrou.json
└── tests/guide-nego/acces.test.ts · appareil.test.ts
```

**Structure Decision** : application web à deux racines symétriques, `backend/` et `frontend/`,
imposée par la constitution (principe II). L'application mobile n'est pas un troisième projet : c'est
le sous-arbre `pages/guide-nego/` du Nuxt existant, avec sa mise en page, ses composants `Gn*` et son
système de design borné — posés à l'étape 0a et repris ici sans rien redessiner. Le back-office de
l'admission s'ajoute à celui de l'ePavillon, sous `pages/admin/negociations/`, avec les composants
`ui/` du site.

## Les phases, et pourquoi dans cet ordre

| # | Phase | Ce qu'elle livre | Pourquoi là |
|---|---|---|---|
| 1 | **Le SQL et sa migration** | Les sept manques dans `docs/database/`, **`migration.sql` écrit, joué, rejoué**, `make check-db-safe`, `docs/progression/modele.md` | Principe I : rien ne s'écrit avant. SQLx ne compile même pas sans la base migrée |
| 2 | **Le crate** | `negotiation` monté, vide, avec son état, sa permission typée, son entrée OpenAPI | Une seule fois, et toutes les routes suivantes s'y posent |
| 3 | **La session dit d'où elle vient** | Objet `client`, rotation qui recopie, durée longue pour l'application, `me` qui le rend, liens de courriel vers Guide Négo | US1. Indépendant du reste, et le site ne bouge pas |
| 4 | **Le code d'invitation** (US2) | `redeem` et ses neuf issues, `me/access`, les essais limités | Le cœur du critère de sortie |
| 5 | **Le back-office des codes** (US3) | Créer, lister, révoquer, voir les usages, retirer un accès | L'autre moitié du critère |
| 6 | **L'admission par approbation** (US4) | Mode réglable, demandes, file, décisions, deux courriels | Se pose sur ce que 4 et 5 ont bâti |
| 7 | **Les écrans de l'application** (US1, US2, US4, US5) | Compte, connexion, code, demande, verrou, « Mon accès », déconnexion | Le client, une fois ses routes servies |
| 8 | **La recette** | **Répétition de la migration sur une copie et comparaison des schémas**, traductions `en`, non-régression du site, planche de composants, `make check-safe`, quickstart | Comme en 0a : la recette a trouvé un défaut que rien d'autre ne montrait. Et la comparaison de schémas a déjà rattrapé un oubli, le 16/09 |

Les phases 3, 4 et 5 sont largement indépendantes une fois la phase 2 passée ; 7 suit ses routes.
`make check-safe` **à la fin d'un cycle**, pas à chaque phase — la porte complète coûte plusieurs
minutes ; entre-temps, `npm run check:guide-nego`, `test:guide-nego`, `typecheck` et
`cargo test -p negotiation`.

## Les dix pièges de cette étape

1. **La rotation efface le client.** Si `session::open` ne recopie pas `client_kind` au renouvellement,
   toute session de l'application redevient « site » au bout d'une heure, et les chiffres mentent sans
   que rien n'échoue. C'est le défaut le plus probable de la phase 3.
2. **Un cache sur le mode d'admission tuerait le critère de sortie.** SC-002 exige que la bascule se
   voie sans mise en ligne : la valeur se relit à chaque tentative.
3. **Révoquer n'est pas retirer.** Deux gestes, deux routes, deux écrans (ADR-006). Les confondre
   couperait l'accès de tout un réseau sur une fuite de code.
4. **L'unicité du code doit couvrir les révoqués**, sinon l'écran « 04c » ne peut pas dire *« révoqué
   le 8 novembre »* — il dirait *« code inconnu »*, et la personne chercherait une faute de frappe.
5. **Le garde-fou de Guide Négo interdit de nommer un cookie du site.** La session se lit par le store
   `auth` existant, jamais en touchant un cookie depuis un fichier de Guide Négo (recherche R9).
6. **Douze heures de session.** C'est la valeur du site, et l'écran de connexion de l'application n'a
   pas de case « se souvenir de moi ». Sans correction, une négociatrice est déconnectée en salle, au
   moment précis où aucun réseau ne lui permet de se reconnecter (recherche R12).
7. **Le lien du courriel sort de l'application.** `mail.rs` envoie vers les écrans du site : qui crée
   son compte depuis l'application se retrouve sur l'ePavillon sans retour. Et sur iPhone, la page
   ouverte dans Safari n'est **pas** dans la session de l'application : elle confirme et renvoie, et
   c'est l'application qui relit l'état à son retour au premier plan (recherche R13).
8. **`down -v` n'est pas une procédure.** Il efface une base sans sauvegarde. La migration est le
   livrable qui rend ce plan applicable en production (recherche R14).
9. **`RequiresAnyScope` laisserait entrer un administrateur d'événement.** Le rôle `admin` porte
   `negotiation.space.manage` et s'attribue aussi sur une édition ; « n'importe quelle portée »
   ouvrirait donc tout le back-office de Guide Négo à quelqu'un qui n'a aucun espace de négociation
   à administrer — et la route paraîtrait gardée. La garde est la portée **globale**, et elle se
   teste sur les **douze** routes, pas seulement sur celles qui listent : l'admission et les
   demandes en font partie (FR-044, SC-008, principe V).
10. **Deux noms voisins, deux choses** : `useGnConnexion` (livré en 0a) dit l'état du **réseau** ;
   `useGnSession`, créé ici, dit celui du **compte**. Les confondre ferait croire qu'une personne
   hors connexion est déconnectée.

## Complexity Tracking

Aucune violation de la constitution à justifier.

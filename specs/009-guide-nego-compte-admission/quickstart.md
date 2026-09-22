# Vérifier 0b — compte et admission

> À dérouler **dans l'ordre**, sur la version construite servie à l'adresse exacte
> d'`APP_PUBLIC_URL`, puis sur un téléphone réel. La recette de 0a a montré qu'un défaut peut n'exister
> qu'au premier chargement : ne pas raccourcir.

## Avant de commencer — migrer, jamais détruire

**La base locale ne se détruit pas.** `identity.sessions` est une table en service ; l'écart se
franchit par [migration.sql](migration.sql), comme il le sera en production (§ 13 de
[DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md)).

```bash
cp .env.example .env                                     # si besoin
docker compose -f ops/docker-compose.dev.yml up -d       # les services, sans toucher au volume
make garage-init                                         # la première fois seulement

# 1. Sauvegarder, toujours, même en local
pg_dump "$DATABASE_URL" > ~/epavillon-avant-0b.sql

# 2. Jouer la migration
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f specs/009-guide-nego-compte-admission/migration.sql

# 3. La rejouer : rien ne doit changer, rien ne doit échouer
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f specs/009-guide-nego-compte-admission/migration.sql

# 4. Contrôler
make check-db-safe

cd backend  && cargo run -p api        # et, dans un autre terminal, cargo run -p worker
cd frontend && npm run dev
```

**SQLx compile contre cette base migrée** : sans l'étape 2, `cargo build` échoue sur les colonnes
nouvelles — et c'est tant mieux, c'est le principe VI qui fait son travail.

**Ne jamais lancer `make check`, `make check-db`, ni `docker compose down -v`.** Les trois détruisent
la base locale, qui n'a pas de sauvegarde. Le 16/09, un `make check` lancé avant un commit l'a
effacée. La porte de qualité est `make check-safe`.

**Les tests n'ont besoin de rien de tout cela** : le harnais reconstruit seul sa base modèle dès que
l'empreinte de `docs/database/` change (`kernel/src/testing.rs`).

### Comparer les deux schémas — le contrôle du § 13, étape 6

À faire **une fois la migration jouée et avant la recette**, puis à refaire avant le déploiement :

```bash
pg_dump --schema-only --no-owner --no-privileges "$DATABASE_URL" | sort > /tmp/migree.sql
# la base modèle du harnais est chargée depuis docs/database/ : elle est la référence
psql -Atc "SELECT datname FROM pg_database WHERE datname LIKE 'epavillon_test_template_%'"
pg_dump --schema-only --no-owner --no-privileges "<cette base>" | sort > /tmp/modele.sql
diff /tmp/modele.sql /tmp/migree.sql
```

Attendu : **aucun écart**, hors les partitions `engagement.email_messages_AAAAMM` créées à la volée
par le worker. Un écart trouvé ici est un oubli de la migration, pas un détail : le 16/09, ce contrôle
a rattrapé un commentaire de colonne coupé par l'extraction.

Le drapeau `guide_nego.enabled` doit être **allumé à 100 %** en local :

```sql
UPDATE platform.feature_flags SET is_enabled = true, rollout_percent = 100 WHERE key = 'guide_nego.enabled';
```

Courriels capturés : `http://localhost:8025` (Mailpit).

## § 1 — Le compte (US1)

1. `/guide-nego` → « Créer mon compte ». Le formulaire annonce **étape 1 sur 3**, porte l'aide
   « Celle de votre compte ePavillon, si vous en avez un », et la sortie « J'ai déjà un compte ».
2. Créer un compte. Le courriel de vérification arrive dans Mailpit ; le confirmer.
3. **En base**, la session doit dire d'où elle vient :
   ```sql
   SELECT client_kind, device_label, device_platform FROM identity.sessions ORDER BY issued_at DESC LIMIT 1;
   -- attendu : app | Android · Chrome (ou l'équivalent) | android
   ```
4. **Le piège n° 1** : attendre le renouvellement du jeton d'accès (ou forcer
   `POST /api/auth/refresh`), puis **refaire la requête ci-dessus**. `client_kind` doit **rester
   `app`**. S'il retombe à `web`, la rotation ne recopie pas le client — corriger avant d'aller plus
   loin.
5. Se connecter **sur le site** avec le même compte : `client_kind = web`, et **aucun second compte**
   (`SELECT count(*) FROM identity.people WHERE primary_email = …` → 1).
6. Se déconnecter depuis « Profil et réglages » de l'application : la session de cet appareil est
   révoquée, **celle du site reste ouverte**.
7. « Mot de passe oublié » : la réponse à l'écran est **la même** que l'adresse existe ou non ; le lien
   reçu vaut une heure.
8. **La durée de la session** — l'écran de connexion de l'application n'a pas de case « se souvenir de
   moi », et douze heures ne suffisent pas à une COP :
   ```sql
   SELECT client_kind, issued_at, expires_at, expires_at - issued_at AS duree
     FROM identity.sessions ORDER BY issued_at DESC LIMIT 2;
   -- app : 90 jours.   web sans « se souvenir de moi » : 12 h, inchangé.
   ```
   Puis forcer une rotation et **revérifier** : la durée doit repartir de 90 jours à chaque
   renouvellement (glissante), pas fondre. Un compte ouvert le 15 octobre ne doit pas expirer le
   14 novembre, en pleine COP.
9. **Le retour du courriel dans l'application.** Créer un compte depuis l'application, ouvrir Mailpit,
   toucher le lien : il mène à **une page de Guide Négo** (`/guide-nego/…`), à son apparence, qui
   confirme l'adresse et ramène à la suite du parcours — et **non** à l'écran du site, aux couleurs de
   l'ePavillon et sans chemin de retour. Refaire depuis le **site** : le lien mène bien à l'écran du
   site, comme avant.
10. **Session expirée hors connexion** : ramener `expires_at` dans le passé, couper le réseau, ouvrir
    l'application. Elle doit **garder ce qui a été lu** et le dire avec son heure ; elle ne réclame la
    reconnexion qu'au retour du réseau, sans effacer l'écran.

## § 2 — Le code (US2)

Semer deux codes au back-office (§ 3) ou en SQL, l'un du réseau, l'autre général.

| À saisir | Attendu à l'écran |
|---|---|
| Le code juste, mode `code` | « Code reconnu. Bienvenue dans le réseau… », accès ouvert aussitôt |
| Le même, une seconde fois | `already_granted` — ni second accès, ni perte de l'accès |
| Le code du réseau, alors qu'on a déjà l'accès général | L'appartenance s'ajoute, l'accès ne double pas |
| Un code inexistant | « Ce code n'est pas reconnu », rappel des caractères, « Demander l'accès à l'IFDD » |
| Un code révoqué | **« révoqué le … »**, avec la date, et la demande d'accès proposée |
| Un code au quota atteint | Message d'épuisement, distinct du précédent |
| Un code dont la validité est passée | Message de fin de validité, distinct de l'épuisement |
| `nego-024`, `NEGO 024`, `Nego024` | **Tous reconnus comme le même code** — casse et séparateurs ignorés |
| Six essais faux d'affilée | Refus annoncé **avec son temps d'attente**, la demande d'accès reste ouverte |

Puis, **réseau coupé** : la saisie d'un code dit qu'elle demande le réseau et **n'annonce aucun
accès**. Rien n'est mis en file : un accès ne se promet pas.

En base, après un code accepté :

```sql
SELECT count(*) FROM negotiation.invitation_code_uses WHERE person_id = :p;   -- 1, pas 2
SELECT role_code, scope_type, scope_id FROM identity.role_assignments WHERE person_id = :p AND revoked_at IS NULL;
SELECT event_type FROM platform.outbox_events ORDER BY created_at DESC LIMIT 3;
```

## § 3 — Les codes tenus par l'IFDD (US3)

1. `/admin/negociations/codes` → créer un code : libellé, **portée — une COP précise, ou Guide Négo en
   entier**, quota, validité, appartenance au réseau. Le code engendré s'affiche, **et reste lisible
   dans la liste**.
2. L'entrée par ce code (§ 2) fait monter le compteur d'usages.
3. **Révoquer** : le code n'ouvre plus dès la tentative suivante — **et les personnes déjà entrées
   gardent leur accès**. Le vérifier explicitement : c'est ADR-006, et c'est le piège n° 3.
4. Ouvrir les usages du code : retirer l'accès d'une personne. Elle ne voit plus les modules réservés
   **à son ouverture suivante**, et « Mon accès » dit qu'il a été retiré.
5. **Le périmètre, URL forgée comprise** : avec un compte administrateur d'un seul événement, ouvrir
   directement l'adresse d'un code d'un autre espace → **refus**, indistinguable d'un identifiant
   inexistant.

## § 4 — Durcir sans redéployer (US4)

1. `/admin/negociations/admission` → basculer sur « approbation ». **Aucune mise en ligne, aucun
   redémarrage.**
2. Dans l'application, ouvrir un module réservé : la saisie de code **n'est plus proposée**, la demande
   l'est.
3. Envoyer la demande : l'écran d'attente porte nom, pays, **heure d'envoi avec son fuseau**, « En
   attente », l'annonce du **courriel**, et rappelle ce qui reste lisible.
4. Renvoyer la demande depuis un second appareil → **une seule ligne** ; l'erreur est traduite en
   français, pas en 500.
5. Au back-office, admettre : **Mailpit reçoit le courriel**, l'accès est accordé, l'appartenance au
   réseau suit si la demande portait un code du réseau.
6. Refuser une autre demande avec un motif : le courriel le porte, « Mon accès » aussi.
7. Basculer sur « les deux », saisir un **code juste** : il **n'ouvre pas**, il ouvre une demande qui
   porte le code.
8. Basculer le mode pendant qu'une demande est en attente : elle reste traitable.
9. Sur l'écran d'attente, « J'ai reçu un code — Le saisir » : le code accepté **remplace** la demande
   (`cancelled`).

## § 5 — Le verrou et « Mon accès » (US5)

1. Sans accès, ouvrir un module réservé : titre du module, « Réservé aux négociatrices et
   négociateurs », ce qui s'y trouve, le rappel de ce qui reste ouvert, **deux sorties**.
2. Sans compte, le verrou mène d'abord à la création de compte, **puis** au code.
2 bis. **Le verrou suit le mode d'admission**, et sa sortie change sans mise en ligne :
   - mode « code seul » et mode « les deux » → il mène à la **saisie du code** ;
   - mode « approbation seule » → il mène à la **demande**, et ne propose aucun code (FR-022) ;
   - une demande **déjà en attente** → il ramène à cette demande, jamais à une saisie.
   Basculer le mode au back-office pendant l'essai et rouvrir le module : la sortie a changé.
3. « Mon accès » dans chacun des cinq états — visiteuse, en attente, admise, refusée, retirée — dit
   ce qu'il faut, **avec ce que l'accès ouvre** : une COP nommée, ou tout Guide Négo.
4. **Réseau coupé** : « Mon accès » s'ouvre et affiche l'état **lu**, avec son heure.
5. **Aucun écran de ce parcours ne demande ni n'affiche un genre.** Le vérifier en le cherchant.

## § 6 — La mesure et le design

- 320, 360 et 390 px : aucun débordement horizontal sur les cinq écrans nouveaux.
- Thème clair et sombre, comparés à `02-socle.html` — mêmes mesures, mêmes états, mêmes mots que
  `design/lexique.md` : « Code d'invitation », jamais « clé » ni « jeton » ; « Demande en attente ».
- `GnVerrou` figure sur `/guide-nego/composants`, dans les deux thèmes, **et sort de la liste des
  absents**.
- Anneau de focus visible partout ; cibles d'au moins 48 px.

## § 7 — Les garde-fous

```bash
cd frontend && npm run check:guide-nego && npm run test:guide-nego && npm run typecheck
cd backend  && cargo test -p negotiation
make openapi && make check-api-contract
npm run verifier-garde:guide-nego <adresse>     # les nouvelles adresses comprises
make check-safe                                  # à la fin du cycle seulement
```

`make check-safe` doit être **au vert** avant le commit. Attention au test `identity` déjà repéré le
21/09 comme sensible à une activité concurrente sur la base : arrêter l'API avant de lancer la porte.

## § 8 — Non-régression du site

À faire **sur la version construite**, comme en 0a :

- accueil, back-office et `/negociations` inchangés, en clair et en sombre ;
- après une visite du site **seul** : **aucun service worker, aucun manifeste** ;
- les pages d'authentification du site fonctionnent **sans** objet `client` — c'est le cas par défaut ;
- `data-theme` reste absent de `<html>`, et le cookie de langue du site n'est pas réécrit.

## § 9 — Sur appareil réel

Ce qu'un poste de travail ne peut pas éprouver : installer sur un Android puis un iPhone, créer le
compte depuis l'application installée, **vérifier que `client_kind` reste `app` après une nuit**, et
dérouler § 2 en 3G lente.

**Sur iPhone, le moment qui casse le plus facilement** : créer le compte depuis l'application
installée, ouvrir le courriel, toucher le lien — il s'ouvre dans Safari, **pas** dans l'application,
qui ne partage pas son stockage. La page doit dire « Adresse confirmée — retournez dans Guide Négo ».
Revenir à l'application par le sélecteur d'applications : elle doit **relire l'état du compte toute
seule**, sans qu'on touche à rien, et passer à la suite du parcours. Sur Android, l'application
installée ouvre le lien elle-même et enchaîne sans détour.

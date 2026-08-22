# Quickstart — Média + Engagement (B6)

**Fonctionnalité** : [spec.md](spec.md) · **Plan** : [plan.md](plan.md) · **Routes** : [contracts/routes.md](contracts/routes.md)

Comment lancer ces deux modules, les éprouver à la main, et savoir qu'ils tiennent. Ce fichier ne contient aucun code d'implémentation : c'est un guide de mise en route et de vérification.

---

## Préalables

```bash
cp .env.example .env                                   # si ce n'est pas déjà fait
docker compose -f ops/docker-compose.dev.yml down -v   # LE SCHÉMA A CHANGÉ : il faut détruire
docker compose -f ops/docker-compose.dev.yml up -d
make garage-init
```

> **`down -v` est obligatoire cette fois.** B6 ajoute **une fonction de lecture** à `110_engagement.sql` (R17) — `engagement.session_reminder_schedule()`. Le schéma n'est chargé qu'au **premier** démarrage du conteneur : sans destruction du volume, la base garde l'ancien schéma **sans le dire**, et la compilation échoue sur une fonction que SQLx ne trouve pas. C'est le piège que `CLAUDE.md` signale, et c'est la première fois depuis B2 qu'il s'applique.

**`make garage-init` importe désormais une clé fixe** lue dans `.env` au lieu d'en créer une aléatoire (R31) : les identifiants S3 survivent à un `down -v`, et le point de contrôle ci-dessous ne casse plus pour une raison sans rapport avec le code.

### Variables nouvelles

| Variable | Défaut | À quoi elle sert |
|---|---|---|
| `MEDIA_STORAGE` | `s3` | `s3` ou `filesystem`. Les tests d'intégration utilisent `filesystem` (R7) |
| `MEDIA_FS_ROOT` | `./.media` | Racine du stockage sur fichiers, quand il est choisi |
| `MEDIA_MAX_UPLOAD_BYTES` | `209715200` | Plafond absolu d'un dépôt — 200 Mio, le poids du fond vidéo de la vitrine |
| `MEDIA_SCANNER` | `none` | `none` ou `clamd`. `none` inscrit son nom dans la trace d'analyse (R13) |
| `MEDIA_CLAMD_ADDR` | — | Adresse du démon d'analyse, quand il est choisi |
| `MEDIA_SCAN_MAX_BYTES` | `52428800` | Au-delà, verdict « non pris en charge » plutôt qu'une analyse de cinq minutes |
| `MEDIA_PURGE_INTERVAL` | `6h` | Période du travail de purge |
| `MEDIA_RECONCILE_INTERVAL` | `24h` | Période de la réconciliation des compteurs |
| `ENGAGEMENT_PARTITION_INTERVAL` | `24h` | Période de préparation des partitions mensuelles |
| `MAIL_WEBHOOK_TOKEN` | *(vide)* | Jeton d'entrée du relais. **Vide = la route d'ingestion n'est pas montée** (R30) |

Les cinq variables `S3_*` existent dans `.env.example` **depuis le 16/08** : rien à ajouter, seulement à renseigner depuis `make garage-init`.

`DATABASE_URL` doit être renseignée et la base démarrée pour **compiler** : SQLx vérifie les requêtes à la compilation.

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API, http://localhost:8080
cd backend && cargo run -p worker     # LE WORKER EST INDISPENSABLE, DEUX FOIS PLUTÔT QU'UNE
```

**Sans worker, deux choses n'arrivent jamais, et aucune ne produit d'erreur** :

1. **un fichier déposé ne devient jamais servable** — il reste « en cours de traitement », ses dimensions ne sont pas relevées, ses déclinaisons n'existent pas, et l'image ne s'affiche nulle part ;
2. **aucun rappel ne part** — les lignes sont créées, les travaux mis en file, et rien ne les exécute.

Ce sont les deux premiers symptômes à connaître. Ils ressemblent tous les deux à un défaut du code.

`GET /api/docs` rend l'OpenAPI engendrée : les trente-trois routes de ces deux modules y figurent avec leurs seize codes d'erreur.

Interfaces locales : **Mailpit** `http://localhost:8025` (les courriels partent au relais du site, qui les remet à Mailpit) · **Jaeger** `http://localhost:16686` · **Garage** par `make garage-info`.

---

## Éprouver les parcours à la main

### 1. Un fichier arrive, et il n'occupe la place qu'une fois

Annoncer d'abord, sans envoyer : `POST /media/assets/precheck` avec le nom, le type, le poids, l'entité porteuse et le rôle. La réponse dit **ce qui se passerait**.

Déposer ensuite : `POST /media/assets` en multipart, avec le fichier. La réponse porte un identifiant d'objet.

**Vérifier sur le stockage lui-même** — c'est ce qu'aucun test automatisé ne fait :

```bash
make garage-info                       # le bucket porte-t-il l'objet ?
docker exec epavillon-garage /garage bucket info epavillon
```

Redéposer **le même contenu sous un autre nom**. La réponse doit rendre **le même identifiant d'objet**, et `SELECT count(*) FROM media.assets` ne doit pas avoir bougé.

**Le point de contrôle du quota** : ramener le plafond d'une organisation à quelques kilo-octets par `PUT /admin/media/quotas/…`, puis tenter un dépôt. Le refus doit porter **les trois chiffres** — plafond, consommé, restant — et sortir sous le même code que la pré-vérification.

### 2. Le fichier devient servable

Juste après le dépôt, demander l'image rattachée à l'entité : **l'adresse de l'original doit déjà être là**, et la liste des déclinaisons **vide mais présente**. C'est ce qui empêche un trou à l'écran.

Attendre le passage du worker, puis `GET /media/assets/{id}/status` : l'état doit être « servable », les dimensions relevées, les déclinaisons prêtes. Relire l'image rattachée : les déclinaisons y sont.

**Le point de contrôle qui compte** : arrêter le worker **avant** de déposer, déposer, constater que rien ne bouge, relancer le worker, constater que le traitement se fait — **et une seule fois**. `SELECT count(*) FROM media.renditions WHERE asset_id = …` doit rendre le nombre de déclinaisons configurées, jamais le double.

### 3. Les trois déclinaisons d'une édition

C'est l'obligation que B3 a laissée. Déposer trois images **aux bonnes formes** — 32:9, 16:9, 1:1 —, puis `PUT /media/attachments` avec les trois affectations en une requête.

Relire `GET /events/{slug}` : les trois déclinaisons doivent y être, résolues. **C'est la route de B3 qui sert de mesure** — si elle les rend, le rattachement a réellement eu lieu.

**Les quatre refus à provoquer, un par un** :

| Ce qu'on tente | Ce qu'on doit lire |
|---|---|
| Un carré comme bandeau | le rapport reçu, le rapport attendu, la tolérance |
| Un PDF comme couverture | le type reçu et les types acceptés |
| Un second bandeau, par l'ajout | « ce rôle n'accepte qu'un seul fichier » |
| Une image sans texte alternatif | le refus **sur le champ** `alt_text` |

Puis retirer une déclinaison (valeur nulle) et **relire l'objet stocké** : il doit exister toujours. C'est l'assertion pour laquelle ce geste existe.

### 4. Le calendrier des rappels — l'écart n° 34

Il faut d'abord de quoi parler : une édition, une séance datée **dans le futur**, une règle de rappel, et des inscrits. Les routes existent toutes depuis B3 et B5.

Poser la règle : `PUT /admin/reminder-rules` avec `event_id` et **quatre décalages**. Inscrire quelques personnes.

`GET /sessions/{id}/reminders` doit rendre **quatre lignes** — pas cent soixante —, chacune portant le nombre d'inscrits, du décalage le plus lointain au plus proche.

**Le point de contrôle qui compte** : lire la réponse **entière**, à l'œil. Aucun nom, aucune adresse, aucun identifiant de personne. C'est ce que le test automatisé vérifie par balayage, et le vérifier à l'œil une fois vaut la peine.

Poser ensuite une règle **de séance** à deux décalages : `GET /sessions/{id}/reminder-rule` doit rendre **deux** décalages et dire que la règle vient de la **séance**. Pas six, pas quatre.

### 5. Le rappel part une fois, et une seule

Dater une séance à **quarante minutes** dans le futur, avec une règle portant un décalage de trente minutes. Inscrire une personne.

Attendre dix minutes — ou avancer l'instant d'envoi à la main en base, ce qui est plus rapide et tout aussi probant.

**Le point de contrôle, en trois temps, celui de B1 réemployé** :

1. worker arrêté, l'heure passe → **rien n'arrive dans Mailpit** ;
2. worker relancé → **un** courriel arrive ;
3. rejouer le travail — `UPDATE platform.jobs SET status='queued' WHERE task='engagement.send_reminder'` → **aucun second courriel**, la ligne de rappel étant déjà partie.

Puis : annuler l'inscription, et vérifier que les rappels restants sont annulés avec leur motif. **La réinscrire**, et vérifier que les rappels **reviennent** — c'est le cas que la clé d'unicité du modèle rend piégeux (R21), et il ne se voit qu'ici.

### 6. Les modèles, et le piège du lien

Écrire une révision dont le corps contient `<a href="{{lien_participation}}">Rejoindre</a>` **et** un `<script>`.

Après enregistrement, relire la révision : le script a disparu, **et le `href` porte encore sa variable**. Si la variable a disparu, le lien du courriel sera mort — c'est le piège nommé en R26, et il ne se voit qu'à la réception.

Publier, envoyer un rappel, lire le courriel dans Mailpit : le lien doit être **résolu**, pas laissé tel quel.

Revenir à la révision précédente : le courriel suivant doit reprendre l'ancien texte.

### 7. Une adresse morte

`POST /admin/email-suppressions` sur l'adresse d'une personne inscrite, puis provoquer un rappel : **rien n'arrive dans Mailpit**, et le rappel porte le motif « adresse supprimée ».

**Le point de contrôle qui prouve la garde d'enveloppe** : provoquer une **invitation d'organisation** (B2) vers cette même adresse. Rien ne doit partir non plus — alors qu'aucune ligne du module Organisations n'a été modifiée. C'est la seule façon de vérifier l'écart n° 133 (R24).

---

## Ce qui doit rester vrai — vérifications mécaniques

```bash
cd backend

# Aucune arête entre les deux nouveaux crates, ni vers un autre module
cargo tree -p media       | grep -E '^\S*(engagement|identity|org|event|programme)' && echo 'ÉCHEC' || echo 'OK'
cargo tree -p engagement  | grep -E '^\S*(media|identity|org|event|programme)'      && echo 'ÉCHEC' || echo 'OK'

# Aucune écriture hors du schéma du module — la promesse la plus forte de ce jalon
grep -rnE 'INSERT INTO (identity|org|event|programme|reference|content)\.|UPDATE (identity|org|event|programme|reference|content)\.' \
     crates/modules/media/src crates/modules/engagement/src && echo 'ÉCHEC' || echo 'OK'

# Aucun fichier au-dessus de mille lignes
find crates -name '*.rs' -exec wc -l {} + | awk '$1 > 1000 && $2 != "total"' | grep . && echo 'ÉCHEC' || echo 'OK'
```

> **Le contrôle d'écriture hors schéma est plus strict ici que dans les modules précédents.** B3 écrit dans `reference`, B5 dans `identity` ; ces deux modules-ci n'écrivent **nulle part** hors de leur propre schéma. Le grep porte donc aussi sur `reference` et `content`, et il doit rester vide.

Puis la porte de qualité complète :

```bash
make check     # DÉTRUIT la base et la recharge de zéro — obligatoire ici, le schéma a changé
```

`check-db` vérifie les seize schémas, le rapport de frontières vide, et le rafraîchissement des projections analytiques. `check-back` exécute `cargo fmt --check`, `clippy -D warnings` et `cargo test --workspace`.

---

## Les huit assertions qui ne se déduisent d'aucune autre

Celles qu'un test doit porter parce qu'aucune relecture de code ne les remplace.

| # | Assertion | Comment on la mesure |
|---|---|---|
| 1 | **Le service n'émet ni n'enfile en double** | compter `outbox_events` et `platform.jobs` après un dépôt : **une** ligne de chaque, pas deux |
| 2 | **Une inscription créée à l'état « inscrit » matérialise ses rappels** | c'est le chemin courant, et c'est celui qu'une lecture du commentaire du modèle aurait cassé (écart n° 126) |
| 3 | **Le calendrier ne porte aucun nom** | balayer la charge utile **sérialisée** entière, pas champ par champ : c'est le champ qu'on ajoutera demain qui compte |
| 4 | **Une inscription reprise reçoit à nouveau ses rappels** | annuler, réinscrire, relire les lignes : `pending`, pas `cancelled` (R21) |
| 5 | **Un `href` porteur d'une variable survit à l'assainissement** | relire le corps enregistré et y chercher `{{` (R26) |
| 6 | **Un objet rattaché ne se supprime pas** | rattacher à deux entités, supprimer, lire le refus **et le nombre** (écart n° 128) |
| 7 | **La table blanche n'a aucune ligne sans garde** | lire `media.attachable_roles` **en base** et échouer sur toute ligne non associée (R15) |
| 8 | **Les trente-trois routes répondent** | les frapper sur la vraie application — le test qui a attrapé trois routes muettes en B2 et un scope non monté en B4 |

---

## Ce que ce jalon ne fait pas, et qu'il ne faut pas chercher

- **Aucun écran de notifications** n'existe côté site : les routes répondent, rien ne les appelle encore.
- **Les courriels de B1 et B2 gardent leur texte en dur.** Ils passent par la garde et par le journal, ils ne passent pas par les modèles administrables (H6).
- **Aucun rappel d'échéance d'appel** : le périmètre de ses destinataires n'est pas défini (H10).
- **Commentaires, réactions, messagerie, mises en relation, infolettres** : hors périmètre (H9). Aucune ligne de code ne nomme ces tables.
- **Ni WebP ni AVIF** parmi les déclinaisons : le modèle rend leur ajout mécanique le jour venu (R12).

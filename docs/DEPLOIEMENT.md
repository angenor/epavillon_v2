# Déploiement

Comment la v2 arrive en ligne, et pourquoi elle y arrive par ce chemin-là.

Pour l'environnement de développement, voir [ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md).

---

## 1. La contrainte, et ce qu'elle n'était pas

Deux serveurs, et un seul porte le nom de domaine.

| | Où | Ce qu'il sait faire |
|---|---|---|
| **cPanel** — `epavillonclimatique.francophonie.org` | `68.168.118.201` | HTML, CSS, JS, PHP. Apache 2.4. **Pas de Node, pas de Passenger.** Sert la v1 |
| **Serveur applicatif** — `epavillon.mefali.com` | `173.209.36.111` | **cPanel/WHM aussi**, mais avec root et SSH. Apache 2.4.68 (service `httpd`), CSF, 16 Go, 8 cœurs. Docker installé le 02/09. Héberge déjà `financedurable.francophonie.org` — Node sous PM2 sur le port 3000, MongoDB, proxifié par Apache. **La licence WHM n'autorise qu'un seul compte cPanel** : le domaine d'ePavillon n'y est donc pas déclaré, son vhost vit dans un include |

Le nom de domaine est un sous-domaine de `francophonie.org`, dont la zone DNS est tenue par l'OIF —
serveurs `agecop` et `palabre`, sans délégation vers le cPanel. **Aucun enregistrement ne peut être
ajouté sans passer par eux**, et le délai n'est pas compatible avec le calendrier du projet.

Deux hypothèses ont été admises sans mesure, et toutes deux étaient fausses :

- *« Seul le serveur du site a le droit d'émettre du courriel. »* Émettre au nom d'un domaine ne
  demande pas de l'héberger : il faut un compte et le port 587 en sortie. Mesuré le 01/09, le serveur
  du domaine répond, négocie TLS et annonce `250-AUTH PLAIN LOGIN`.
- *« Apache mutualisé ne peut pas relayer. »* Le flag `[P]` de `mod_rewrite` est autorisé en
  `.htaccess` sur cet hébergement. Éprouvé le 01/09 : 200, contenu relayé.

C'est la seconde qui commande toute l'architecture ci-dessous.

---

## 2. Le chemin d'une requête

```
navigateur ──HTTPS──> epavillonclimatique.francophonie.org/v2/…   (Apache, cPanel institutionnel)
                          │  .htaccess : RewriteRule … [P]
                          │
                          └──HTTPS──> epavillon.mefali.com/v2/…   (Apache, VPS)
                                         │  include userdata/ : ProxyPass
                                         ├─ /v2/api/* ──> 127.0.0.1:8080  (préfixe retiré)
                                         └─ /v2/*     ──> 127.0.0.1:3100  (préfixe conservé)
```

**Aucun serveur web dans la pile Docker**, et c'est le point qui a fait tomber
le plan initial : le VPS est un cPanel, son Apache tient déjà 80 et 443 pour un
site en production. Un Caddy qui les réclamerait ne démarrerait pas — ou pire,
démarrerait et couperait ce site. Les deux conteneurs publient sur `127.0.0.1`
et rien d'autre ; Apache termine le TLS, avec le certificat qu'AutoSSL a émis.
Le port 3000 étant pris par l'application de `financedurable`, le site va sur
3100.

Le greffage passe par un **include `userdata/`** et jamais par un `VirtualHost`
écrit à la main : cPanel régénère `httpd.conf` — à la création d'un compte, au
renouvellement d'un certificat, à une mise à jour — et un vhost manuel y
disparaît sans prévenir. Le même mécanisme sert déjà à `financedurable` sur ce
serveur.

**Le visiteur ne voit qu'une adresse.** `epavillon.mefali.com` est une adresse de transport : elle
n'apparaît ni dans la barre du navigateur, ni dans un lien, ni dans un courriel.

Elle existe pour une seule raison : **chiffrer le segment entre les deux serveurs**. Relayer vers
l'adresse IP en clair ferait traverser l'Internet public aux cookies de session, à l'insu du
navigateur — qui voit, lui, une connexion sécurisée de bout en bout et n'a aucun moyen d'apprendre
qu'elle ne l'est pas.

Deux conséquences que cette forme donne gratuitement, et qui auraient chacune coûté cher autrement :
les cookies restent **first-party**, puisque le navigateur ne connaît qu'un hôte ; et il n'y a **pas
de CORS**, puisqu'il n'y a qu'une origine.

---

## 3. Le préfixe `/v2`, et les trois endroits où il se glisse

La v1 tourne à la racine et doit continuer. La v2 vit donc sous `/v2` le temps de la recette.

Un préfixe n'est pas un détail d'hébergement : il traverse l'application. **Trois endroits le
portent, et deux d'entre eux échouent en silence si on les oublie.**

**Le chemin des cookies.** Servi sous `/v2`, le navigateur appelle `/v2/api/auth/refresh` ; un cookie
posé sur `/api/auth` ne lui est jamais renvoyé. La connexion réussit, la navigation fonctionne quinze
minutes — la durée du jeton d'accès —, puis tout le monde se retrouve déconnecté. Aucune erreur,
aucune trace en journal. Fermé par `Config::app_base_path` et le test
[`cohabitation_sous_prefixe.rs`](../backend/crates/modules/identity/tests/cohabitation_sous_prefixe.rs).

**L'origine autorisée.** Un navigateur n'annonce jamais de chemin dans son en-tête `Origin` :
comparer à l'URL complète refuserait **toute écriture**. Fermé par `Config::app_public_origin`.

**Les chemins de `public/`.** Un `src="/logos/…"` écrit en dur vise la racine du domaine — donc, ici,
la v1. Le symptôme est un logo manquant, la cause est ailleurs. Fermé par `assetUrl()`.

Ces trois valeurs **dérivent toutes d'`APP_PUBLIC_URL`**, et aucune ne se règle à part : deux valeurs
à tenir d'accord finissent par diverger, et le jour où elles divergent, plus rien ne fonctionne sans
que rien ne l'explique.

---

## 4. Le script

`./deploy.sh` porte tout ce qui suit. `./deploy.sh` sans argument liste ses
commandes.

> **Il n'est pas versionné**, et ne doit pas l'être : il porte l'adresse du
> serveur, son port SSH et les chemins d'un hébergement qui abrite aussi la
> plateforme d'un tiers. Une carte de l'infrastructure n'a pas à voyager avec
> le code. **Les sections 5 à 7 ci-dessous font foi** : elles décrivent chaque
> geste à la main, et suffisent à réécrire le script si la machine qui le porte
> disparaît.

| | |
|---|---|
| `update` | le geste courant : envoi du code, reconstruction du site et de l'API, redémarrage |
| `deploy` | tout, y compris la base et le stockage |
| `status` · `sante` · `logs <service>` | surveiller |
| `backup` · `restore <fichier>` | `pg_dump` compressé, rapatrié dans `sauvegardes/` |
| `vhost` · `ssl` | l'infrastructure Apache et le certificat |
| `reseau` | après un rechargement du pare-feu, si la pile est devenue injoignable |

**Chaque commande qui touche l'infrastructure mesure `financedurable.francophonie.org`
avant et après, et s'arrête s'il a bougé.** C'est la contrainte n° 1 de cette
machine, et elle vaut plus que la rapidité d'un déploiement.

Les sections qui suivent décrivent ce que le script fait, pour le jour où il
faudra le faire à la main.

## 5. Mise en place, geste par geste

### 4.1 DNS — chez ton registrar, pas chez l'OIF

```
epavillon.mefali.com.  IN  A  173.209.36.111
```

Attendre la propagation avant l'étape suivante : Caddy ne peut obtenir son certificat que si le nom
résout déjà vers la machine.

### 4.2 Boîte d'expédition — dans cPanel

Créer `ne-pas-repondre@epavillonclimatique.francophonie.org`, puis vérifier l'authentification **avant**
de renseigner quoi que ce soit :

```bash
printf '\0%s\0%s' 'ne-pas-repondre@epavillonclimatique.francophonie.org' 'MOT_DE_PASSE' | base64
openssl s_client -starttls smtp -connect epavillonclimatique.francophonie.org:587 -crlf
# puis, dans la session : EHLO test  /  AUTH PLAIN <la chaîne base64>
```

`235 Authentication succeeded` : c'est bon. `535` : l'identifiant n'est pas au format attendu — certains
serveurs veulent `boite+domaine.org` plutôt que `boite@domaine.org`.

### 4.3 Serveur applicatif

Docker a été installé le 02/09 depuis le dépôt officiel, **sans aucune mise à
jour globale du système** — on n'installe que ce qu'on nomme, pour ne toucher à
rien de ce que cPanel tient. Le pare-feu a été prévenu : `DOCKER = "1"` dans
`/etc/csf/csf.conf`. **Sans ce réglage, le réseau Docker tomberait au prochain
rechargement de CSF** — des jours plus tard, sans cause apparente, et sans que
`financedurable` en souffre, ce qui rend le diagnostic d'autant plus long. Les
règles d'origine sont sauvegardées dans `/root/avant-epavillon/`.

Le domaine `epavillon.mefali.com` doit exister **dans cPanel** — sans cela,
aucun vhost, donc aucun certificat et aucun include possible.

```bash
git clone <dépôt> /opt/epavillon && cd /opt/epavillon
cp .env.prod.example .env.prod        # puis renseigner tout ce qui est marqué À REMPLIR
docker compose --env-file .env.prod -f ops/docker-compose.prod.yml up -d --build
```

Puis le stockage objet, qui refuse toute écriture tant que son layout n'est pas assigné :

```bash
make garage-init                       # reporter les deux clés rendues dans .env.prod
docker compose --env-file .env.prod -f ops/docker-compose.prod.yml up -d api worker
```

Vérifier avant d'aller plus loin :

```bash
curl -i https://epavillon.mefali.com/v2/api/health     # 200, et un certificat valide
```

### 4.4 VPS — greffer le proxy Apache

```bash
USER=<le compte cPanel du domaine>
D=/etc/apache2/conf.d/userdata/ssl/2_4/$USER/epavillon.mefali.com
mkdir -p $D && cp ops/apache-epavillon.conf $D/epavillon-proxy.conf
/scripts/ensure_vhost_includes --user=$USER
apachectl configtest && systemctl reload apache2
```

`configtest` avant le rechargement n'est pas une politesse : une directive
fautive fait échouer le démarrage d'Apache, **et emporte alors le site en
production avec elle**.

```bash
curl -i https://epavillon.mefali.com/v2/api/health
```

### 4.5 cPanel institutionnel — le relais public

Déposer [`ops/htaccess-v2.conf`](../ops/htaccess-v2.conf) en `public_html/v2/.htaccess`.

**Ne pas toucher au `.htaccess` de la racine** : il sert la v1.

```bash
curl -i https://epavillonclimatique.francophonie.org/v2/api/health
```

---

## 6. Ce qui reste à vérifier une fois en ligne

Trois choses qu'aucun test local ne peut dire, dans l'ordre où elles font mal :

1. **Le premier courriel réel.** Un serveur de soumission n'accepte le plus souvent qu'un expéditeur
   appartenant au compte authentifié : `SMTP_FROM` différent de `SMTP_USERNAME` donne un 550 au
   premier message. Et l'hébergement mutualisé impose une limite horaire d'envoi, à connaître avant
   le jour d'un appel à propositions.
2. **Le téléversement d'un gros média.** La configuration accepte 200 Mio ; Apache mutualisé impose
   souvent moins, et le refus vient du relais, pas de l'application. Si ça bloque, le dépôt de médias
   — une opération du back-office, où l'adresse affichée n'a aucune importance — peut passer
   directement par `epavillon.mefali.com`.
3. **Une session complète**, connexion puis navigation au-delà de quinze minutes. C'est ce qui
   éprouve le chemin des cookies pour de vrai.

---

## 7. Le jour de la bascule à la racine

Quand l'OIF aura répondu, ou quand la v1 pourra être remplacée :

1. `APP_PUBLIC_URL` sans `/v2`, `NUXT_APP_BASE_URL=/`, `NUXT_PUBLIC_API_BASE=/api`
2. **Reconstruire l'image du site** — la base préfixe chaque URL d'asset écrite dans le HTML, et la
   changer sans reconstruire ne les réécrit pas
3. Le `.htaccess` passe à la racine de `public_html`, `RewriteBase /` et la cible sans `/v2/`
4. Dans le Caddyfile, `handle /api/*` et `handle /*`

Aucun code ne change. Le préfixe n'existe qu'en configuration, et c'est le seul point de ce montage
qui méritait d'être payé d'avance.

---

## 8. Ce que le serveur a appris de nous, et nous de lui (02/09)

Sept obstacles, tous rencontrés une fois, tous silencieux ou trompeurs. Ils sont
consignés ici parce qu'aucun ne se déduit d'une documentation.

**Le service Apache s'appelle `httpd`, pas `apache2`.** `systemctl reload apache2`
échoue sur « Unit not found » — mais `httpd -S` lit la configuration **sur le
disque**. Un vhost peut donc y figurer, paraître parfaitement chargé, et n'avoir
jamais atteint la mémoire du serveur. On cherche alors une faute dans le fichier
pendant que le rechargement n'a simplement pas eu lieu.

**Les vhosts de cPanel sont déclarés sur `173.209.36.111:80`, jamais sur `*:80`.**
Apache traite les deux comme des ensembles séparés : un vhost générique n'est
jamais consulté pour une adresse qui a les siens. Le symptôme est la page 404 de
cPanel, qui fait chercher du côté des chemins et des droits.

**Le domaine n'a pas à exister dans cPanel.** `conf.d/includes/post_virtualhost_global.conf`
est référencé par la configuration engendrée et jamais réécrit. C'est ce qui
permet de se passer d'un compte — la licence n'en autorise qu'un — et de ne rien
modifier au compte de `financedurable`. Le certificat vient donc de certbot, en
mode `--webroot` sur `/var/www/certbot`, et non d'AutoSSL, qui ne couvre que ce
que le panneau connaît.

**CSF efface les chaînes de Docker à chaque rechargement.** Le réglage
`DOCKER = "1"` de `csf.conf` ne suffit pas : il ne couvre que `172.17.0.0/16`,
le pont par défaut. Deux gestes le corrigent — `systemctl restart docker` après
un `csf -r`, qui recrée les chaînes, et surtout `/etc/csf/csfpost.sh`, exécuté
par CSF après chaque reconstruction, qui autorise `172.28.0.0/16`. **Sans ce
fichier, la pile tombe au prochain rechargement du pare-feu**, spontané ou
déclenché par lfd, et le symptôme — une connexion acceptée puis refermée — fait
suspecter l'application, à qui rien n'est parvenu.

**`set -o pipefail` plus un filtre qui sort tôt tue un script sans un mot.**
`garage layout show | awk '… exit'` ferme le tube, `garage` reçoit un SIGPIPE,
le code de retour devient non nul, et `set -e` arrête tout — sans que personne
n'ait échoué au sens habituel. On capture la sortie d'abord, on la filtre après.

**Un nœud Garage assigné mais non validé apparaît dans « STAGED ROLE CHANGES ».**
Y chercher son identifiant fait croire que le layout est appliqué, et un script
idempotent saute alors l'application à chaque relance : le stockage reste
indéfiniment en refus d'écriture. On teste la version **appliquée**.

**Deux bases d'API, et elles ne peuvent pas être la même.** Le navigateur appelle
un chemin — `/v2/api` —, ce qui garde une seule origine, donc des cookies de
première partie et aucun CORS. Mais un chemin n'a pas d'origine : au rendu
serveur, il désigne Nitro lui-même, qui ne connaît pas cette route. Les pages
publiques se rendaient vides, sans une ligne d'erreur, **et d'abord pour les
moteurs de recherche** — c'est-à-dire là où personne ne regarde. D'où
`NUXT_API_BASE_SERVER`, l'adresse interne entre conteneurs. La preuve que la
correction tient se lit à l'œil : l'accueil est passé de 117 Ko de squelettes de
chargement à 37 Ko portant « Aucune édition », qui est la réponse **exacte** de
l'API sur une base sans données.

---

## 9. État au 02/09

Déployé et vérifié sur `https://epavillon.mefali.com/v2` :

| | |
|---|---|
| Six conteneurs | `api`, `worker`, `front`, `postgres`, `valkey`, `garage` |
| Schéma | chargé au premier démarrage du volume |
| Stockage objet | layout appliqué, bucket `epavillon`, clé engendrée dans `.env.prod` |
| TLS | certbot, expire le 01/12/2026, renouvellement automatique armé |
| Rendu serveur | atteint l'API ; la page d'accueil rend ses états vides, qui sont exacts |
| `financedurable.francophonie.org` | **200 à chaque étape**, vérifié avant et après chaque geste |

Ce qui manque encore :

1. **Le relais du cPanel institutionnel** — `ops/htaccess-v2.conf` à déposer en
   `public_html/v2/.htaccess`. Tant qu'il n'y est pas, la v2 n'est joignable que
   par son adresse de transport.
2. **La boîte d'expédition** — `SMTP_USERNAME` et `SMTP_PASSWORD` sont vides.
   `SMTP_FROM` est renseignée pour laisser l'API démarrer, mais **aucun courriel
   ne part** : sans authentification, le serveur de soumission refuse le relais.
   Les envois échouent, sont rejoués, puis meurent en file.
3. **Aucune donnée métier** — ni édition, ni appel. À créer depuis le back-office,
   ou à semer.

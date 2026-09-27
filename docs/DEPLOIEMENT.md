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
                                         ├─ /v2/api/*   ──> 127.0.0.1:8080  (préfixe retiré)
                                         ├─ /v2/media/* ──> 127.0.0.1:3120  (relais Garage)
                                         └─ /v2/*       ──> 127.0.0.1:3100  (préfixe conservé)
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

1. ~~Le premier courriel réel.~~ **Fait le 02/09** — voir le § 10.
2. **Le téléversement d'un gros média.** La configuration accepte 200 Mio ; Apache mutualisé impose
   souvent moins, et le refus vient du relais, pas de l'application. Si ça bloque, le dépôt de médias
   — une opération du back-office, où l'adresse affichée n'a aucune importance — peut passer
   directement par `epavillon.mefali.com`.
3. **Une session complète**, connexion puis navigation au-delà de quinze minutes. C'est ce qui
   éprouve le chemin des cookies pour de vrai.
4. **La garde de Guide Négo**, après **chaque** mise en ligne :

   ```bash
   cd frontend && node scripts/guide-nego-verifier-garde.mjs https://<domaine>/v2/guide-nego/
   ```

   Le script lit `sw.js`, en tire la liste des fichiers gardés et les demande un par un. Il échoue
   sur toute réponse autre qu'un 200 franc — **une redirection comprise**, qu'un navigateur refuse
   ensuite de servir à une navigation. C'est une panne autrement muette : l'installation du service
   worker est tout ou rien, et une seule adresse cassée — la route des traductions derrière Apache,
   par exemple — suffit à ce qu'aucun téléphone ne garde l'application, sans qu'aucun message ne le
   dise. `./deploy.sh` le lance tout seul après `update` et `deploy`.

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
2. ~~La boîte d'expédition.~~ **Faite le 02/09** : l'API envoie, éprouvé de bout
   en bout — voir le § 10.
3. **Aucune donnée métier** — ni édition, ni appel. À créer depuis le back-office,
   ou à semer.
4. ~~Les médias ne sont pas servis.~~ **Fait le 04/09** — voir le § 12.

---

## 10. Le courriel : qui a le droit d'écrire au nom du domaine (02/09)

**L'expéditeur porte le SOUS-DOMAINE, et cette lettre-là décide de tout.**

```
francophonie.org                     v=spf1 include:spf.protection.outlook.com … -all
_dmarc.francophonie.org              v=DMARC1; p=reject
epavillonclimatique.francophonie.org v=spf1 +mx +a +ip4:173.209.54.36 -all
```

Le domaine parent est sur Microsoft 365 et n'autorise que ses propres relais ; le
`-all` interdit tout le reste, et `p=reject` fait **refuser** — pas classer en
indésirable — les messages qui échouent. Le serveur applicatif (`173.209.36.111`)
n'y figure pas.

Le sous-domaine, lui, publie son propre SPF, et son `+a` autorise l'adresse du
domaine — c'est-à-dire `68.168.118.201`, le serveur qui héberge la plateforme.
C'est une configuration délibérée : **seule la machine du site peut écrire en son
nom**. D'où le montage retenu, et le seul qui fonctionne sans rien demander à
l'OIF : l'API se connecte à ce serveur en client authentifié, et c'est lui qui
émet.

Une adresse en `@francophonie.org` serait donc rejetée, quelle que soit la
machine qui l'envoie de notre côté. La faire accepter demanderait que l'OIF
ajoute une IP à son SPF, ou fournisse un relais joignable depuis l'Internet —
`courriels.francophonie.org`, transmis le 02/09, **n'existe pas dans le DNS
public** : ni A, ni CNAME. C'est vraisemblablement un nom interne à leur réseau.

**Le port 465 est fermé sur ce serveur**, malgré ce qu'affiche cPanel dans
« Connect Devices » : la connexion est refusée. C'est 587 avec STARTTLS qui
répond, ce que la configuration prend par défaut.

Éprouvé de bout en bout le 02/09 : une inscription par `POST /auth/register` a
posé son travail différé, le worker l'a pris, et `identity.send_verification_email`
est passé en `succeeded` au premier essai. Ce n'est pas un envoi de test à côté
de l'application — c'est l'application qui a écrit.

Reste à connaître, et qui ne se mesure qu'à l'usage : **la limite horaire
d'envoi** de l'hébergement mutualisé, à vérifier avant le jour d'un appel à
propositions.

---

## 11. L'adresse de recette, et pourquoi elle garde son préfixe (02/09)

**Le relais depuis le domaine institutionnel ne fonctionne pas encore**, et le
diagnostic est arrêté : `mod_rewrite [P]` vers une cible **HTTPS** exige
`SSLProxyEngine On`, directive qu'Apache n'accepte qu'en configuration de
serveur — jamais en `.htaccess`. Mesuré depuis le compte, avec deux relais de
test identiques à la cible près :

| cible | code |
|---|---|
| `http://example.com/` | **200** |
| `https://example.com/` | **500** |

Le proxy fonctionne donc ; c'est le chiffrement de la requête sortante qui
manque. Un ticket est ouvert chez l'hébergeur pour l'ajouter — une ligne, sans
effet sur le reste du site. Le `.htaccess` déposé attend sous le nom
`.htaccess.desactive` dans `public_html/v2/` ; le jour de la réponse, un `mv`
suffit.

Relayer en clair a été écarté : les cookies de session traverseraient l'Internet
public entre les deux serveurs, et le navigateur, qui voit une connexion
chiffrée de bout en bout, n'aurait aucun moyen de le signaler.

**La recette vit donc sur `https://epavillon.mefali.com/v2`, préfixe compris.**
Servir à la racine serait plus présentable et ne coûterait qu'une reconstruction
— mais **le préfixe est la partie fragile du montage** : il a produit à lui seul
les trois défauts silencieux du § 3. Une recette servie à la racine n'en
exercerait aucun, et validerait une configuration qui n'est pas celle qui
partira en production. La laideur de l'adresse est le prix d'un test qui porte
sur la vraie chose.

`https://epavillon.mefali.com/` redirige vers `/v2/` : tant que la recette vit
là, taper le nom nu doit aboutir quelque part.

**Et `APP_PUBLIC_URL` doit suivre — elle ne suivait pas.** Le serveur portait
encore `https://epavillonclimatique.francophonie.org/v2`, alors que l'adresse de
recette est l'autre. Or cette valeur donne à l'API **la seule origine acceptée
en écriture** : mesuré le 04/09, un `POST /auth/login` depuis
`https://epavillon.mefali.com` rendait `403 IDENTITY_ORIGIN_REJECTED`.
**Personne ne pouvait se connecter en ligne**, et le message ne nommait pas la
cause côté écran. Corrigé le 04/09 — la même requête rend maintenant 200 avec
`invalid_credentials`, c'est-à-dire la réponse métier.

Trois choses en dérivent, et toutes trois étaient fausses : l'origine acceptée,
la base des liens envoyés par courriel, et depuis le § 12 l'adresse publique des
médias. Le jour du relais institutionnel, cette valeur reprend l'adresse
définitive, et `ops/init-garage-prod.sh` est à rejouer pour que les médias
suivent.

Le seul reste bâti sur l'ancienne valeur est `NUXT_PUBLIC_SITE_URL`, figé à la
construction de l'image du site : il ne sert qu'aux liens `hreflang` et aux URL
canoniques. Les laisser pointer vers l'adresse définitive pendant la recette est
sans effet sur le fonctionnement — et plutôt souhaitable, l'adresse de transport
n'ayant pas à être indexée.

Ce choix se renverse le jour où la bascule s'éloigne durablement — changer
`NUXT_APP_BASE_URL`, `NUXT_PUBLIC_API_BASE`, retirer le `/v2` du vhost, et
reconstruire l'image du site. C'est le même travail à quelque moment qu'on le
fasse.

### Le jour du `mv` — ce qu'on vérifie par l'adresse institutionnelle

1. `APP_PUBLIC_URL` reprend l'adresse définitive, et une connexion aboutit
   (réponse métier, jamais `IDENTITY_ORIGIN_REJECTED`).
2. `ops/init-garage-prod.sh` est rejoué, et une image de la vitrine s'affiche.
3. **Un choix fait hors connexion dans Guide Négo survit au retour du réseau.**
   Ouvrir « Mes thématiques » en ligne, passer en mode avion, changer une
   thématique, revenir en ligne : le choix doit être enregistré, **sans** le
   message « Vos thématiques ont changé sur un autre appareil ».
4. **Le PDF d'un document se lit par morceaux à travers le relais** (étape 1b
   de Guide Négo). Une requête par plage sur la route du fichier d'un document
   public publié, par l'adresse institutionnelle :

   ```bash
   curl -s -D - -o /dev/null -H 'Accept-Encoding: br, gzip' -H 'Range: bytes=0-262143' \
     "https://<adresse institutionnelle>/v2/api/negotiation/documents/<id>/file"
   ```

   doit rendre **`206`**, `Content-Range: bytes 0-262143/<taille>`,
   `Accept-Ranges: bytes`, et **aucun `Content-Encoding`** autre qu'`identity`.
   Puis le guide s'ouvre dans Guide Négo, première page avant le fichier
   entier.

La troisième vérification tient à un détail mesuré le 22/09 : **ce relais
compresse ses réponses en brotli**, et Apache, réglé par défaut
(`BrotliAlterETag` / `DeflateAlterETag AddSuffix`), ajoute alors « -br » ou
« -gzip » à l'`ETag`. L'empreinte revient ainsi réécrite en `If-Match`. L'API
n'en compare que les 32 caractères hexadécimaux qu'elle a émis
(`kernel::empreinte`), `W/` et suffixe de relais retirés : c'est
elle qui compare ce qui a du sens, puisqu'on ne tient pas la configuration de
l'hébergeur. Mais ni le développement ni la recette ne compressent : **seul ce
geste, par cette adresse, prouve que la comparaison tient** — un échec rendrait
`412` à chaque choix fait sans réseau, et abandonnerait l'intention avec un
message faux.

La quatrième tient au même relais : **une réponse partielle compressée casse
le chargement par morceaux**. pdf.js renonce aux plages dès qu'une réponse
porte un `Content-Encoding`, et relit alors le fichier entier avant la
première page. L'API pose donc `Content-Encoding: identity` et
`Cache-Control: no-transform` sur la route du fichier, qu'Apache respecte ;
seule cette requête, par cette adresse, prouve que le relais les respecte
aussi. Un échec se règle chez l'hébergeur (exclure `application/pdf` de la
compression), pas dans l'application.

---

## 12. Les médias, et pourquoi ils vivent sur l'origine du site (04/09)

Le défaut s'est manifesté en local — « impossible de téléverser une image » —,
et le téléversement n'y était pour rien : **il réussissait**. C'est l'affichage
qui échouait, et la production porte exactement le même trou.

**Deux adresses, pas une.** L'API S3 de Garage exige une signature à chaque
lecture : un navigateur ne peut donc pas y prendre une image. C'est le point
d'accès *web* qui sert les lectures anonymes — et il n'ouvre un bucket que si
celui-ci l'y autorise (`bucket website --allow`), qu'il n'y a aucune raison de
deviner.

**Et il n'écoute qu'en sous-domaine.** `epavillon.web.garage.localhost`, quand
le modèle compose l'adresse d'un objet **en chemin** — `<base>/<bucket>/<clé>`,
ce que sert n'importe quel stockage en « path-style » et ce que ferait un
fournisseur cloud. Un relais traduit l'un en l'autre, et rien d'autre :
`ops/media-proxy.conf`, un nginx de dix lignes, le même fichier en
développement et en production.

**Le chemin retenu est `/v2/media/`**, sur l'origine du site :

```
navigateur ──> …/v2/media/epavillon/2026/09/<clé>.webp
                  │  Apache : ProxyPass /v2/media/ → 127.0.0.1:3120
                  └──> nginx : Host « epavillon.web.garage.localhost » → garage:3902
```

Un sous-domaine `media.…` aurait demandé une entrée DNS à l'OIF — le § 1 dit ce
que cela coûte en délai — et un certificat de plus. Le chemin ne demande rien,
et **suit le domaine du jour** : le relais institutionnel, quand il sera posé,
relaiera `/v2/media/` comme le reste.

La règle de vhost est placée **avant** `/v2/`, pour la même raison que celle de
l'API : Apache retient la première qui correspond, et Nuxt rendrait sa page 404
à la place de chaque image — avec un code 200.

**L'adresse publique est un réglage de base**, `media.public_base_url`, et non
une variable d'environnement. Le modèle y met l'adresse de production définitive
(`docs/database/050_media.sql` § 8), si bien que **tout rechargement du schéma
la remet en silence** ; le seul signe est un `ERR_NAME_NOT_RESOLVED` en console,
le dépôt ayant réussi. `ops/init-garage-prod.sh` la pose désormais d'après
`APP_PUBLIC_URL` — il est donc à rejouer après un rechargement du schéma, et le
jour où l'adresse publique change.

En développement, `make garage-init` fait les deux mêmes gestes, et `check-db`
l'appelle déjà : après un `down -v`, il n'y a rien à refaire à la main.

**Déployé et éprouvé le 04/09.** Un objet de test déposé dans le bucket est
revenu par son adresse publique en 200, `text/plain`, contenu exact — la chaîne
entière, d'Apache à Garage, est donc prouvée et non déduite. L'objet et l'image
utilitaire qui l'a déposé ont été retirés dans la foulée. `financedurable`
mesuré à 200 avant, pendant et après.

Pour refaire ces gestes ailleurs, ou après un rechargement du schéma :

```bash
./deploy.sh push
ssh … "cd /opt/epavillon && docker compose --env-file .env.prod \
       -f ops/docker-compose.prod.yml up -d media-proxy"
./deploy.sh vhost                     # configtest, reload, voisin mesuré
ssh … "cd /opt/epavillon && bash ops/init-garage-prod.sh"
curl -sI https://epavillon.mefali.com/v2/media/epavillon/<une-clé>
```

**Un défaut de `deploy.sh` a été corrigé en chemin** : `scp` nomme le port `-P`
et non `-p` — qui signifie chez lui « préserver les horodatages ». Les deux
appels réutilisaient les options de `ssh` telles quelles, et prenaient donc le
numéro de port pour un fichier à copier. La cible `vhost` échouait sur
« stat local "2243" », après avoir mesuré le voisin et avant d'avoir rien
touché : sans dommage, mais sans effet.

---

## 13. Faire évoluer le schéma d'une base en service (16/09)

**Il n'existe pas d'outil de migration.** `docs/database/` est chargé une seule
fois, à la création du volume ; ensuite, la base garde le schéma qu'elle avait,
**sans le dire**. Recharger efface les données : dès qu'il y a un compte en
production, on migre à la main. Première migration faite le 16/09 — schéma du
02/09 vers celui du 16/09 : dossiers sans résumé ni catégorie unique, appel
avec son texte « après l'envoi », rôle d'organisation attribué avec
l'adhésion.

**La méthode, et chaque étape a une raison :**

1. **Mesurer l'écart, ne pas le déduire.** On exporte le schéma de production
   (`pg_dump --schema-only`) et celui d'une base locale chargée depuis
   `docs/database/`, puis on les compare. L'historique git dit ce qui a changé
   dans les fichiers, pas ce que la base porte réellement.
2. **Éprouver le script sur une copie de la production**, données comprises,
   puis recomparer à la base de référence. Comparer après tri des lignes : une
   base restaurée depuis une sauvegarde réécrit le texte de ses vues, et une
   comparaison brute accuse des écarts qui n'en sont pas.
3. **Sauvegarder** (`./deploy.sh backup`).
4. **Construire les images AVANT de migrer** — l'ancienne version continue de
   servir pendant les minutes de compilation.
5. **Migrer, puis redémarrer aussitôt** (`up -d api worker front`) : l'ancien
   code face au nouveau schéma ne dure que quelques secondes.
6. **Recomparer le schéma de production au modèle.** C'est ce contrôle qui a
   trouvé le seul oubli du 16/09 — un commentaire de colonne dont le texte
   contient un point-virgule, coupé par l'extraction.

**Un écart qui n'en est pas** : `engagement.email_messages_AAAAMM`. Les
partitions de courriels sont créées à la volée par le worker ; la production en
porte que la base de référence n'a pas.

**Où sont les scripts** : `/root/epavillon-migrations/` sur le serveur, hors
du dossier synchronisé — l'envoi du code efface côté serveur ce qui n'existe
pas en local.


---

## 14. Ouvrir Guide Négo (21/09)

L'application mobile est **fermée par `guide_nego.enabled`**, semé à `false`. Tant que le drapeau
est éteint, toute adresse `guide-nego/` sert la page « bientôt disponible » ; le reste du site
ignore l'application.

### La bascule, et sa seule forme valable

```sql
UPDATE platform.feature_flags
   SET is_enabled = true, rollout_percent = 100
 WHERE key = 'guide_nego.enabled';
```

**La réponse doit être `UPDATE 1`.** `UPDATE 0` veut dire que la ligne n'existe pas : elle n'est
semée que par `900_seed.sql`, chargé sur une base neuve, et une base en service ne l'a jamais reçue.
L'application resterait alors fermée, sans un message. Jouer d'abord la migration de 0a —
`specs/008-guide-nego-coquille/migration.sql`, § 15 —, puis rejouer la bascule.

**Les deux colonnes, toujours ensemble. Pas de déploiement progressif** — ce n'est pas une
préférence, c'est ce que la fonction permet :

```sql
-- platform.is_feature_enabled(p_key, p_person_id)
f.rollout_percent = 100
OR (p_person_id IS NOT NULL AND p_person_id = ANY (f.enabled_for))
OR (p_person_id IS NOT NULL AND <tirage sur md5(clé || personne)> < f.rollout_percent)
```

Les deux dernières branches exigent une personne identifiée. **Guide Négo s'ouvre sans compte** :
`p_person_id` y vaut `NULL`, et seule `rollout_percent = 100` peut alors être vraie. Un drapeau
allumé à 50 % n'ouvre donc l'application à *personne* — ni à la moitié des gens, ni à un groupe
d'essai. Même remarque pour `enabled_for` : la liste ne sert à rien ici.

Pour ouvrir à quelques personnes avant tout le monde, la réponse n'est pas le drapeau : c'est
l'adresse, qu'on ne communique pas encore.

### Le cinquième onglet

`negotiation.channels` commande l'onglet « Échanges », et **ne s'allume pas** à ce stade : l'étape
0a ne livre que la coquille. Il obéit à la même règle des deux colonnes, et l'onglet n'apparaît
jamais dans une application fermée — `guide_nego.enabled` prime.

### Fermer

```sql
UPDATE platform.feature_flags SET is_enabled = false WHERE key = 'guide_nego.enabled';
```

Sans redéploiement, comme pour les six modules du site. Mais **la fermeture n'est pas immédiate sur
un téléphone déjà ouvert** : seule une réponse réussie qui dit « éteint » ferme l'application. Une
API injoignable, lente ou en erreur laisse le dernier état lu en place — c'est voulu, une panne en
pleine COP ne doit pas fermer l'application. Un téléphone hors connexion gardera donc l'application
ouverte jusqu'à sa prochaine lecture réussie.

### L'ordre des gestes, le jour de l'ouverture

1. **Déployer** (`./deploy.sh deploy`), drapeau encore éteint — et, la première fois, migrer
   selon le § 15.
2. **Vérifier la garde** — § 6, point 4. Une adresse cassée et aucun téléphone n'installe
   l'application, sans le moindre message.
3. **Basculer le drapeau**, les deux colonnes. **`UPDATE 1`, sinon s'arrêter** (ci-dessus).
4. **Ouvrir `…/v2/guide-nego/` sur un téléphone réel**, l'installer, puis couper le réseau et la
   rouvrir. Le service worker et la garde ne se laissent pas éprouver depuis un poste de travail.
   La liste complète est au § 15.4, étape 0a.

Rien à redémarrer entre 3 et 4 : le drapeau se lit à chaque ouverture.


---

## 15. Mettre en ligne les étapes 0a à 5 (22/09, complété les 24, 25, 26 et 27/09)

Une seule mise en ligne porte les dix premières étapes de Guide Négo : le code de la branche, et
**dix migrations**. Préparée ici, **pas encore exécutée**. Le drapeau reste éteint pendant toute
la mise en ligne : le site ne voit que ce qui le touche (§ 3 ci-dessous), l'application ne s'ouvre
qu'à la recette sur téléphones (§ 4).

### Ce qui change

| Ordre | Migration | Ce qu'elle porte |
|---|---|---|
| 1 | `specs/008-guide-nego-coquille/migration.sql` | Une ligne : le drapeau `guide_nego.enabled`, éteint. Sans elle, la bascule du § 14 rend `UPDATE 0` |
| 2 | `specs/009-guide-nego-compte-admission/migration.sql` | `identity.sessions` dit d'où vient la session (site ou application) ; le vocabulaire des réseaux ; les codes d'invitation, leurs usages, les demandes d'accès, les appartenances ; deux réglages d'admission |
| 3 | `specs/010-guide-nego-accueil-profil/migration.sql` | Le vocabulaire des thématiques et ses dix termes ; `negotiation.theme_subscriptions` ; `identity.sessions.replaced_by`, qui distingue une réponse de rotation perdue d'un vol (ADR-020) |
| 4 | `specs/011-guide-nego-documents/migration.sql` | Les documents : le réglage `media.private_bucket` et le registre des colonnes qui désignent un objet ; deux types de document et le libellé « Guide » ; les documents, leur extraction, leurs pages, les notes de correction ; le rôle `expert` et ses deux permissions |
| 5 | `specs/012-guide-nego-lecteur-pdf/migration.sql` | Le lecteur montre le PDF d'origine : « ouvrir tel quel » devient le choix « Texte agrandi » (`large_text_choice`), la règle des deux modes écrite une fois (`negotiation.document_reading_modes`), les images de pages réservées à l'aperçu du back-office. **Aucune ligne semée** |
| 6 | `specs/013-guide-nego-faq-lexique/migration.sql` | La FAQ, le parcours « Ma première COP » et le lexique (étape 2) : vocabulaires des rubriques et des familles ; entrées de FAQ, sources, lectures, retours, signalements (`faq_report_status`), questions aux experts ; groupes, étapes et coches du parcours ; entrées du lexique, favoris, termes proposés ; `negotiation.glossary_resolve()` ; les permissions `negotiation.knowledge.publish` et `.review`. **Aucun contenu semé** : les données d'essai (`donnees-essai.sql`) ne partent jamais en production |
| 7 | `specs/014-guide-nego-sessions-agenda/migration.sql` | Les sessions de négociation (3a) : vocabulaires des types de réunion et des groupes ; points de l'ordre du jour ; colonnes de la source sur `negotiation.meetings` ; l'import, son journal, les écarts, les traductions de titres ; « Mon groupe » et « Mon agenda ». Sème le réglage `ai.drafting_model` et **l'import de la COP31, éteint**. Indépendante de celle de l'étape 2 : si l'étape 2 part dans la même mise en ligne, sa migration passe avant, dans l'ordre des numéros |
| 8 | `specs/015-guide-nego-signalements/migration.sql` | Les signalements du réseau (3b) : la colonne `notify_changes` sur `negotiation.theme_subscriptions` ; les signalements, les réunions non annoncées et leur agenda ; les deux fonctions qui disent qui prévenir. Sème la permission `negotiation.report.validate` (donnée au rôle `admin`) et **quatre types de notification**. Passe **après** celle de 3a, dont elle étend les tables |
| 9 | `specs/016-guide-nego-reunions/migration.sql` | Les réunions de la Francophonie (4) : le vocabulaire `francophone_meeting_type` et ses trois natures ; sur `negotiation.meetings`, la nature, le public d'un accès limité, le lien facultatif vers une activité du Pavillon (`ON DELETE SET NULL`), l'inscription requise et la liste d'attente ; sur `negotiation.meeting_registrations`, le rang d'attente et la référence du téléphone qui rend une inscription hors connexion unique ; la validation, la jauge et la promotion depuis l'attente. Sème **deux types de notification**. Aucune session importée n'est touchée. Passe **après** celle de 3b |
| 10 | `specs/017-guide-nego-pavillon/migration.sql` | Le Pavillon de la Francophonie (5) : **aucune table**, un seul objet — la vue `programme.v_public_schedule` gagne neuf colonnes **en queue** (liste d'attente, inscription requise et sa fenêtre, nombre en attente, date du dernier changement, langues, rediffusion et sa durée). Celles que le site lit ne bougent pas. Aucune ligne semée. Passe **après** celle de l'étape 4 |

Les dix sont **rejouables** : un second passage ne crée rien, ne perd rien, n'échoue pas.

**Aucun réglage à ajouter à `.env.prod`.** Les deux réglages nouveaux ont un défaut, et ce défaut
est la valeur voulue :

| Réglage | Défaut | Ce qu'il fait |
|---|---|---|
| `AUTH_SESSION_TTL_APP` | 90 jours, glissants | Durée d'une session ouverte depuis Guide Négo, sans case à cocher |
| `AUTH_REFRESH_GRACE` | 60 secondes | Fenêtre où l'ancien jeton de rafraîchissement, représenté après une réponse perdue, n'est pas pris pour un vol |

Ne les écrire que pour s'écarter du défaut. `PRIVACY_POLICY_VERSION` disparaît : encore posée dans
`.env.prod`, elle est sans effet — la version vient désormais du texte servi par l'API.

**L'étape 1 ajoute deux choses hors de la base**, déjà dans le dépôt :

- **Le bucket privé** `epavillon-prive`, qui garde les PDF et les pages des documents réservés.
  `ops/init-garage-prod.sh` le crée, ouvert à la clé de l'API et **fermé au web** ; il est
  rejouable, on le relance tel quel. Sans lui, le premier dépôt d'un PDF échoue.
- **PDFium dans l'image du worker** (ADR-021) : le `Dockerfile` l'installe et pose
  `PDFIUM_LIB_PATH=/opt/pdfium/lib`. **Ne pas écrire `PDFIUM_LIB_PATH` dans `.env.prod`** : le
  fichier d'environnement l'emporte sur l'image, et le chemin du poste de développement y ferait
  échouer toute extraction.

**L'étape 1b ajoute deux choses hors de la base**, déjà dans le dépôt :

- **pdf.js dans l'image du site** (ADR-022). `pdfjs-dist` est une dépendance du front, installée
  par `npm ci` ; la construction en copie les fichiers lus par adresse — travailleur, polices,
  `wasm`, profils de couleur — sous `/v2/guide-nego/pdfjs/6.3.289/`, que la coquille garde pour le
  mode avion. Rien à installer à part. Contrôle après le redémarrage :
  `curl -sI https://<domaine>/v2/guide-nego/pdfjs/6.3.289/pdf.worker.min.mjs` rend `200`.
- **La relance d'extraction des documents publiés avant la migration 5**, pour recalculer
  `reading_bytes`, la place annoncée d'une copie (PDF et texte). En production, **aucun ne l'est** :
  le guide se publie au § 3, après les migrations, et son extraction est déjà celle de 1b. Sur une
  base de recette qui en porte, pour chaque document publié :
  ```bash
  curl -X POST -b <cookie d'administration> -H 'Origin: <APP_PUBLIC_URL>' \
    "<APP_PUBLIC_URL>/v2/api/admin/negotiation/documents/<id>/extraction"
  ```
  ou « Relancer l'extraction » sur sa fiche du back-office ; la fiche repasse par « Prête ».

**L'étape 3a ajoute deux choses hors de la base :**

- **`OPENROUTER_API_KEY` dans `.env.prod`**, la clé de production (CLAUDE.md, « Clés d'API ») : le
  worker la lit pour traduire les titres des sessions. Sans elle, l'import tourne et les titres
  s'affichent en anglais seul, sans « Traduction automatique » — rien ne casse.
- **L'import reste éteint.** Le lecteur de la source réelle est écrit mais branché sur rien tant
  que l'accord du secrétariat de la CCNUCC manque ; le lecteur archivé rejoue la COP30. **Ne
  l'allumer en production que le temps de la recette sur téléphones** (§ 4), puis l'éteindre et
  retirer ce qu'il a écrit :
  ```sql
  BEGIN;
  DELETE FROM negotiation.session_reports r USING event.events e
   WHERE e.id = r.event_id AND e.slug = 'cop31';        -- 3b : réunions non annoncées et leur agenda suivent
  DELETE FROM engagement.notifications WHERE type_code LIKE 'negotiation.%';
  DELETE FROM negotiation.meetings m USING event.events e
   WHERE e.id = m.event_id AND e.slug = 'cop31' AND m.source_key IS NOT NULL;  -- agenda et écarts suivent
  DELETE FROM negotiation.agenda_items a USING event.events e WHERE e.id = a.event_id AND e.slug = 'cop31';
  DELETE FROM negotiation.import_runs;
  UPDATE negotiation.official_imports SET is_enabled = false, missed_reads = 0, last_success_at = NULL,
         last_attempt_at = NULL, last_error = NULL, last_change_count = NULL, failing_since = NULL;
  COMMIT;
  ```

**La dette de R4 doit être nulle avant la bascule** — aucun objet privé ne doit rester dans le
bucket ouvert au web. La requête est dans [la recette de l'étape 1](../specs/011-guide-nego-documents/quickstart.md)
(« Préalables ») ; elle doit rendre **0**, sinon déplacer ces objets avant la mise en ligne.

### 1. Répéter sur une copie de la production — la veille

```bash
./deploy.sh backup     # rapatrie sauvegardes/epavillon-AAAAMMJJ-HHMMSS.sql.gz
COPIE=postgres://postgres:dev@localhost:5442/copie_prod
psql postgres://postgres:dev@localhost:5442/postgres -c 'CREATE DATABASE copie_prod'
gunzip -c sauvegardes/epavillon-AAAAMMJJ-HHMMSS.sql.gz | psql "$COPIE"

for passage in 1 2; do          # deux passages : le second ne doit rien changer
  for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents 012-guide-nego-lecteur-pdf 013-guide-nego-faq-lexique 014-guide-nego-sessions-agenda 015-guide-nego-signalements 016-guide-nego-reunions 017-guide-nego-pavillon; do
    psql "$COPIE" -v ON_ERROR_STOP=1 -f "specs/$etape/migration.sql" || exit 1
  done
done
```

Une base **à part** : la base de développement n'est pas touchée, et `make media-base-url` n'a pas
lieu d'être.

Puis **comparer les schémas**, § 13 — la copie migrée contre une base chargée depuis
`docs/database/` (la base modèle du harnais de test, `epavillon_test_template_<empreinte>`, fait
l'affaire), après tri des lignes :

```bash
pg_dump --schema-only --no-owner --no-privileges "$COPIE"  | sort > /tmp/migree.sql
pg_dump --schema-only --no-owner --no-privileges "$MODELE" | sort > /tmp/modele.sql
diff /tmp/modele.sql /tmp/migree.sql
```

Écarts admis : les partitions `engagement.email_messages_AAAAMM`, et le texte des vues réécrit par
la restauration. Tout autre écart arrête la mise en ligne.

**Et les lignes semées**, que la comparaison des schémas ne voit pas — c'est ainsi que le drapeau
de 0a a manqué :

```sql
SELECT (SELECT count(*) FROM platform.feature_flags  WHERE key = 'guide_nego.enabled')                  AS drapeau,      -- 1
       (SELECT count(*) FROM platform.settings       WHERE key IN ('negotiation.admission_mode',
                                                                   'negotiation.invitation_attempts'))  AS reglages,     -- 2
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'negotiation_network')      AS reseaux,      -- 1
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'negotiation_theme')        AS thematiques,  -- 10
       (SELECT count(*) FROM platform.settings       WHERE key = 'media.private_bucket')                AS bucket_prive, -- 1
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'document_type'
                                                        AND code IN ('summary', 'bulletin'))            AS types_doc,    -- 2
       (SELECT count(*) FROM identity.role_permissions WHERE role_code = 'expert')                      AS expert,       -- 2
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'negotiation_meeting_type') AS types_reunion, -- 9
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'negotiation_group')        AS groupes,      -- 13
       (SELECT count(*) FROM platform.settings       WHERE key = 'ai.drafting_model')                   AS modele_ia,    -- 1
       (SELECT count(*) FROM negotiation.official_imports WHERE NOT is_enabled)                          AS import_eteint, -- 1 (0 sans l'édition cop31)
       (SELECT count(*) FROM identity.role_permissions WHERE permission_code = 'negotiation.report.validate') AS valider,   -- 1
       (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'francophone_meeting_type') AS natures_reunion, -- 3
       (SELECT count(*) FROM engagement.notification_types WHERE code LIKE 'negotiation.%')              AS types_avis;   -- 7 (5 avant l'étape 4)
```

### 2. Le jour de la mise en ligne

Dans l'ordre du § 13, chaque étape pour sa raison :

1. **Sauvegarder** : `./deploy.sh backup`.
2. **Envoyer le code** sans rien reconstruire : `./deploy.sh push`.
3. **Déposer les migrations** hors du dossier synchronisé, renommées — elles s'appellent toutes
   `migration.sql` :
   ```bash
   for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents 012-guide-nego-lecteur-pdf 013-guide-nego-faq-lexique 014-guide-nego-sessions-agenda 015-guide-nego-signalements 016-guide-nego-reunions 017-guide-nego-pavillon; do
     scp "specs/$etape/migration.sql" "root@<serveur>:/root/epavillon-migrations/$etape.sql"
   done
   ```
4. **Construire les images avant de migrer** — l'ancienne version sert pendant la compilation.
   Sur le serveur (`./deploy.sh connect`), dans le dossier de la pile :
   ```bash
   COMPOSE="docker compose --env-file .env.prod -f ops/docker-compose.prod.yml"
   $COMPOSE build api worker front
   ```
   Puis **le bucket privé**, avant de migrer : `ops/init-garage-prod.sh` (rejouable).
5. **Migrer, puis redémarrer aussitôt** :
   ```bash
   for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents 012-guide-nego-lecteur-pdf 013-guide-nego-faq-lexique 014-guide-nego-sessions-agenda 015-guide-nego-signalements 016-guide-nego-reunions 017-guide-nego-pavillon; do
     $COMPOSE exec -T postgres psql -U postgres -d epavillon -v ON_ERROR_STOP=1 \
       < /root/epavillon-migrations/$etape.sql || break
   done
   $COMPOSE up -d api worker front
   ```
   Une migration qui échoue arrête la boucle, et sa transaction est annulée : la base reste dans
   l'état de l'étape précédente. Ne pas redémarrer ; lire l'erreur.
6. **Santé et garde** : `./deploy.sh sante`, puis la garde du § 6, point 4, et la
   **vérification 4 du § 11** : le PDF se lit par morceaux à travers le relais — `206`, sans
   `Content-Encoding`. Elle se déroule une fois le guide publié (§ 3).
7. **Recomparer** le schéma de production au modèle, puis la requête des lignes semées (§ 1
   ci-dessus). `GET /v2/api/platform/feature-flags` doit rendre `guide_nego.enabled` — éteint.

### 3. Juste après le redémarrage : le site

Ce que cette mise en ligne touche sur le site — les sessions, le renouvellement du jeton, la version
consignée d'un consentement. À faire dans le quart d'heure, avec un compte ordinaire puis un
compte d'administration.

- [ ] **La connexion.** `/v2/connexion`, sans « Rester connecté » : l'accueil s'ouvre, le menu du
      compte paraît. La session porte son origine :
      `SELECT client_kind FROM identity.sessions WHERE person_id = :p ORDER BY issued_at DESC LIMIT 1;`
      → `web`.
- [ ] **Une session qui tient au-delà d'un quart d'heure.** Le jeton d'accès vit quinze minutes.
      Laisser l'onglet seize minutes, puis ouvrir une page de l'espace organisation : on reste
      connecté, sans retour à l'écran de connexion. C'est le renouvellement qu'ADR-020 a modifié.
      Aucune coupure prise pour un vol :
      `SELECT revoked_reason, count(*) FROM identity.sessions WHERE revoked_at > now() - interval '1 hour' GROUP BY 1;`
      → pas de `reuse_detected`.
- [ ] **L'inscription, et son consentement sous `2026-01`.** Créer un compte : courriel reçu, lien
      suivi, connexion. **Créer un compte ne consigne aucun consentement** — constaté le 22/09 ;
      l'accord est écrit à l'inscription à une **séance** dont le formulaire pose une question
      sensible. S'inscrire à une telle séance, y répondre, accepter, puis :
      `SELECT purpose, policy_version FROM identity.current_consents WHERE person_id = :p;`
      → `2026-01`, la version servie par `GET /v2/api/legal/privacy`. S'il n'existe aucune séance
      de ce genre en production, ne pas en créer pour l'occasion : ce chemin est éprouvé par
      `programme/tests/consentement.rs`, dans `make check-safe`.
- [ ] **Le dépôt d'une proposition.** « Déposer une proposition », sur l'appel ouvert, jusqu'à
      l'envoi : l'écran confirme l'envoi, et la proposition paraît dans l'espace organisation
      comme dans la liste du back-office.
- [ ] **Le back-office.** En administrateur : la liste des propositions, celle des utilisateurs,
      puis `/v2/admin/negociations` — nouveau en 0b, les codes d'invitation. Y **créer le code**
      qui servira au § 4.
- [ ] **Les médias.** Le dépôt d'une image a changé de chemin à l'étape 1. En administrateur,
      « Modifier l'édition » : déposer une image dans un emplacement, la voir s'afficher, puis
      **Annuler** — rien n'est rattaché. Les images de l'accueil et de la fiche d'édition
      s'affichent toujours.
- [ ] **Le bucket privé est fermé au web.** `curl -I <APP_PUBLIC_URL>/media/epavillon-prive/x`
      rend une erreur, jamais un objet.
- [ ] **Le guide se publie.** En administrateur, `/v2/admin/negociations/documents` : créer le
      guide, déposer le PDF, attendre « Prête » — c'est la preuve que le worker charge PDFium —,
      feuilleter l'aperçu, publier. L'aperçu propose « Texte agrandi », coché par défaut ;
      « Ouvrir tel quel » n'y est plus. Il servira au § 4.
- [ ] **Le PDF passe le relais par morceaux** : la vérification 4 du § 11, sur le guide publié.
- [ ] **Une note de correction** : avec un compte d'expert, poser une note sur un passage de la
      page 59 du guide et une note sans passage sur la page 60 — elles serviront au § 4.
- [ ] **Les réunions de la Francophonie (étape 4).** En administrateur,
      `/v2/admin/negociations/reunions` : la liste de l'édition s'ouvre, vide. Y créer, **titrées
      « Recette — … »**, les réunions qui serviront au § 4 : une en ligne avec un lien de visio,
      sans limite ; une à capacité 1 **avec** liste d'attente ; une à capacité 1 **sans** liste
      d'attente. Les publier. Un compte sans la permission : l'écran dit « Accès refusé », y compris
      sur l'adresse d'une fiche tapée à la main.
- [ ] **La page « Programmations » du site (étape 5).** La vue du programme a gagné des colonnes :
      `/v2/programmations?edition=cop31` s'affiche comme avant — bandeau, filtres, activités, nombre
      d'inscrits —, sans erreur dans la console ; ouvrir le détail d'une activité et s'y inscrire.
- [ ] **Guide Négo reste fermée** : `/v2/guide-nego/` sert « bientôt disponible ».

Un point qui échoue et ne se corrige pas sur place : `./deploy.sh restore <sauvegarde de l'étape 1>`
ramène la base d'avant les migrations, puis redéployer la version précédente du code.

### 4. Les essais sur téléphones réels, à cocher

**La seule liste** des essais qu'aucun ordinateur ne peut remplacer, étape par étape, de 0a à 5,
puis le scénario qui clôt le MVP. Elle se déroule par le commanditaire, sans l'équipe, sur deux
jours de suite : plusieurs essais demandent une nuit. Les identifiants entre parenthèses renvoient
aux tâches des `tasks.md`.

**Ce qu'il faut**

- **Deux téléphones** : un Android **de milieu de gamme**, et un iPhone — **si possible un iPhone 8
  ou X**, le plus ancien que l'étape 1b vise. Noter pour chacun le modèle et la version du système.
- **L'adresse de la version en ligne**, avec sa barre finale : `https://<domaine>/v2/guide-nego/`.
  C'est d'elle qu'on installe l'application.
- **Deux adresses électroniques** qu'on lit sur les téléphones, une par téléphone : les comptes
  s'y créent à l'étape 0b.
- **Deux comptes déjà ouverts** : un compte **expert** (étapes 1b et 2) et un compte
  **administrateur** (étapes 3b et 4 : il valide les signalements, tient les réunions). Pour le
  scénario qui clôt le MVP, une troisième adresse et un troisième compte (voir son titre).
- **Ce que le § 3 a préparé** : le code d'invitation, le guide publié, les deux notes de l'expert
  sur les pages 59 et 60. Et, au back-office : les contenus de l'étape 2 publiés (FAQ, parcours
  « Ma première COP », lexique) ; l'import des sessions allumé — « Négociations → Import », lecteur
  archivé `cop30/lecture-1`, premier jour de l'archive = **aujourd'hui**, puis « Lire maintenant » ;
  les réunions « Recette — … » de l'étape 4 ; les activités et la rediffusion de l'étape 5 (voir
  leurs titres).
- **Le drapeau ouvert** : § 14, la bascule doit rendre `UPDATE 1`.
- **Un débit bridé** : sur Android, Chrome relié à `chrome://inspect` d'un ordinateur, profil
  « 3G lente » ; à défaut, le téléphone réglé sur la 3G seule.

**Comment lire la liste**

- Chaque case se fait **sur l'Android, puis sur l'iPhone** — sauf « Android seulement » ou
  « iPhone seulement ».
- « **Avant la nuit** » : se fait en fin de première journée, et se laisse tel quel. « **Le
  lendemain** » : se fait le matin suivant, **téléphones restés en mode avion** sauf mention.
- « **En base** » : une vérification à lancer sur le serveur (`./deploy.sh connect`, puis `psql`
  comme au § 2). `:p` y désigne le compte essayé :
  `SELECT id FROM identity.people WHERE primary_email = '<adresse>';`.

**L'ordre conseillé**

1. **Premier jour, l'Android** : les étapes dans l'ordre, de 0a à 3b — chacune s'appuie sur la
   précédente (l'application installée, puis le compte, le code, les thématiques, le guide gardé).
2. **Premier jour, l'iPhone** : le même parcours.
3. **Premier jour, les deux ensemble** : les étapes 4 et 5, dont plusieurs cases se jouent à deux
   téléphones.
4. **Le soir** : les cases « Avant la nuit ».
5. **Le lendemain matin** : les cases « Le lendemain » ; puis les deux dernières cases des
   documents réservés (étape 1) ; puis le scénario qui clôt le MVP, en dernier.
6. **Après** : éteindre l'import et retirer ce qu'il a écrit — signalements et notifications de
   recette compris — (« L'étape 3a ajoute… », en tête de ce § 15) ; retirer les réunions de
   l'étape 4 et la rediffusion de l'étape 5 (sous leurs titres) ; dépublier le document réservé.

#### 0a — La coquille (T071)

- [ ] **Installer (Android seulement).** Ouvrir `guide-nego/installer`, installer, lancer depuis
      l'icône : l'application occupe tout l'écran, l'icône et le nom sont justes.
- [ ] **Installer (iPhone seulement).** « Installer » mène aux étapes à suivre à la main ;
      l'installation par le bouton Partager fonctionne.
- [ ] **Prête sans réseau.** « Continuer en visiteur » : le message « Prête hors connexion » paraît,
      sans avoir visité les onglets.
- [ ] **L'adresse sans barre finale.** Ouvrir `https://<domaine>/v2/guide-nego` (sans `/` à la fin)
      avec le réseau, puis passer en mode avion et recharger : l'application s'ouvre. Cette adresse
      est hors de portée de la garde hors connexion ; le serveur y ramène par une redirection que
      le navigateur retient. (T077 de l'étape 2)
- [ ] **Relancer en mode avion.** Relancer depuis l'icône : tout s'ouvre, le bandeau jaune paraît
      une fois, puis « lu à … » reste dans l'en-tête. Les lettres « œ », « Œ », « É » s'affichent bien.
- [ ] **Le réseau revient.** Application ouverte sans réseau, rendre le réseau : « Synchronisé à … »
      paraît, sans recharger.
- [ ] **Thème sombre.** Choisir « Sombre », fermer, rouvrir en mode avion : l'écran est sombre
      d'emblée, sans flash blanc.
- [ ] **Sur réseau lent (Android seulement).** Débit bridé, application déjà installée : elle
      s'ouvre en moins de deux secondes.
- [ ] **Une mise à jour coupée (Android seulement).** Pendant qu'une nouvelle version de
      l'application s'installe, couper le réseau : l'ancienne version sert toujours.
- [ ] **La langue.** Téléphone réglé en anglais : Guide Négo s'affiche en français.

#### 0b — Le compte et l'admission (T112)

- [ ] **Créer un compte (Android seulement)** depuis l'application installée : le lien du courriel
      s'ouvre dans l'application et enchaîne sur la saisie du code.
- [ ] **Créer un compte (iPhone seulement)** depuis l'application installée : le lien du courriel
      s'ouvre **dans Safari** et dit « Adresse confirmée — retournez dans Guide Négo ». Revenir à
      l'application par le sélecteur d'applications : elle relit l'état du compte **toute seule** et
      passe au code. C'est le moment qui casse le plus facilement.
- [ ] **Un code sans réseau.** En mode avion, saisir un code : l'écran dit qu'il faut le réseau, et
      n'annonce aucun accès.
- [ ] **Le code.** En débit bridé sur l'Android, avec le réseau sur l'iPhone : un code faux donne
      un message d'erreur ; le code du § 3 donne « Code reconnu », et l'accès s'ouvre.
- [ ] **Le lendemain — la session a tenu.** L'application s'ouvre sans redemander de connexion.
      En base :
      `SELECT client_kind, expires_at FROM identity.sessions WHERE person_id = :p AND revoked_at IS NULL;`
      → `app`, et une échéance à quatre-vingt-dix jours.

#### 0c — L'accueil et le profil (T096 à T098)

- [ ] **Choisir ses thématiques.** En débit bridé sur l'Android, avec le réseau sur l'iPhone :
      choisir ses thématiques ; « Ma journée » s'ouvre.
- [ ] **Les changer sans réseau.** En mode avion, changer ses thématiques : le nouveau choix
      s'affiche aussitôt.
- [ ] **Avant la nuit (Android seulement).** Toujours en mode avion, changer encore ses thématiques,
      **noter lesquelles**, fermer l'application.
- [ ] **Le lendemain (Android seulement).** **Rendre le réseau sans ouvrir l'application**, puis
      l'ouvrir : le choix de la veille part tout seul. En base : les thématiques notées la veille,
      en une seule écriture.

#### 1 — Les documents (T116)

- [ ] **Télécharger le guide.** En débit bridé sur l'Android, avec le réseau sur l'iPhone : depuis
      la fiche du guide, « Télécharger ». La progression avance ; la copie paraît dans « Mes
      documents », avec la place qu'elle prend.
- [ ] **Avant la nuit.** En mode avion, lire le guide jusqu'à une page, **la noter**, fermer
      l'application.
- [ ] **Le lendemain — le guide se lit en salle.** Toujours en mode avion, ouvrir le guide :
      « Reprise à la page … » (celle notée), le sommaire s'ouvre, une recherche sans accent trouve.
      C'est la preuve que l'étape 1 est finie : le guide se lit sans réseau, après une nuit.

**Les documents réservés** (T102, quickstart § 5 de `specs/011-…`, étapes 1, 4 et 5). Le compte
admis par le code en 0b. Les deux dernières cases déconnectent puis retirent l'accès : les faire
**en fin de séance**, après l'étape 5, ou redonner l'accès par un nouveau code.

- [ ] **Préparer.** Au back-office, publier un document **réservé** (réservé aux négociatrices et
      négociateurs). Le dépublier à la fin de la séance.
- [ ] **Le télécharger.** Compte admis connecté : le document réservé paraît dans la bibliothèque ;
      « Télécharger » — la copie paraît dans « Mes documents », à côté du guide.
- [ ] **Le lire sans réseau.** Mode avion : le document réservé s'ouvre et se lit.
- [ ] **Se déconnecter.** Avec le réseau, se déconnecter : le message nomme les documents réservés
      qui vont partir. Après : le document réservé n'est plus lisible, le guide l'est toujours.
- [ ] **Retirer l'accès.** Se reconnecter, retélécharger le document réservé. Au back-office de 0b,
      retirer l'accès de ce compte. Rouvrir l'application avec le réseau : le document réservé
      s'efface du téléphone.

#### 1b — Le lecteur du PDF (T084)

Noter l'appareil et la version du système pour chaque case.

- [ ] **Fluide sur les 90 pages.** Guide téléchargé, mode avion : défiler de la page 1 à la page 90
      puis revenir, d'un trait. Aucune saccade visible, aucune page blanche qui dure ; le bas de
      l'écran suit (« Page 61 sur 90 · … »). Agrandir à deux doigts le tableau des sigles jusqu'au
      maximum : il est net une fois le geste fini ; toucher deux fois : retour à la largeur.
- [ ] **La première page, sur réseau lent.** Débit bridé, guide **non** téléchargé : l'ouvrir.
      La jauge avance ; au bout de trois secondes, deux choix paraissent :
  - [ ] « Lire le texte en attendant » : le texte s'ouvre à la même page, moins de cinq secondes
        après l'ouverture ; la page du PDF prend sa place quand elle arrive. Refaire, et choisir
        « Rester sur le texte » : on y reste.
  - [ ] « Télécharger pour lire sans réseau » : le téléchargement part, la copie paraît dans
        « Mes documents ».
- [ ] **Passer en « Texte agrandi ».** À la page 59, toucher la page, puis « Texte » dans la barre :
      la même page, en texte. « Réglages » : le mode, le thème, les trois tailles. Revenir à
      « Pages » : on est toujours à la page 59. Fermer, ouvrir un autre document : il s'ouvre dans
      le dernier mode choisi.
- [ ] **La note en marge** — les deux notes posées au § 3. Relire la bibliothèque avec le réseau,
      puis mode avion. Page 59 : un trait rouge et un petit triangle au bord de la page, à la
      hauteur du paragraphe ; page 60 : en haut de la page. Toucher le triangle : le texte et la
      signature en bas de l'écran, le passage reste visible au-dessus ; la ligne ou la croix
      referme. En
      « Texte agrandi », la note borde le paragraphe.
- [ ] **Le lendemain — chercher sans réseau.** Mode avion, **après une nuit** : « Rechercher »
      « progrès collectifs » — les passages et leurs pages ; en toucher un : sa page s'ouvre, le
      passage surligné. Chercher sans accent (« negociation ») : mêmes résultats.
- [ ] **iPhone seulement — la version d'iOS.** La noter. Dès iOS 16.4 (l'iPhone 8 et le X sont en
      16.7), le document s'ouvre **sur ses pages**. En dessous, il s'ouvre en « Texte agrandi » et
      le dit. C'est le seuil qu'ADR-022 a déduit sans appareil : cette case le vérifie.

#### 2 — La FAQ, le parcours et le lexique

Le compte admis de 0b, le compte expert, le guide téléchargé sur chaque téléphone.

- [ ] **Le lexique sans réseau.** Ouvrir l'application une fois avec le réseau ; mode avion,
      relancer depuis l'icône. « Aa » depuis « Ma journée », depuis Ressources et depuis le lecteur :
      le lexique s'ouvre, « N entrées, sans réseau ». Taper `contact grup` : *contact group* vient en
      tête, **sans attente perceptible** après la dernière lettre. `GGA`, `groupe de contact`,
      `braketed` trouvent aussi leur entrée. La croix ramène à l'écran de départ.
- [ ] **Le clavier sort tout seul.** Toucher « Aa » : le champ est prêt **et le clavier monte**
      sans second toucher. Si le clavier ne sort pas sur l'iPhone, le noter avec la version d'iOS.
- [ ] **L'alphabet au doigt.** Dans la liste du lexique : toucher une lettre saute à sa section ;
      **glisser le doigt** le long des lettres fait défiler lettre à lettre, sans saccade ; une
      lettre grisée ne réagit pas. Refaire avec la taille de texte du téléphone au plus grand.
- [ ] **Partager.** Sur une entrée, « Partager » ouvre le partage du téléphone, avec le titre et le
      lien. Envoyé à soi-même, le lien ouvre l'entrée dans l'application (Android) ou dans Safari
      (iPhone).
- [ ] **La FAQ sans réseau.** Mode avion : chaque rubrique, une entrée complète avec « Vérifié le … » ;
      sa source ouvre le guide téléchargé à la page citée. « Dépassé ou faux » avec deux motifs :
      « partira au retour du réseau ». Réseau rendu : **un** signalement dans la file des experts.
- [ ] **Les coches du parcours, d'un téléphone à l'autre.** Le même compte sur les deux. Cocher
      trois étapes sur l'Android, dont deux en mode avion ; réseau rendu : l'iPhone les montre à
      l'ouverture. Décocher la même étape sur chacun, l'un après l'autre : le second à retrouver le
      réseau dit « mises à jour depuis un autre appareil » et garde le dernier geste.
- [ ] **Les courriels.** Compte admis : poser une question à un expert ; l'expert répond au
      back-office — le courriel arrive **sur le téléphone**, son lien ouvre « Mes questions ».
      Proposer un terme depuis « aucun résultat » ; l'expert le publie — le courriel arrive, son
      lien ouvre l'entrée.
- [ ] **Du lecteur au lexique.** Guide téléchargé, « Texte agrandi », mode avion : toucher
      *global goal on adaptation* — un volet monte ; « Ouvrir dans le lexique » mène à l'entrée.

#### 3a — Les sessions de négociation

L'import allumé (voir « Ce qu'il faut »).

- [ ] **La liste sans réseau.** Avec le réseau, ouvrir « Négociations » ; puis mode avion, relancer
      depuis l'icône : chaque jour de la bande, une fiche **jamais ouverte** et « Mon agenda »
      s'affichent, avec « Hors connexion — lu à … ».
- [ ] **L'agenda sans réseau.** En mode avion, « Ajouter à mon agenda » sur une fiche : la session
      paraît aussitôt dans « Mon agenda ». Réseau rendu, application ouverte — en base, **une**
      ligne : `SELECT count(*) FROM negotiation.agenda_entries WHERE person_id = :p AND meeting_id = :m;` → 1
      (`:m` : la session, lue au back-office).
- [ ] **L'interrupteur au doigt.** Sur une session de l'agenda, « Me rappeler 15 minutes avant »
      bascule au premier toucher et revient au second. (Sur ordinateur, seul l'outil automatique le
      manquait : il cliquait hors de l'écran — constaté le 25/09.)
- [ ] **Le rappel, application ouverte.** Au back-office, régler le premier jour de l'archive pour
      qu'une session commence dans vingt minutes ; l'ajouter à l'agenda, armer le rappel, garder
      l'application à l'écran : à quinze minutes, le bandeau jaune paraît en haut, **une fois** ;
      aucun son, aucune notification (écart 46). Fermer puis rouvrir : il ne revient pas.
- [ ] **Le fuseau.** Téléphone réglé sur un autre fuseau horaire : les heures restent celles
      d'Antalya, avec « heure d'Antalya ».
- [ ] **Le lien du lexique.** Sur une fiche de session, le lien vers le lexique ouvre le lexique,
      et non l'écran d'attente d'avant l'étape 2. (Journal du 25/09 : à vérifier une fois l'étape 2
      en ligne.)

#### 3b — Les signalements et les notifications

L'import toujours allumé. Sur l'un des téléphones, le compte admis, avec une session dans « Mon
agenda » ; sur l'autre, le compte administrateur.

- [ ] **Signaler sans réseau.** Téléphone admis en mode avion, fiche d'une session : « Signaler un
      changement » → « La salle a changé » → une salle → « Envoyer ». La fiche dit « Votre
      signalement — partira au retour du réseau » ; « Mes signalements » (profil) le montre
      « Envoyé — partira au retour du réseau ». Rendre le réseau, application ouverte — en base,
      **une** ligne :
      `SELECT count(*) FROM negotiation.session_reports WHERE author_id = :p AND meeting_id = :m;` → 1.
- [ ] **Valider, puis « Annuler ».** Téléphone administrateur, Ressources → « Validation » →
      « Signalements » : « Valider », puis « Annuler » dans les six secondes. La carte revient « à
      traiter » ; **aucune** notification sur le téléphone admis, **aucun** courriel dans la minute.
      Puis « Valider » sans annuler : « Validé. Affiché dans une minute au plus. » (écart 51) ;
      l'encadré violet paraît sur la fiche, côté téléphone admis.
- [ ] **« Ne pas retenir ».** Sur un autre signalement : les trois motifs, la précision, « Ne pas
      retenir » ; chez l'autrice, « Non retenu à … » et le motif.
- [ ] **La cloche et les notifications.** Téléphone admis : la cloche porte un compteur jaune ;
      la liste s'ouvre, chaque ligne commence par ce qui a changé, puis « Sessions de négociation ·
      heure » (écart 59) ; un toucher ouvre la fiche et marque la ligne lue ; « Tout marquer comme
      lu » remet le compteur à zéro.
- [ ] **Un courriel reçu.** Au back-office, lire `cop30/lecture-2` : le changement d'une session de
      l'agenda arrive **par courriel** sur le téléphone, dans les dix minutes, avec « heure
      d'Antalya ». Puis « À propos » → « Notifications » éteint, nouvelle lecture qui change une
      session : la notification paraît, **aucun** courriel. Rallumer.
- [ ] **Tout se relit sans réseau.** Avec le réseau, ouvrir les notifications, « Mes signalements »,
      une fiche avec un encadré, et une réunion non annoncée validée ; puis mode avion, relancer
      depuis l'icône : tout se relit, avec « Hors connexion — lu à … », l'encadré et la réunion
      (« Non annoncée — signalée par le réseau, validée à … ») compris.

#### 4 — Les réunions de la Francophonie (T033 de `specs/016-…`)

Les réunions « Recette — … » créées au § 3 : une en ligne avec un lien de visio ; une à une place
**avec** liste d'attente ; une à une place **sans** liste d'attente. Le compte administrateur sur
un ordinateur. Les cases « à deux téléphones » se font avec les deux ensemble, chacun sous son
compte admis de 0b : téléphone 1, l'Android ; téléphone 2, l'iPhone.

- [ ] **Le sélecteur au doigt.** Onglet « Francophonie » : « Réunions · Pavillon » bascule au
      premier toucher, le titre change avec lui ; l'étiquette « Se tient aussi au Pavillon » ouvre
      la section Pavillon. Aucun défilement horizontal, même en texte agrandi.
- [ ] **S'inscrire en mode avion.** Avec le réseau, ouvrir la liste puis la fiche de la réunion en
      ligne ; mode avion, « M'inscrire » : « Inscrite » et « Partira au retour du réseau ». Rendre
      le réseau, application ouverte — en base, **une** ligne :
      `SELECT count(*) FROM negotiation.meeting_registrations WHERE person_id = :p AND meeting_id = :m;` → 1.
- [ ] **Le lien visio sans réseau.** Inscrite, avec le réseau : « Rejoindre à distance » paraît.
      Mode avion, relancer depuis l'icône, rouvrir la fiche : le lien est toujours là, avec
      « Hors connexion — lu à … » ; le toucher ouvre la visio une fois le réseau rendu.
- [ ] **La liste d'attente et l'avis (à deux téléphones).** Sur la réunion à une place avec liste
      d'attente : le téléphone 1 prend la place ; le téléphone 2 touche « Rejoindre la liste
      d'attente » → « Liste d'attente — position 1 ». Le téléphone 1 touche « Inscrite » et se
      désinscrit : le téléphone 2 passe « Inscrite », la cloche porte l'avis « Une place s'est
      libérée », et le courriel arrive sur le téléphone. Refaire une seconde fois avec les mêmes
      comptes : le second courriel arrive aussi.
- [ ] **Complet pendant le mode avion (à deux téléphones).** Sur la réunion à une place **sans**
      liste d'attente : le téléphone 2, en mode avion, s'inscrit pendant que le téléphone 1, en
      ligne, prend la place. Au retour du réseau, le téléphone 2 dit « Complet — pas de liste
      d'attente », sans inscription fantôme.
- [ ] **Déplacée puis annulée (à deux téléphones).** Au back-office, changer l'heure d'une réunion
      où les deux téléphones sont inscrits, puis l'annuler avec un motif : l'avis « Déplacée » puis
      « Annulée » paraît ; la fiche dit « Annulée » et le motif.

**Après** : retirer les réunions de recette, avec leurs inscriptions et leurs avis :

```sql
BEGIN;
DELETE FROM engagement.notifications n USING negotiation.meetings m
 WHERE n.subject_id = m.id AND m.source_key IS NULL AND m.title->>'fr' LIKE 'Recette — %';
DELETE FROM negotiation.meetings
 WHERE source_key IS NULL AND kind IN ('preparatory_workshop', 'francophone_consultation')
   AND title->>'fr' LIKE 'Recette — %';                 -- inscriptions et journal suivent
COMMIT;
```

#### 5 — Le Pavillon de la Francophonie (T019 de `specs/017-…`)

Des activités de la COP31 publiées et à venir : une qui ne demande que le pays, une à
formulaire, une à une place avec liste d'attente. Les deux téléphones, chacun sous son compte
admis. **Rien n'écrit la rediffusion** : le back-office du direct appartient à l'ePavillon et
n'existe pas encore. Pour la recette, en poser une en base sur une activité passée, puis la
retirer :

```sql
INSERT INTO live.streams (session_id, event_id, provider, kind, status, watch_url, replay_url, started_at, ended_at)
SELECT s.id, s.event_id, 'youtube', 'replay', 'ended', :'url', :'url', s.starts_at, s.starts_at + interval '52 minutes'
  FROM programme.sessions s WHERE s.slug = :'activite';
-- après la séance
DELETE FROM live.streams WHERE replay_url = :'url';
```

- [ ] **La section et les jours au doigt.** Onglet « Francophonie » → « Pavillon » : le bloc du
      lieu, puis la bande des jours ; chaque jour s'ouvre au premier toucher, le sous-titre dit le
      jour affiché. « Les jours suivants » ouvre le jour qui suit. Aucun défilement horizontal, même
      en texte agrandi. Un compte qui ne suit qu'une thématique voit **toutes** les activités.
- [ ] **S'inscrire d'un geste.** Sur l'activité qui ne demande que le pays : « M'inscrire » →
      « Inscrite », sans formulaire. Toucher « Inscrite », se désinscrire, puis « Annuler » dans les
      six secondes : on reste inscrite.
- [ ] **S'inscrire par formulaire.** Sur l'activité à formulaire : le formulaire monte, le pays est
      déjà rempli, le clavier ne cache pas le champ touché ; une question sensible demande l'accord.
- [ ] **S'inscrire en mode avion.** Avec le réseau, ouvrir la section et une fiche ; mode avion,
      « M'inscrire » : « Inscrite » et « Partira au retour du réseau ». Rendre le réseau,
      application ouverte — en base, **une** ligne :
      `SELECT count(*) FROM programme.registrations WHERE person_id = :p AND session_id = :s AND status <> 'cancelled';` → 1.
- [ ] **Lire sans réseau.** Mode avion, relancer depuis l'icône : la section et une fiche **jamais
      ouverte** se lisent, avec « Hors connexion — lu à … ».
- [ ] **Complet, liste d'attente (à deux téléphones).** Sur l'activité à une place : le
      téléphone 1 prend la place ; le téléphone 2 lit « Rejoindre la liste d'attente », le touche →
      « Liste d'attente — position 1 ».
- [ ] **La rediffusion s'ouvre.** Sur l'activité passée de la recette : « Rediffusion · 52 min »
      dans la liste, « Revoir · 52 min » sur la fiche ; le toucher ouvre la vidéo (application
      YouTube ou navigateur), et le retour ramène dans Guide Négo.
- [ ] **« Ma journée ».** Le jour d'une activité : la ligne Pavillon montre l'heure, le titre,
      « stand, salle » et « Inscrite » ; un jour sans activité : « Rien aujourd'hui. Prochaine : … ».

#### Le scénario qui clôt le MVP (T021 de `specs/017-…`)

En dernier, d'une traite, sur chaque téléphone. Joué au navigateur à 360 px le 27/09 ; jamais
encore sur un appareil. Il faut : un compte **neuf** et une adresse lue sur le téléphone ; un code
d'invitation créé pour la séance, **envoyé au téléphone par WhatsApp** ; l'administrateur sur son
propre téléphone ; un troisième compte qui a dans « Mon agenda » la session qu'on va signaler.
L'import toujours allumé.

- [ ] **Entrer.** Ouvrir le code reçu sur WhatsApp ; installer Guide Négo, créer le compte,
      confirmer l'adresse depuis le courriel **ouvert sur le téléphone**, saisir le code, choisir
      ses thématiques : « Ma journée » s'ouvre.
- [ ] **Lire en salle.** Télécharger le guide ; mode avion : le lire, puis trouver *contact group*
      dans le lexique et par « Rechercher ».
- [ ] **Les sessions du jour.** Réseau rendu : les sessions de négociation du jour, pour ses
      thématiques, avec « lu à … ».
- [ ] **Signaler une annulation.** Signaler l'annulation d'une session ; **l'administrateur la
      valide depuis son téléphone** ; l'encadré paraît sur la fiche, et le troisième compte voit
      l'avis dans la cloche, puis reçoit le courriel.

#### Quand c'est fini

Un écart se note dans le fichier de son étape, `docs/AppNego/progression/etapes/`, avec l'appareil et la version du système. Tout
coché, on coche aussi T071 (`specs/008-…`), T112 (`specs/009-…`), T096 à T098 (`specs/010-…`),
T116 et T102 (`specs/011-…`) et T084 (`specs/012-…`) dans leurs `tasks.md`.

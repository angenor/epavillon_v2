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

Rien à redémarrer entre 3 et 4 : le drapeau se lit à chaque ouverture.


---

## 15. Mettre en ligne 0a, 0b, 0c et l'étape 1 (22/09, complété le 24/09)

Une seule mise en ligne porte les quatre premières étapes de Guide Négo : le code de la branche, et
**quatre migrations**. Préparée ici, **pas encore exécutée**. Le drapeau reste éteint pendant toute
la mise en ligne : le site ne voit que ce qui le touche (§ 3 ci-dessous), l'application ne s'ouvre
qu'à la recette sur téléphones (§ 4).

### Ce qui change

| Ordre | Migration | Ce qu'elle porte |
|---|---|---|
| 1 | `specs/008-guide-nego-coquille/migration.sql` | Une ligne : le drapeau `guide_nego.enabled`, éteint. Sans elle, la bascule du § 14 rend `UPDATE 0` |
| 2 | `specs/009-guide-nego-compte-admission/migration.sql` | `identity.sessions` dit d'où vient la session (site ou application) ; le vocabulaire des réseaux ; les codes d'invitation, leurs usages, les demandes d'accès, les appartenances ; deux réglages d'admission |
| 3 | `specs/010-guide-nego-accueil-profil/migration.sql` | Le vocabulaire des thématiques et ses dix termes ; `negotiation.theme_subscriptions` ; `identity.sessions.replaced_by`, qui distingue une réponse de rotation perdue d'un vol (ADR-020) |
| 4 | `specs/011-guide-nego-documents/migration.sql` | Les documents : le réglage `media.private_bucket` et le registre des colonnes qui désignent un objet ; deux types de document et le libellé « Guide » ; les documents, leur extraction, leurs pages, les notes de correction ; le rôle `expert` et ses deux permissions |

Les quatre sont **rejouables** : un second passage ne crée rien, ne perd rien, n'échoue pas.

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
  for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents; do
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
       (SELECT count(*) FROM identity.role_permissions WHERE role_code = 'expert')                      AS expert;       -- 2
```

### 2. Le jour de la mise en ligne

Dans l'ordre du § 13, chaque étape pour sa raison :

1. **Sauvegarder** : `./deploy.sh backup`.
2. **Envoyer le code** sans rien reconstruire : `./deploy.sh push`.
3. **Déposer les migrations** hors du dossier synchronisé, renommées — elles s'appellent toutes
   `migration.sql` :
   ```bash
   for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents; do
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
   for etape in 008-guide-nego-coquille 009-guide-nego-compte-admission 010-guide-nego-accueil-profil 011-guide-nego-documents; do
     $COMPOSE exec -T postgres psql -U postgres -d epavillon -v ON_ERROR_STOP=1 \
       < /root/epavillon-migrations/$etape.sql || break
   done
   $COMPOSE up -d api worker front
   ```
   Une migration qui échoue arrête la boucle, et sa transaction est annulée : la base reste dans
   l'état de l'étape précédente. Ne pas redémarrer ; lire l'erreur.
6. **Santé et garde** : `./deploy.sh sante`, puis la garde du § 6, point 4.
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
      feuilleter l'aperçu, publier. Il servira au § 4.
- [ ] **Guide Négo reste fermée** : `/v2/guide-nego/` sert « bientôt disponible ».

Un point qui échoue et ne se corrige pas sur place : `./deploy.sh restore <sauvegarde de l'étape 1>`
ramène la base d'avant les migrations, puis redéployer la version précédente du code.

### 4. La recette sur téléphones réels — une seule séance

Ce qu'aucun poste de travail ne peut éprouver, pour les quatre étapes à la fois : l'appareil réel de
0a (T071), T112 de 0b, T096 à T098 de 0c, T116 de l'étape 1. **Deux jours de suite** — deux points exigent une nuit ;
on les prépare en fin de première journée.

**Avant** : le drapeau ouvert (§ 14, `UPDATE 1`) ; le code créé au back-office (§ 3) ; un Android,
un iPhone, deux adresses électroniques qu'on relève sur le téléphone ; un débit bridé — sur Android,
Chrome relié à `chrome://inspect` d'un poste, profil « 3G lente » ; à défaut, le téléphone réglé sur
la 3G seule. L'application s'installe depuis `https://<domaine>/v2/guide-nego/` — **avec la barre
finale** : sans elle, l'adresse est hors de la portée du service worker et tombe sur l'erreur du
navigateur hors connexion.

**Premier jour — Android**

- [ ] `guide-nego/installer`, installer, lancer depuis l'icône : plein écran, icône et nom justes.
- [ ] « Continuer en visiteur » : le message « Prête hors connexion » paraît, sans visiter les onglets.
- [ ] Débit bridé, application gardée : elle s'ouvre en moins de deux secondes sur la version gardée.
      Couper le réseau pendant l'installation d'une nouvelle version : l'ancienne sert toujours.
- [ ] Créer un compte **depuis l'application installée** ; le lien du courriel s'ouvre dans
      l'application et enchaîne sur le code.
- [ ] En débit bridé, saisir le code : « Code reconnu », accès ouvert. Un code faux : message distinct.
- [ ] En débit bridé, choisir ses thématiques ; « Ma journée » s'ouvre.
- [ ] Mode avion : changer ses thématiques — le choix s'affiche aussitôt.
- [ ] En débit bridé, **télécharger le guide** depuis sa fiche : la progression avance, la copie
      paraît dans « Mes documents » avec sa place.
- [ ] **Préparer la nuit** : toujours en mode avion, changer encore ses thématiques, noter lesquelles,
      lire le guide jusqu'à une page notée, fermer l'application.

**Premier jour — iPhone**

- [ ] « Installer » mène aux étapes manuelles ; l'installation par Partager fonctionne.
- [ ] Créer un compte depuis l'application installée ; le lien du courriel s'ouvre **dans Safari** et
      dit « Adresse confirmée — retournez dans Guide Négo ». Revenir par le sélecteur
      d'applications : l'application relit l'état du compte **seule** et passe au code.
- [ ] Saisir le code, choisir ses thématiques, arriver sur « Ma journée ».
- [ ] Mode avion, relancer depuis l'icône : tout s'ouvre, bandeau une fois, « lu à … » dans
      l'en-tête ; la police rend « œ », « Œ », « É ».
- [ ] Thème « Sombre », fermer, rouvrir en mode avion : sombre d'emblée, sans éclair.
- [ ] Avec le réseau, télécharger le guide ; en mode avion, le lire jusqu'à une page notée.

**Le lendemain matin**

- [ ] Les deux téléphones, **encore en mode avion** : ouvrir le guide — « Reprise à la page … »,
      sommaire, recherche sans accent. C'est le critère de sortie de l'étape 1 : le guide se lit en
      salle, sans réseau, après une nuit de veille.
- [ ] Android : **rendre le réseau sans ouvrir l'application**, puis l'ouvrir. Le choix de la veille
      part — c'est le départ à l'ouverture qui l'attrape, aucun `online` n'ayant été émis. En base,
      les thématiques notées la veille, en une écriture.
- [ ] Les deux téléphones : `client_kind` est resté `app` après la nuit —
      `SELECT client_kind, expires_at FROM identity.sessions WHERE person_id = :p AND revoked_at IS NULL;`
      → `app`, et une échéance à quatre-vingt-dix jours.
- [ ] Les deux téléphones : ouverts sans réseau puis rendus au réseau, « Synchronisé à … » sans
      recharger.

Un écart se note dans `docs/AppNego/progress.md`, avec l'appareil et le système. Tout coché, T071,
T112, T096, T097 et T098 le sont aussi dans leurs `tasks.md`.

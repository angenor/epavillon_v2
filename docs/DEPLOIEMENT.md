# Déploiement

Comment la v2 arrive en ligne, et pourquoi elle y arrive par ce chemin-là.

Pour l'environnement de développement, voir [ENVIRONNEMENT_LOCAL.md](ENVIRONNEMENT_LOCAL.md).

---

## 1. La contrainte, et ce qu'elle n'était pas

Deux serveurs, et un seul porte le nom de domaine.

| | Où | Ce qu'il sait faire |
|---|---|---|
| **cPanel** — `epavillonclimatique.francophonie.org` | `68.168.118.201` | HTML, CSS, JS, PHP. Apache 2.4. **Pas de Node, pas de Passenger.** Sert la v1 |
| **Serveur applicatif** — `epavillon.mefali.com` | `173.209.36.111` | Docker, SSH, root. Ubuntu 22.04 |

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
navigateur ──HTTPS──> epavillonclimatique.francophonie.org/v2/…   (Apache, cPanel)
                          │  .htaccess : RewriteRule … [P]
                          │
                          └──HTTPS──> epavillon.mefali.com/v2/…   (Caddy, serveur applicatif)
                                         ├─ /v2/api/*  ──> api:8080   (préfixe retiré)
                                         └─ /v2/*      ──> front:3000 (préfixe conservé)
```

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

## 4. Mise en place

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

```bash
git clone <dépôt> epavillon && cd epavillon
cp .env.prod.example .env.prod        # puis renseigner tout ce qui est marqué À REMPLIR
make sqlx-prepare                      # seulement si des requêtes SQL ont changé
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

### 4.4 cPanel — le relais

Déposer [`ops/htaccess-v2.conf`](../ops/htaccess-v2.conf) en `public_html/v2/.htaccess`.

**Ne pas toucher au `.htaccess` de la racine** : il sert la v1.

```bash
curl -i https://epavillonclimatique.francophonie.org/v2/api/health
```

---

## 5. Ce qui reste à vérifier une fois en ligne

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

## 6. Le jour de la bascule à la racine

Quand l'OIF aura répondu, ou quand la v1 pourra être remplacée :

1. `APP_PUBLIC_URL` sans `/v2`, `NUXT_APP_BASE_URL=/`, `NUXT_PUBLIC_API_BASE=/api`
2. **Reconstruire l'image du site** — la base préfixe chaque URL d'asset écrite dans le HTML, et la
   changer sans reconstruire ne les réécrit pas
3. Le `.htaccess` passe à la racine de `public_html`, `RewriteBase /` et la cible sans `/v2/`
4. Dans le Caddyfile, `handle /api/*` et `handle /*`

Aucun code ne change. Le préfixe n'existe qu'en configuration, et c'est le seul point de ce montage
qui méritait d'être payé d'avance.

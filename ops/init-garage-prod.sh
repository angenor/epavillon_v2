#!/usr/bin/env bash
# Initialisation du stockage objet en production.
#
# Un nœud Garage neuf REFUSE toute écriture tant que son layout n'est pas
# assigné : le conteneur démarre, l'API S3 répond, et chaque dépôt échoue avec
# une erreur qui parle de quorum et non de configuration.
#
# Contrairement à `make garage-init`, qui importe la clé du `.env` de
# développement, celui-ci l'ENGENDRE et l'écrit dans `.env.prod` : un secret de
# production ne se recopie pas depuis un fichier versionné.
#
# Idempotent : relançable sans dommage.
set -euo pipefail

cd "$(dirname "$0")/.."
ENV_FILE=.env.prod
COMPOSE="docker compose --env-file $ENV_FILE -f ops/docker-compose.prod.yml"
GARAGE="$COMPOSE exec -T garage /garage"

# ┌─ POURQUOI ON N'ENCHAÎNE PAS DIRECTEMENT DANS UN TUBE ───────────────────┐
# │ `set -o pipefail` + un filtre qui sort tôt (`head`, `awk … exit`) tue le │
# │ script : le tube se ferme, `garage` reçoit un SIGPIPE, le code de retour │
# │ devient non nul, et `set -e` arrête tout — SANS afficher la moindre      │
# │ erreur, puisque personne n'a échoué au sens habituel. On capture donc la │
# │ sortie d'abord, on la filtre ensuite.                                    │
# └──────────────────────────────────────────────────────────────────────────┘
statut=$($GARAGE status 2>/dev/null || true)
noeud=$(printf '%s\n' "$statut" | grep -Eo '^[0-9a-f]{8,}' | head -1 || true)
[ -n "$noeud" ] || { echo 'ÉCHEC : nœud Garage introuvable — le conteneur tourne-t-il ?'; exit 1; }
echo "Nœud : $noeud"

# **On teste la version APPLIQUÉE, pas la présence du nœud.** Un nœud assigné
# mais non validé apparaît dans « STAGED ROLE CHANGES » : y chercher son
# identifiant fait croire que tout est fait, et le script saute l'application
# à chaque relance — le stockage reste alors indéfiniment sans layout, donc
# en refus d'écriture.
layout=$($GARAGE layout show 2>/dev/null || true)
version=$(printf '%s\n' "$layout" | awk -F': ' '/Current cluster layout version/ {print $2}' | head -1 || true)
if [ "${version:-0}" -eq 0 ]; then
    $GARAGE layout assign -z dc1 -c 50G "$noeud" || true
    $GARAGE layout apply --version 1
fi

cle=$(grep -E '^S3_ACCESS_KEY_ID=' "$ENV_FILE" | cut -d= -f2 || true)
secret=$(grep -E '^S3_SECRET_ACCESS_KEY=' "$ENV_FILE" | cut -d= -f2 || true)

if [ -z "$cle" ] || [ "$cle" = "A_REMPLIR_PAR_GARAGE_INIT" ]; then
    # Le format est imposé par Garage : « GK » suivi de 24 hexadécimaux pour
    # l'identifiant, 64 pour le secret. Une clé mal formée est refusée à
    # l'import, pas à l'usage.
    cle="GK$(openssl rand -hex 12)"
    secret=$(openssl rand -hex 32)
    sed -i "s|^S3_ACCESS_KEY_ID=.*|S3_ACCESS_KEY_ID=$cle|" "$ENV_FILE"
    sed -i "s|^S3_SECRET_ACCESS_KEY=.*|S3_SECRET_ACCESS_KEY=$secret|" "$ENV_FILE"
    echo "Clé engendrée et inscrite dans $ENV_FILE"
fi

$GARAGE bucket create epavillon 2>/dev/null || true
$GARAGE key import --yes -n epavillon-prod "$cle" "$secret" >/dev/null 2>&1 || true
$GARAGE bucket allow --read --write epavillon --key "$cle"

# ┌─ CE QUI REND LES MÉDIAS VISIBLES, ET QUI MANQUAIT ──────────────────────┐
# │ L'API S3 exige une signature : le navigateur ne peut donc pas y lire un │
# │ objet. C'est le point d'accès WEB qui sert les lectures anonymes, et il │
# │ n'ouvre un bucket que si celui-ci l'autorise explicitement.             │
# │                                                                         │
# │ Sans ces deux gestes, tout se passe bien jusqu'à l'affichage : le dépôt │
# │ réussit, l'objet est écrit, et seule l'image reste cassée — le symptôme │
# │ paraît alors venir du téléversement, qui n'y est pour rien.             │
# └─────────────────────────────────────────────────────────────────────────┘
$GARAGE bucket website --allow epavillon >/dev/null
echo 'Garage : bucket « epavillon » ouvert à la clé de .env.prod, et lisible par le web.'

# L'adresse publique des objets suit celle du SITE : `<APP_PUBLIC_URL>/media`,
# servi par le relais `media-proxy` que l'Apache du VPS atteint sous ce chemin.
# Rien à demander à l'OIF — ni entrée DNS, ni certificat —, et la valeur suit le
# domaine du jour : le jour du relais institutionnel, seule `APP_PUBLIC_URL`
# change, et un nouveau passage ici reporte le changement.
#
# LE RÉGLAGE VIT EN BASE, pas dans l'environnement : le modèle y met l'adresse
# de production définitive (`docs/database/050_media.sql` § 8), et tout
# rechargement du schéma la remet en silence.
adresse=$(grep -E '^APP_PUBLIC_URL=' "$ENV_FILE" | cut -d= -f2- | tr -d '"' || true)
[ -n "$adresse" ] || { echo 'ÉCHEC : APP_PUBLIC_URL absente de .env.prod.'; exit 1; }
base_media="${adresse%/}/media"
$COMPOSE exec -T postgres psql -U postgres -d epavillon -q \
    -c "UPDATE platform.settings SET value = to_jsonb('$base_media'::text) WHERE key = 'media.public_base_url';"
echo "Médias : URL publique → $base_media"

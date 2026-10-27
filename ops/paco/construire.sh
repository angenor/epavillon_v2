#!/usr/bin/env bash
# Construit PACO (l'application v1, inchangée à l'écran) pour l'Apache de la v2.
# Usage : ops/paco/construire.sh [dossier de travail]
# Ne touche ni au dépôt v1 ni au serveur. Mode d'emploi : LISEZMOI.md.
set -euo pipefail

ICI="$(cd "$(dirname "$0")" && pwd)"
V2="$(cd "$ICI/../.." && pwd)"
V1="${PACO_V1:-$(cd "$V2/.." && pwd)/epavillonvue}"
TRAVAIL="${1:-${TMPDIR:-/tmp}/paco-build}"
TRAVAIL="${TRAVAIL%/}"
URL_SITE="https://epavillonclimatique.francophonie.org"
ENV_V1="$V1/.env.local"

echec() { echo "ÉCHEC : $*" >&2; exit 1; }
etape() { printf '\n== %s\n' "$*"; }

[ -f "$V1/package.json" ] || echec "dépôt v1 introuvable : $V1 (PACO_V1=… pour un autre chemin)"
[ -f "$ENV_V1" ] || echec "$ENV_V1 introuvable"
case "$TRAVAIL" in "$V1"|"$V1"/*|"$V2"|"$V2"/*) echec "le dossier de travail doit être hors des deux dépôts" ;; esac

# Une variable VITE_* exportée dans le shell entrerait dans le build.
for v in $(env | sed -n 's/^\(VITE_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$v"; done

etape "1. Copie de l'arbre de travail v1 (sans .git, dist, .env*)"
mkdir -p "$TRAVAIL"
rsync -a --delete --delete-excluded \
  --exclude='/.git/' --exclude='/dist/' --exclude='/.env*' \
  --exclude='/node_modules/.vite/' --exclude='/node_modules/.cache/' \
  "$V1/" "$TRAVAIL/"
rm -f "$TRAVAIL"/.env*

etape "2. Garde du routeur"
cp "$V1/src/router/index.js" "$TRAVAIL/src/router/index.js"
patch -d "$TRAVAIL" -p1 --forward --batch --quiet < "$ICI/garde-routeur.patch"
grep -q 'PACO_PATHS' "$TRAVAIL/src/router/index.js" || echec "garde non posée"

etape "3. Variables (trois, valeurs jamais affichées)"
lire() {
  awk -v k="$1" 'index($0, k "=") == 1 { v = substr($0, length(k) + 2) } END { print v }' "$ENV_V1" \
    | tr -d '\r' | sed -E "s/^[[:space:]]*[\"']?//; s/[\"']?[[:space:]]*$//"
}
SB_URL="$(lire VITE_SUPABASE_URL)"
SB_KEY="$(lire VITE_SUPABASE_ANON_KEY)"
[ -n "$SB_URL" ] || echec "VITE_SUPABASE_URL vide dans $ENV_V1"
[ -n "$SB_KEY" ] || echec "VITE_SUPABASE_ANON_KEY vide dans $ENV_V1"
case "$SB_URL" in https://*) ;; *) echec "VITE_SUPABASE_URL n'est pas une adresse https" ;; esac
case "$SB_KEY" in
  sb_secret_*) echec "la clé Supabase est une clé secrète, pas la clé publique" ;;
  sb_publishable_*) echo "clé Supabase : publique (sb_publishable)" ;;
  eyJ*)
    ROLE="$(SB_KEY="$SB_KEY" node -e '
      const p = process.env.SB_KEY.split(".")[1] || "";
      try { console.log(JSON.parse(Buffer.from(p, "base64url").toString()).role || "?") } catch { console.log("?") }')"
    [ "$ROLE" = "anon" ] || echec "la clé Supabase porte le rôle « $ROLE », attendu « anon »"
    echo "clé Supabase : JWT de rôle anon" ;;
  *) echec "format de clé Supabase inconnu" ;;
esac
( umask 077
  printf 'VITE_SUPABASE_URL=%s\nVITE_SUPABASE_ANON_KEY=%s\nVITE_APP_URL=%s\n' "$SB_URL" "$SB_KEY" "$URL_SITE" \
    > "$TRAVAIL/.env.production.local" )
ls -a "$TRAVAIL" | grep '^\.env' | sed 's/^/fichier d’environnement : /'

etape "4. Build"
cd "$TRAVAIL"
[ -d node_modules ] || npm ci
npm run build

etape "5. Vérifications"
DIST="$TRAVAIL/dist"
[ -f "$DIST/index.html" ] || echec "dist/index.html absent"

# Valeurs réelles des clés d'IA de la v1 : aucune ne doit figurer dans le build.
SECRETS="$(mktemp "$TRAVAIL/.secrets.XXXXXX")"
trap 'rm -f "$SECRETS"' EXIT
grep -E '^[[:space:]]*VITE_[A-Z0-9_]*KEY[A-Z0-9_]*=' "$ENV_V1" | grep -v '^[[:space:]]*VITE_SUPABASE_ANON_KEY=' \
  | sed -E 's/^[^=]*=//' | tr -d '\r' | sed -E "s/^[\"']//; s/[\"']$//" | awk 'length($0) >= 16' > "$SECRETS" || true
if [ -s "$SECRETS" ] && grep -rlFf "$SECRETS" "$DIST" >/dev/null 2>&1; then
  grep -rlFf "$SECRETS" "$DIST" >&2
  echec "une clé d'IA de la v1 est dans le build"
fi
echo "clés d'IA de la v1 ($(wc -l < "$SECRETS" | tr -d ' ') valeurs) : absentes du build"

MOTIF_CLE='(^|[^A-Za-z0-9])(sk-(or-v1-|or-|proj-|ant-)?[A-Za-z0-9_-]{20,}|sb_secret_[A-Za-z0-9_-]{8,})'
if grep -rlE "$MOTIF_CLE" "$DIST" >/dev/null 2>&1; then
  grep -rlE "$MOTIF_CLE" "$DIST" >&2
  echec "valeur de forme sk-… ou sb_secret_… dans le build"
fi
echo "valeurs sk-, sk-or-, sk-proj-, sk-ant-, sb_secret_ : aucune"

echo "noms (pas des valeurs) service_role, OPENROUTER, OPENAI, ANTHROPIC :"
grep -rhoE '[A-Za-z_]*(service_role|OPENROUTER|OPENAI|ANTHROPIC)[A-Za-z_]*' "$DIST" | sort | uniq -c | sed 's/^/  /' || echo "  aucun"

grep -rqF "$SB_URL" "$DIST/assets" && echo "adresse Supabase : présente" || echec "adresse Supabase absente du build"
grep -rqF "$URL_SITE" "$DIST/assets" && echo "VITE_APP_URL : présente" || echo "VITE_APP_URL : absente (repli sur l'origine)"
grep -rqF 'forgot-password|reset-password)$/' "$DIST/assets" \
  && echo "garde du routeur : dans le build" || echec "garde du routeur absente du build"

# Seul ce que l'Apache sert est déposé : ni .htaccess, ni vidéos, ni events/.
DEPOT="$TRAVAIL/epavillon-paco"
rm -rf "$DEPOT"; mkdir -p "$DEPOT"
for f in index.html assets images fonts logos favicon.ico logo-ifdd.png version.json; do
  [ -e "$DIST/$f" ] && cp -R "$DIST/$f" "$DEPOT/"
done
find "$DEPOT" -name '.DS_Store' -delete

for d in _nuxt api media v2; do
  [ ! -e "$DEPOT/$d" ] || echec "$d/ au premier niveau du dépôt : collision avec la v2"
done
V2_PUBLIC="$V2/frontend/public"
COMMUNS="$(comm -12 <(cd "$DEPOT" && find . -type f | sort) <(cd "$V2_PUBLIC" && find . -type f | sort))"
[ -z "$COMMUNS" ] || { echo "$COMMUNS" >&2; echec "fichiers communs avec frontend/public de la v2"; }
echo "chevauchement avec frontend/public, /_nuxt/, /api/, /media/ : aucun"

etape "Résultat"
echo "build complet : $DIST ($(du -sh "$DIST" | cut -f1))"
echo "à déposer     : $DEPOT ($(du -sh "$DEPOT" | cut -f1)), vers /var/www/epavillon-paco/"
ls -1p "$DEPOT" | sed 's/^/  /'

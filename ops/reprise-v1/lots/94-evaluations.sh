#!/usr/bin/env bash
# Lot 94 (phase 2) : évaluations des activités (94-evaluations.sql).
#
#   94-evaluations.sh <clone>   sur un clone local de la production
#   94-evaluations.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                               le psql de la production (BASCULE.md, « Après »)
#
# Lit la v1 au banc (BANC, défaut epavillon_repetition) : notes, fil de révision,
# consultations des évaluateurs ; des lectures de messages, les seuls identifiants.
# Notes et messages voyagent avec leur texte : c'est la donnée reprise.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

banc() { docker exec -i "$CONTENEUR" psql -U postgres -d "$BANC" -X -q -At -v ON_ERROR_STOP=1 "$@"; }

copie() {
    local table=$1 colonnes=$2 requete=$3
    printf 'CREATE TEMP TABLE %s (%s);\n' "$table" "$colonnes"
    printf 'COPY %s FROM STDIN WITH (FORMAT text);\n' "$table"
    banc -c "COPY ($requete) TO STDOUT"
    printf '%s\n' '\.'
}

flux() {
    copie lot94_note 'id uuid PRIMARY KEY, activity_id uuid, evaluateur uuid, note numeric(4,2), commentaire text, created_at timestamptz, updated_at timestamptz' \
        'SELECT id, activity_id, revisionniste_id, rating, comment, created_at, updated_at FROM v1.activity_ratings'
    # Un auteur sans rôle v1 est un soumissionnaire qui répond : son message est vu du soumissionnaire.
    copie lot94_message 'id uuid PRIMARY KEY, activity_id uuid, auteur uuid, texte text, partage_soumissionnaire boolean, auteur_equipe boolean, created_at timestamptz, updated_at timestamptz' \
        'SELECT c.id, c.activity_id, c.created_by, c.comment_text, COALESCE(c.shared_with_submitter, false),
                EXISTS (SELECT 1 FROM v1.user_roles r WHERE r.user_id = c.created_by), c.created_at, c.updated_at
         FROM v1.revision_comments c'
    copie lot94_vue 'id uuid PRIMARY KEY, activity_id uuid, personne uuid, premiere timestamptz, derniere timestamptz, nombre integer' \
        'SELECT id, activity_id, revisionniste_id, first_viewed_at, last_viewed_at, view_count FROM v1.revisionniste_activity_views'
    copie lot94_lecture 'id uuid PRIMARY KEY' 'SELECT id FROM v1.revision_comment_reads'
    cat "$ICI/94-evaluations.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'

#!/usr/bin/env bash
# Lot 95 (phase 2) : messages de révision, leurs destinataires et leurs lectures
# (95-messages-de-revision.sql).
#
#   95-messages-de-revision.sh <clone>   sur un clone local de la production
#   95-messages-de-revision.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                                        le psql de la production (BASCULE.md, « Après »)
#
# Lit la v1 au banc (BANC, défaut epavillon_repetition). Les messages voyagent
# avec leur texte (la donnée reprise) ; destinataires et lectures, en identifiants.
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
    # Un auteur sans rôle v1 est un soumissionnaire qui répond : son message est vu du soumissionnaire.
    copie lot95_message 'id uuid PRIMARY KEY, activity_id uuid, auteur uuid, texte text, partage_soumissionnaire boolean, auteur_equipe boolean, destinataires uuid[], created_at timestamptz, updated_at timestamptz' \
        'SELECT c.id, c.activity_id, c.created_by, c.comment_text, COALESCE(c.shared_with_submitter, false),
                EXISTS (SELECT 1 FROM v1.user_roles r WHERE r.user_id = c.created_by),
                COALESCE(c.shared_with_revisionists, ARRAY[]::uuid[]), c.created_at, c.updated_at
         FROM v1.revision_comments c'
    copie lot95_lecture 'id uuid PRIMARY KEY, comment_id uuid, lecteur uuid, read_at timestamptz' \
        'SELECT id, comment_id, revisionniste_id, read_at FROM v1.revision_comment_reads'
    cat "$ICI/95-messages-de-revision.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'

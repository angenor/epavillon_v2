#!/usr/bin/env bash
# Pose la v1 brute (Supabase : public, auth, storage) dans epavillon_repetition,
# sous les schémas v1, v1_auth et v1_storage, puis vérifie les comptages.
#
# Chemin : restauration filtrée dans une base de passage, renommage des schémas
# là-bas, puis recopie des trois schémas renommés. Restaurer directement dans le
# banc ferait entrer l'archive dans son `public`, celui des extensions de la v2.
#
# Retenu de l'archive : schémas, types, tables, données, clés primaires et
# uniques, clés étrangères, commentaires. Laissé : fonctions, vues, déclencheurs,
# politiques et RLS, index, séquences, droits et propriétaires Supabase ; les
# valeurs par défaut sont retirées (elles appellent `extensions.uuid_generate_v4()`).
set -euo pipefail
source "$(dirname "$0")/commun.sh"

test -f "$EXTRACTION_V1" || { echo "Extraction v1 introuvable : $EXTRACTION_V1" >&2; exit 1; }
test -x "$PG_RESTORE" || { echo "pg_restore introuvable : $PG_RESTORE" >&2; exit 1; }

# Jetons de connexion et journaux techniques de Supabase : tables créées vides.
# Aucune valeur pour la reprise (plan § 4, D3), et des secrets encore valides.
# Motif sur les lignes de la table des matières de l'archive.
DONNEES_NON_POSEES='TABLE DATA (auth (audit_log_entries|refresh_tokens|sessions|one_time_tokens|mfa_amr_claims|flow_state|schema_migrations)|storage migrations) '

TRAVAIL=$(mktemp -d)
trap 'rm -rf "$TRAVAIL"' EXIT

etape "Base de passage $BASE_PASSAGE"
recreer_base "$BASE_PASSAGE"
psql_dans "$BASE_PASSAGE" <<'SQL'
-- Ce que les définitions de tables Supabase exigent pour se créer.
CREATE SCHEMA extensions;
CREATE EXTENSION "uuid-ossp" WITH SCHEMA extensions;
CREATE EXTENSION vector WITH SCHEMA public;
SQL

etape "Restauration filtrée de $(basename "$EXTRACTION_V1")"
"$PG_RESTORE" -l "$EXTRACTION_V1" \
  | grep -E '^[0-9]+; [0-9]+ [0-9]+ (SCHEMA - (auth|storage) |TYPE (auth|public|storage) |TABLE (auth|public|storage) |TABLE DATA (auth|public|storage) |(FK )?CONSTRAINT (auth|public|storage) |COMMENT (auth|public|storage) (TABLE|COLUMN|TYPE) )' \
  | grep -v -E "$DONNEES_NON_POSEES" \
  > "$TRAVAIL/selection.toc"
echo "$(wc -l < "$TRAVAIL/selection.toc" | tr -d ' ') entrées retenues"
"$PG_RESTORE" -L "$TRAVAIL/selection.toc" --no-owner --no-acl -f - "$EXTRACTION_V1" \
  | psql_dans "$BASE_PASSAGE" >/dev/null

etape "Renommage des schémas"
psql_dans "$BASE_PASSAGE" <<'SQL'
DO $$
DECLARE c record;
BEGIN
    FOR c IN
        SELECT table_schema, table_name, column_name
        FROM information_schema.columns
        WHERE table_schema IN ('public', 'auth', 'storage') AND column_default IS NOT NULL
    LOOP
        EXECUTE format('ALTER TABLE %I.%I ALTER COLUMN %I DROP DEFAULT',
                       c.table_schema, c.table_name, c.column_name);
    END LOOP;
END $$;
-- `vector` suit `public` au renommage : on l'écarte, puis on le rend au nouveau
-- `public`, où le banc a le sien (extension de la v2).
ALTER EXTENSION vector SET SCHEMA extensions;
ALTER SCHEMA public RENAME TO v1;
ALTER SCHEMA auth RENAME TO v1_auth;
ALTER SCHEMA storage RENAME TO v1_storage;
CREATE SCHEMA public;
ALTER EXTENSION vector SET SCHEMA public;
COMMENT ON SCHEMA v1 IS 'Supabase v1, schéma public, brut (ops/reprise-v1). Supprimé après la bascule.';
COMMENT ON SCHEMA v1_auth IS 'Supabase v1, schéma auth, brut (ops/reprise-v1). Supprimé après la bascule.';
COMMENT ON SCHEMA v1_storage IS 'Supabase v1, schéma storage, brut (ops/reprise-v1). Supprimé après la bascule.';
SQL

etape "Recopie dans $BASE_BANC"
psql_dans "$BASE_BANC" -c "SET client_min_messages = warning" -c "DROP SCHEMA IF EXISTS v1, v1_auth, v1_storage CASCADE"
docker exec "$CONTENEUR" pg_dump -U postgres -d "$BASE_PASSAGE" -n v1 -n v1_auth -n v1_storage --no-owner --no-acl \
  | psql_dans "$BASE_BANC" >/dev/null
psql_dans postgres -c "DROP DATABASE $BASE_PASSAGE WITH (FORCE)"

etape "Comptages contre comptes-v1-attendus.txt"
grep -v -E '^\s*(#|$)' "$ICI/comptes-v1-attendus.txt" | awk '{print $1 "\t" $2}' > "$TRAVAIL/attendus.tsv"
psql_dans "$BASE_BANC" -At -F $'\t' <<'SQL' > "$TRAVAIL/obtenus.tsv"
SELECT replace(replace(replace(n.nspname, 'v1_auth', 'auth'), 'v1_storage', 'storage'), 'v1', 'public')
       || '.' || c.relname,
       (xpath('/row/n/text()',
              query_to_xml(format('SELECT count(*) AS n FROM %I.%I', n.nspname, c.relname), false, true, '')))[1]::text
FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname IN ('v1', 'v1_auth', 'v1_storage') AND c.relkind IN ('r', 'p')
ORDER BY 1;
SQL
ecarts=$(join -t $'\t' -a1 -a2 -e ABSENTE -o 0,1.2,2.2 <(sort "$TRAVAIL/attendus.tsv") <(sort "$TRAVAIL/obtenus.tsv") \
         | awk -F'\t' '$2 != $3')
printf '%s tables v1 posées, %s lignes\n' "$(wc -l < "$TRAVAIL/obtenus.tsv" | tr -d ' ')" \
       "$(awk -F'\t' '{s += $2} END {print s}' "$TRAVAIL/obtenus.tsv")"
if [ -n "$ecarts" ]; then
    printf 'ÉCHEC, comptages différents (table, attendu, obtenu) :\n%s\n' "$ecarts" >&2
    exit 1
fi
# PACO (D9) vit encore et fait bouger ses tables d'une extraction à l'autre :
# la part reprise des inscriptions, elle, ne doit pas bouger.
hors_paco=$(psql_dans "$BASE_BANC" -At -c "
    SELECT count(*) FROM v1.activity_registrations r
    JOIN v1.activities a ON a.id = r.activity_id JOIN v1.events e ON e.id = a.event_id
    WHERE e.title NOT ILIKE 'PACO%'")
if [ "$hors_paco" != 105 ]; then
    echo "ÉCHEC : $hors_paco inscriptions hors PACO, 105 attendues (carte du cœur)" >&2
    exit 1
fi
echo "Comptages conformes (dont 105 inscriptions hors PACO)."

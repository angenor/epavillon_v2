# Reprise v1 : contrat commun des lots

Lu par chaque script de `ops/reprise-v1/lots/`. Plan : [`docs/progression/reprise-v1/plan.md`](../../../docs/progression/reprise-v1/plan.md) ; écarts tranchés : [`ecarts.md`](../../../docs/progression/reprise-v1/ecarts.md).

**Échéance (commanditaire, 27/10)** : réouverture le 27/10 au soir ou le 28/10 au matin, **en deux temps**. La phase 1 reprend ce qui sert la COP31 ; la phase 2 le reste, site ouvert.

## Banc

- `epavillon_repetition` : copie de la production v2, données d'essai purgées (D2), v1 brute posée à côté. **Modèle intouchable** : chaque lot travaille sur son clone, `CREATE DATABASE epavillon_lot_<nom> TEMPLATE epavillon_repetition`, puis y joue les lots qui le précèdent et le sien.
- v1 brute, tables et données seules, noms de Supabase : schémas `v1` (son `public`), `v1_auth` (son `auth`), `v1_storage` (son `storage`).
- Conteneur `epavillon-postgres`, utilisateur `postgres`. Aucune autre base n'est à toi. Jamais `make check`, `make check-db`, `down`, `down -v`, `pkill -f`.

## Table de correspondance `legacy.id_map`

- Une ligne par ligne v1 du périmètre : `legacy_table` = nom **Supabase** qualifié (`auth.users`, `public.organizations`, `public.activities`…), `legacy_id` en texte.
- `disposition` : `migrated`, `merged` (plusieurs lignes v1, une ligne v2) ou `skipped` (écartée, `target_id` nul). `note` : la raison, en français, **obligatoire** hors `migrated`.
- `legacy.map(table, id)` rend la cible ; `legacy.remember(...)` l'écrit.
- Personne v1 = `auth.users.id` → `identity.people` (`legacy_table = 'auth.users'`) ; le compte se retrouve par `person_id`.

## Règles d'écriture

- Un fichier `NN-domaine.sql` par lot, **une transaction**, qui commence par `set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true)` (personne « Reprise v1 », lot 00) et `set_config('app.request_id', 'reprise-v1-NN', true)`.
- Rejouable par construction : le banc repart de zéro, le lot ne se joue jamais deux fois sur la même base.
- Lignes insérées **à leur état final, dates fournies** ; jamais d'`UPDATE` d'état qui émettrait une notification ; jamais d'`ON CONFLICT` contre une ligne de production (une fusion est une décision écrite).
- Aucun nom de table ou colonne v2 deviné : lire `docs/database/*.sql`.
- **Compte rendu en fin de lot** : pour chaque table v1 du périmètre, lignes extraites = `migrated` + `merged` + `skipped` dans `id_map`, sinon `RAISE EXCEPTION`.
- Données personnelles : jamais dans le dépôt, un journal, un message de commit ou un compte rendu ; chiffres seulement.

## Lots

| NN | Domaine | Phase |
|---|---|---|
| 00 | socle : personne « Reprise v1 », réglages de lot | 1 |
| 10 | référentiels : pays, termes de taxonomie manquants | 1 |
| 20 | personnes, comptes, rôles | 1 |
| 30 | organisations, noms, domaines, adhésions | 1 |
| 40 | événements, appels | 1 |
| 50 | activités : propositions, séances, intervenants, documents, thématiques | 1 |
| 60 | fichiers : buckets Supabase vers Garage, `media.assets`, `media.attachments` | 1 |
| 70 | témoignages de l'accueil, écrits dans le code v1 (`code.herosection`), et leurs photos | 2, joué le 27/10 |
| 80 | inscriptions (`public.activity_registrations`) | 2, joué le 27/10 |
| 90 | journées spéciales de la CdP30 (`code.journees-speciales`) : activités de la page statique seules, relevé `journees-speciales/` | 2, joué le 27/10 |
| 91 | complément du lot 90 : intervenants et visuel des propositions recopiés sur ses 14 séances | 2, joué le 27/10 |
| 92 | intervenants écartés pour D16 : lignes sans personne (migration `2026-10-27-intervenant-sans-adresse` d'abord) | 2, joué le 27/10 |
| 93 | photos des intervenants du lot 92 (3), sur leur ligne du dossier ; fichiers déposés par 60-fichiers.sh | 2, joué le 27/10 |
| 94 | évaluations : notes et consultations (`public.activity_ratings`, `revisionniste_activity_views`) | 2, joué le 28/10 |
| 95 | messages de révision avec destinataires et lectures (`revision_comments`, `revision_comment_reads`), après la fonction v2 | 2, joué le 28/10 |
| 96 | galeries d'activité (D7) : 11 photos de `public.media_gallery` en rôle `gallery` sur la séance, légende sur le rattachement (migration `2026-10-29-galerie-activite` d'abord) ; fichiers déposés par 60-fichiers.sh | 2 |
| 100 | rediffusions CdP30 attendues : `is_recorded` vrai pour toutes les séances publiées (61, toutes filmées), les 33 vidéos v1 consignées `skipped` (`public.activities-youtube_link`, chaîne piratée, à reposer après la CdP31) | 2 |
| 101+ | historique des modifications, documents de négociation, modules fermés (questions aux activités : non reprises, commanditaire 28/10) | 2 |

Phase 2, site ouvert : le lot se répète sur un clone local de la production (qui dépose aussi les fichiers), puis son flux SQL est lu par le psql du serveur (exemple : `70-temoignages.sh`).

# Contrat — Codes d'erreur

**Fonctionnalité** : Média + Engagement (B6) · **Date** : 2026-08-21

> Un code stable, un message français, un champ quand la faute est sur un champ. Le front branche sur le **code**, jamais sur le texte. Les codes sont déclarés au catalogue du noyau (`kernel::error`), d'où la documentation OpenAPI les engendre : un code ajouté apparaît au prochain démarrage, un code oublié n'existe pas.

---

## Seize codes ajoutés

### Module Média (11)

| Code | Statut | Message | Champ | Origine |
|---|---|---|---|---|
| `MEDIA_QUOTA_EXCEEDED` | 422 | « L'espace de stockage de cette organisation est atteint. » | — | Le contrôle préalable **et** `SQLSTATE 53100` de `tg_enforce_quota` — **le même code pour les deux** (R14, écart n° 136). Le détail porte plafond, consommé, restant |
| `MEDIA_MIME_NOT_ALLOWED` | 422 | « Ce type de fichier n'est pas accepté pour ce rôle. » | `file` | Table blanche, avant écriture ; et le refus du trigger de rattachement |
| `MEDIA_TOO_LARGE` | 413 | « Ce fichier dépasse la taille acceptée pour ce rôle. » | `file` | Table blanche ; et le refus du trigger |
| `MEDIA_ASPECT_RATIO` | 422 | « Les dimensions de cette image ne correspondent pas à la forme attendue. » | `file` | `tg_validate_attachment`. Le détail porte **largeur, hauteur, rapport obtenu, rapport attendu, tolérance** (FR-037) |
| `MEDIA_ROLE_NOT_DECLARED` | 422 | « Ce rôle n'est pas prévu pour ce type de contenu. » | `role` | `tg_validate_attachment`, `integrity_constraint_violation` |
| `MEDIA_ROLE_EXCLUSIVE` | 409 | « Ce rôle n'accepte qu'un seul fichier ; remplacez celui qui s'y trouve. » | `role` | `tg_validate_attachment`, `unique_violation`, et `ux_attachments_exclusive_role` sous concurrence |
| `MEDIA_ASSET_NOT_SERVABLE` | 422 | « Ce fichier n'est pas exploitable : il est supprimé ou en quarantaine. » | `asset_id` | `tg_validate_attachment` |
| `MEDIA_ALT_TEXT_REQUIRED` | 422 | « Décrivez cette image en une phrase : elle ne pourra pas s'afficher sans. » | `alt_text` | Le service (R9, écart n° 129) |
| `MEDIA_ASSET_IN_USE` | 409 | « Ce fichier est encore utilisé ; il ne peut pas être supprimé. » | — | Le service (R11, écart n° 128). Le détail porte **le nombre d'entités** qui l'utilisent |
| `MEDIA_UPLOAD_INCOMPLETE` | 400 | « L'envoi du fichier s'est interrompu. » | `file` | Flux rompu, ou poids reçu différent du poids annoncé (FR-017) |
| `MEDIA_STORAGE_UNAVAILABLE` | 503 | « Le stockage des fichiers est momentanément indisponible. » | — | Le contrat de stockage. **Aucune description n'est écrite** quand il remonte |

### Module Engagement (5)

| Code | Statut | Message | Champ | Origine |
|---|---|---|---|---|
| `ENGAGEMENT_REMINDER_OFFSETS_INVALID` | 422 | « Les délais de rappel doivent être compris entre un et huit valeurs, toutes positives. » | `offsets` | `ck_reminder_rules_offsets`, via `are_offsets_valid()` |
| `ENGAGEMENT_REMINDER_SCOPE_INVALID` | 422 | « Une règle de rappel vise une édition ou une séance, jamais les deux. » | `scope` | `ck_reminder_rules_scope` |
| `ENGAGEMENT_TEMPLATE_VARIABLE_UNKNOWN` | 422 | « Ce modèle utilise une variable que ce type de message ne fournit pas. » | `body_html` | Le service, à la publication (FR-083). Le détail **nomme la variable** |
| `ENGAGEMENT_TEMPLATE_VERSION_UNKNOWN` | 404 | « Cette révision de modèle n'existe pas. » | — | Le service |
| `ENGAGEMENT_NOTIFICATION_TYPE_UNKNOWN` | 422 | « Ce type de notification n'existe pas ou n'est plus actif. » | `type_code` | `is_channel_enabled()` rend **faux** pour un type inconnu ; l'écriture de préférence, elle, refuse explicitement |

---

## Ce que le service traduit, et à partir de quoi

**Toujours le `SQLSTATE` et le nom de la contrainte, jamais le texte du message.** La règle vient de B3 : un message français traduit se périme au premier ajustement du SQL, et deux libellés du même refus finissent par diverger.

| SQLSTATE | Contrainte ou fonction | Code rendu |
|---|---|---|
| `53100` | `tg_enforce_quota` | `MEDIA_QUOTA_EXCEEDED` |
| `23505` | `ux_attachments_exclusive_role`, `ux_attachments` | `MEDIA_ROLE_EXCLUSIVE` |
| `23505` | `ux_reminder_rules_event`, `ux_reminder_rules_session` | traité comme une **modification**, pas comme une erreur (FR-073) |
| `23505` | `ux_scheduled_reminders_once` | **absorbé** : c'est la garantie anti-doublon, pas un refus |
| `23505` | `ux_email_messages_provider` | **absorbé** : annonce de délivrabilité déjà vue (FR-101) |
| `23514` | `ck_reminder_rules_offsets` | `ENGAGEMENT_REMINDER_OFFSETS_INVALID` |
| `23514` | `ck_reminder_rules_scope` | `ENGAGEMENT_REMINDER_SCOPE_INVALID` |
| `23514` | `ck_assets_alt_text_required` | ne doit **jamais** remonter : le service refuse avant (R9). S'il remonte, c'est un défaut du service, et il sort en 500 |
| `23514` | `i18n_text_check` | déjà au catalogue depuis B1 |
| `23503` / `P0001` | `tg_validate_attachment` | selon le libellé de la contrainte visée : rôle, type, poids, forme, quarantaine |

**`tg_validate_attachment` lève ses refus par `RAISE EXCEPTION` avec un `ERRCODE` mais sans nom de contrainte.** Trois de ses cinq refus partagent `integrity_constraint_violation`. Le service les distingue donc **par le contrôle qu'il a lui-même effectué en amont** — type, poids et forme sont vérifiés contre la table blanche avant l'appel — et non par le texte du message. Ce qui remonte malgré tout sous `integrity_constraint_violation` sans avoir été prévu sort en `MEDIA_ROLE_NOT_DECLARED`, le seul cas restant.

---

## Ce qui n'est PAS une erreur

| Situation | Ce qui est rendu | Pourquoi |
|---|---|---|
| Un fichier annoncé qui serait refusé | **200** avec `UploadVerdict` | La pré-vérification est une question, pas une tentative |
| Un contenu déjà connu | **200** avec l'objet existant | C'est le succès de la déduplication |
| Un objet en cours de traitement | **200** avec son état | « en traitement » n'est pas « absent » |
| Un objet en échec ou en quarantaine | **200** avec son état, sur la route d'avancement ; **absent** des lectures publiques | L'écran doit pouvoir le dire ; le public ne doit pas le voir |
| Une séance sans règle de rappel | **200**, liste vide **et** `has_rule: false` | FR-051 : une liste vide muette se confond avec « tout est parti » |
| Un rappel écarté | **200**, l'état et le motif dans le calendrier | C'est une issue, pas une panne |
| Un type sans modèle publié | **200**, le message part avec le texte de secours | FR-086 : un rappel ne se perd jamais en silence |
| Une préférence posée sur un type critique | **200**, enregistrée, avec `is_overridable: false` | FR-095 : l'écran le dit, l'API ne refuse pas |
| Une annonce de délivrabilité déjà reçue | **200**, comptée dans `ignored` | FR-101 |

---

## Les refus d'accès

La règle du principe IX ne souffre aucune exception ici : **un identifiant hors périmètre se refuse comme un identifiant inexistant.** Les six entités porteuses de la table blanche, la séance d'un calendrier de rappels, l'édition d'une règle et l'organisation d'un quota sont toutes concernées.

Un compte sans aucun périmètre d'administration reçoit un **refus explicite**, jamais une liste vide (principe V) — sur le tableau des quotas, les orphelins et la liste des règles.

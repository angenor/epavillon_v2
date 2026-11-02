-- Plusieurs onglets ne déconnectent plus (02/11). Rejouable, sans effet sur les
-- données : seul le commentaire de colonne change.
-- Modèle : docs/database/030_identity.sql (identity.sessions.replaced_by).
--
-- Le jeton tourné représenté dans la minute ouvre une session sœur au lieu de
-- fermer la remplaçante (ADR-024 de Guide Négo). La colonne existe déjà
-- (2026-09-22) ; l'API peut être déployée avant ou après ce script.

COMMENT ON COLUMN identity.sessions.replaced_by IS
    'La dernière session ouverte depuis le jeton de celle-ci. Le jeton d''une session tournée, présenté dans la minute qui suit sa rotation (AUTH_REFRESH_GRACE), vient d''un autre onglet (les onglets partagent les cookies) ou d''une réponse perdue : il ouvre une session sœur sans fermer la première, si la lignée suivie par cette colonne vit encore (une lignée fermée par déconnexion, changement de mot de passe ou suspension ne se rouvre pas). Hors de la minute, tout est coupé (« reuse_detected »). Le motif « response_lost » n''est plus écrit depuis le 02/11/2026 ; son jeton qui revient reste un vol. ADR-024 de Guide Négo, qui remplace la reprise d''ADR-020.';

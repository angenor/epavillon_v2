-- Temps réel (31/10) : `platform.signal(topic, payload)`, signal d'écran par pg_notify.
-- Rejouable (CREATE OR REPLACE), additive, aucune donnée touchée.
-- Modèle : docs/database/010_platform.sql § 3 bis.
--
-- Jouer AVANT de démarrer la nouvelle API : elle écoute le canal `platform_signal`
-- et ses tests appellent la fonction. Sans la migration, l'API démarre mais
-- aucun écran ne reçoit de signal (les modules qui émettent échoueraient).
BEGIN;

CREATE OR REPLACE FUNCTION platform.signal(p_topic text, p_payload jsonb DEFAULT NULL)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_message text;
BEGIN
    IF p_topic IS NULL
       OR length(p_topic) > 200
       OR p_topic !~ '^[a-z][a-z_]*(:[a-z0-9_-]+)+$' THEN
        RAISE EXCEPTION 'sujet de signal invalide : %', coalesce(p_topic, 'NULL')
            USING ERRCODE = 'check_violation';
    END IF;

    IF p_payload IS NOT NULL AND jsonb_typeof(p_payload) <> 'object' THEN
        RAISE EXCEPTION 'charge d''un signal : objet JSON attendu'
            USING ERRCODE = 'check_violation';
    END IF;

    v_message := jsonb_build_object('topic', p_topic, 'payload', p_payload)::text;

    IF octet_length(v_message) > 7500 THEN
        RAISE EXCEPTION 'signal trop volumineux (% octets) : un signal annonce, il ne transporte pas',
            octet_length(v_message)
            USING ERRCODE = 'check_violation';
    END IF;

    PERFORM pg_notify('platform_signal', v_message);
END;
$$;

COMMENT ON FUNCTION platform.signal(text, jsonb) IS
    'Signal d''écran : annonce qu''un sujet a changé (pg_notify sur platform_signal, livré au commit). Ne porte jamais la donnée ; les pages relisent leur route REST.';

COMMIT;

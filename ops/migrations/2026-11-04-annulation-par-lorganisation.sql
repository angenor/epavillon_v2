-- L'organisation annule elle-même son activité retenue (04/11). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 1 (transitions autorisées).
--
-- `accepted -> cancelled` s'ouvre au porteur. Le motif reste facultatif dans la
-- règle, pour l'équipe (28/10) ; celui de l'organisation est exigé par le
-- service de sa route, qui refuse aussi dès qu'une séance a commencé.
--
-- Déployer avec l'API : sans elle, la règle ouverte n'a aucune route, et rien
-- ne change pour personne. Le worker doit suivre : c'est lui qui écrit à
-- l'équipe (engagement.send_owner_cancellation).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-annulation-par-lorganisation', true);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM programme.proposal_transitions_allowed
                    WHERE from_status = 'accepted' AND to_status = 'cancelled') THEN
        RAISE EXCEPTION 'Règle accepted -> cancelled absente : base inattendue, rien n''est modifié.';
    END IF;
END $$;

UPDATE programme.proposal_transitions_allowed
   SET allowed_for_owner = true
 WHERE from_status = 'accepted' AND to_status = 'cancelled'
   AND NOT allowed_for_owner;

DO $$
BEGIN
    IF NOT (SELECT allowed_for_owner FROM programme.proposal_transitions_allowed
             WHERE from_status = 'accepted' AND to_status = 'cancelled') THEN
        RAISE EXCEPTION 'La règle accepted -> cancelled n''est pas ouverte au porteur.';
    END IF;
END $$;

COMMIT;

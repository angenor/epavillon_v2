-- Le formulaire d'inscription par défaut ne demande plus que le pays (nom,
-- prénom et adresse viennent du compte ou de l'invité). Les trois autres
-- questions restent déclarées, désactivées. Modèle : docs/database/075_programme_sessions.sql.
BEGIN;
UPDATE programme.registration_form_fields ff
   SET is_active = false
  FROM programme.registration_forms f
 WHERE f.id = ff.form_id
   AND f.code = 'default'
   AND ff.code IN ('job_title', 'organization', 'referral_source')
   AND ff.is_active;
COMMIT;

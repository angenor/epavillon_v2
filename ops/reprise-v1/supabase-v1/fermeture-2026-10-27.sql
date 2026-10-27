-- Supabase v1 (projet epavillon) : fermeture des tables ouvertes à la clé publique.
-- Accord du commanditaire le 27/10. Retour arrière : rouverture-2026-10-27.sql.
--
-- 39 tables de `public` sans RLS : la clé publique (dans le code de toute page v1)
-- pouvait les lire et les écrire, `user_roles` comprise, donc se donner un rôle.
-- RLS sans politique ferme une table à `anon` et `authenticated` ; `postgres` et
-- `service_role` (extraction de la reprise, clé secrète) passent outre, et les
-- fonctions SECURITY DEFINER aussi (inscriptions PACO, `handle_new_user`).
--
-- Ce que PACO et sa coquille lisent encore, relevé dans le code v1 le 27/10 :
-- - `user_roles` : sa politique existante (rôles actifs lisibles) s'applique
--   désormais ; plus aucune écriture.
-- - `activity_speakers` : la fenêtre de courriel du super administrateur y lit
--   les adresses des intervenants ; une politique de lecture la lui garde.
-- - `user_blocks` : seul un blocage depuis les notifications y écrit, v1 hors ligne.
--
-- Les quatre fonctions `*_role_simple` écrivaient dans `user_roles` sans vérifier
-- l'appelant, exécutables par `anon` : fermer la table ne suffisait pas. Seul
-- l'écran de gestion des utilisateurs v1 (hors ligne) s'en servait.

BEGIN;

ALTER TABLE public.activity_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_modifications ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_question_answers ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_speaker_connections ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_speakers ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.email_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_answers ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_questions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.group_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.innovation_reactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.lesson_contents ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.live_quizzes ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiation_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiation_sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiator_consultations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.newsletter_campaigns ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.newsletter_lists ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.organization_aliases ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.organization_validations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.participant_chapter_progress ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.participant_lesson_progress ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.poll_responses ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.polls ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_questions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_responses ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_results ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.ratings ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.rls_monitoring ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.session_registrations ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.technical_documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.training_chapters ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_blocks ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_roles ENABLE ROW LEVEL SECURITY;

CREATE POLICY "Lecture par les super administrateurs" ON public.activity_speakers
  FOR SELECT TO authenticated
  USING (EXISTS (
    SELECT 1 FROM public.user_roles r
    WHERE r.user_id = auth.uid() AND r.role = 'super_admin' AND r.is_active
      AND (r.valid_until IS NULL OR r.valid_until > now())
  ));

REVOKE EXECUTE ON FUNCTION public.add_role_simple(uuid, public.user_role_type, timestamptz) FROM PUBLIC, anon, authenticated;
REVOKE EXECUTE ON FUNCTION public.deactivate_role_simple(uuid) FROM PUBLIC, anon, authenticated;
REVOKE EXECUTE ON FUNCTION public.reactivate_role_simple(uuid) FROM PUBLIC, anon, authenticated;
REVOKE EXECUTE ON FUNCTION public.update_role_expiry_simple(uuid, timestamptz) FROM PUBLIC, anon, authenticated;

COMMIT;

-- Retour arrière de fermeture-2026-10-27.sql : rend les tables et les fonctions
-- à leur état du 27/10 au matin (ouvertes à la clé publique). À ne jouer qu'en cas
-- de panne de PACO causée par la fermeture.

BEGIN;

ALTER TABLE public.activity_documents DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_modifications DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_question_answers DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_speaker_connections DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_speakers DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.email_history DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_answers DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_questions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluation_results DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.evaluations DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.group_members DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.innovation_reactions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.lesson_contents DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.live_quizzes DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_groups DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.message_templates DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.messages DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiation_documents DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiation_sessions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.negotiator_consultations DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.newsletter_campaigns DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.newsletter_lists DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.organization_aliases DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.organization_validations DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.participant_chapter_progress DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.participant_lesson_progress DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.poll_responses DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.polls DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_questions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_responses DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_results DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.quiz_subscriptions DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.ratings DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.rls_monitoring DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.session_registrations DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.technical_documents DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.training_chapters DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_blocks DISABLE ROW LEVEL SECURITY;
ALTER TABLE public.user_roles DISABLE ROW LEVEL SECURITY;

DROP POLICY "Lecture par les super administrateurs" ON public.activity_speakers;

GRANT EXECUTE ON FUNCTION public.add_role_simple(uuid, public.user_role_type, timestamptz) TO PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.deactivate_role_simple(uuid) TO PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.reactivate_role_simple(uuid) TO PUBLIC, anon, authenticated;
GRANT EXECUTE ON FUNCTION public.update_role_expiry_simple(uuid, timestamptz) TO PUBLIC, anon, authenticated;

COMMIT;

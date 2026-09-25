-- =============================================================================
-- Guide Négo, étape 2 — données d'essai du savoir (FAQ, parcours, lexique)
--
-- Écrites à la main depuis docs/AppNego/design/donnees-savoir.md et
-- donnees-lexique.md. HORS de docs/database/ : elles ne partent jamais en
-- production. Rejouables : identifiants fixes, ON CONFLICT DO NOTHING.
--
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f specs/013-guide-nego-faq-lexique/donnees-essai.sql
--
-- La seule entrée de FAQ qui a sa réponse est publiée, vérifiée par le premier
-- compte `expert` ; les dix-sept autres restent en brouillon. Les quatre termes
-- « rédigés » sans source citable restent en brouillon : 15 termes publiés.
-- Les sources « Guide IFDD » pointent sur le guide publié de la bibliothèque
-- s'il existe, sinon sur une référence extérieure.
-- =============================================================================

BEGIN;

CREATE TEMP TABLE essai ON COMMIT DROP AS
SELECT (SELECT ra.person_id
          FROM identity.role_assignments ra
         WHERE ra.role_code = 'expert' AND ra.revoked_at IS NULL
         ORDER BY ra.granted_at, ra.id
         LIMIT 1) AS expert_id,
       (SELECT d.id
          FROM negotiation.documents d
         WHERE d.published_at IS NOT NULL AND d.title ->> 'fr' ILIKE 'Guide des négociations%'
         ORDER BY d.published_at DESC
         LIMIT 1) AS guide_id;

DO $$
BEGIN
    IF (SELECT expert_id FROM essai) IS NULL THEN
        RAISE EXCEPTION 'Aucun compte ne porte le rôle « expert » : attribuez-le depuis l''écran des utilisateurs, puis rejouez ce script.';
    END IF;
END;
$$;

-- FAQ

INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000001', t.id, '{"fr": "Où retirer mon badge ?"}', NULL, 'draft', NULL, NULL, 2
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000002', t.id, '{"fr": "Comment lire les sessions du jour ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000003', t.id, '{"fr": "Qui coordonne mon groupe sur l''adaptation ?"}', NULL, 'draft', NULL, NULL, 3
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000004', t.id, '{"fr": "Comment rejoindre la coordination du Groupe africain ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000005', t.id, '{"fr": "Que faire le premier jour ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000006', t.id, '{"fr": "Comment rédiger ma restitution du soir ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'first_cop'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000007', t.id, '{"fr": "Quelle différence entre un groupe de contact et des consultations informelles ?"}', '{"fr": "Les deux sont des réunions de négociation ouvertes à toutes les Parties, et de plus en plus aux observateurs. La différence tient à l''organisation, pas à l''importance : au plus deux groupes de contact peuvent siéger en même temps, contre six consultations informelles, si bien que la plupart des points de l''ordre du jour passent en consultations informelles. Les deux peuvent aboutir à un texte convenu — un document L — soumis à l''adoption. Dans les deux cas, on parle anglais, sans interprétation. Les « informal informals » sont plus petites, se tiennent sans le Secrétariat, et rendent compte au groupe."}', 'published', current_date, e.expert_id, 1
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'process'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000008', t.id, '{"fr": "Qu''est-ce qu''un document L ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'process'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000009', t.id, '{"fr": "Que veut dire un texte entre crochets ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'process'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000010', t.id, '{"fr": "Qui sont les co-facilitateurs ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'process'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000011', t.id, '{"fr": "Comment un point passe-t-il de l''OSMOE à la CdP ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'process'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000012', t.id, '{"fr": "Mon pays est dans quels groupes ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'negotiating_groups'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000013', t.id, '{"fr": "Qui parle au nom du Groupe africain ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'negotiating_groups'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000014', t.id, '{"fr": "Puis-je prendre la parole si mon groupe a déjà parlé ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'negotiating_groups'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000015', t.id, '{"fr": "Que se passe-t-il quand mon pays et mon groupe divergent ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'negotiating_groups'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000016', t.id, '{"fr": "Comment entrer dans une réunion à accès limité ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'on_site'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000017', t.id, '{"fr": "Où trouver l''interprétation en français ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'on_site'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_entries (id, section_term_id, question, answer, status, verified_on, verified_by, editorial_rank)
SELECT '00000000-0013-7000-8000-f00000000018', t.id, '{"fr": "Comment obtenir la version française d''un document ?"}', NULL, 'draft', NULL, NULL, NULL
  FROM reference.taxonomy_terms t, essai e WHERE t.taxonomy_code = 'faq_section' AND t.code = 'on_site'
ON CONFLICT (id) DO NOTHING;

INSERT INTO negotiation.knowledge_sources (id, faq_entry_id, document_id, external_title, section_label, page_from, page_to, quote, sort_order)
SELECT '00000000-0013-7000-8000-d00000000001', '00000000-0013-7000-8000-f00000000007', e.guide_id,
       CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END,
       'annexe A.3', 74, 74, 'Il existe une perception selon laquelle un groupe de contact aurait plus d''importance qu''une consultation informelle. Cependant, il est important de souligner que le travail mené dans le cadre d''une consultation informelle peut, au même titre qu''un groupe de contact, aboutir à un texte convenu (document L).', 1
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, faq_entry_id, external_title, section_label, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000002', '00000000-0013-7000-8000-f00000000007', '« Au nom de ma délégation… », IISD', 'Le contexte de négociation', 38, 39, 2)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.faq_related (entry_id, related_id, sort_order) VALUES
    ('00000000-0013-7000-8000-f00000000007', '00000000-0013-7000-8000-f00000000008', 1),
    ('00000000-0013-7000-8000-f00000000007', '00000000-0013-7000-8000-f00000000010', 2)
ON CONFLICT DO NOTHING;

-- Lexique
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000001', t.id, 'contact group', NULL, ARRAY['contact groups']::text[], '{"fr": "groupe de contact"}', '{"fr": "Réunion établie pour négocier un point de l''ordre du jour. Ouverte à toutes les Parties et, sauf objection, aux observateurs. Peut aboutir à un texte convenu. Au plus deux groupes de contact siègent en même temps."}', 'The contact group will reconvene at 3 p.m.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000003', '00000000-0013-7000-8000-a00000000001', '« Au nom de ma délégation… », IISD', 38, 38, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, document_id, external_title, page_from, page_to, sort_order)
SELECT '00000000-0013-7000-8000-d00000000004', '00000000-0013-7000-8000-a00000000001', e.guide_id, CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END, 74, 74, 2
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000002', t.id, 'informal consultations', NULL, ARRAY['informal consultation']::text[], '{"fr": "consultations informelles"}', '{"fr": "Réunion de négociation créée par les Parties, très proche d''un groupe de contact dans son fonctionnement, et de plus en plus ouverte aux observateurs. La plupart des points de l''ordre du jour passent par là. En anglais, sans interprétation."}', 'Informal consultations on the GGA are moved to room 9.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000005', '00000000-0013-7000-8000-a00000000002', '« Au nom de ma délégation… », IISD', 38, 38, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, document_id, external_title, page_from, page_to, sort_order)
SELECT '00000000-0013-7000-8000-d00000000006', '00000000-0013-7000-8000-a00000000002', e.guide_id, CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END, 74, 74, 2
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000003', t.id, 'informal informals', NULL, '{}'::text[], '{"fr": "— (se dit tel quel)"}', '{"fr": "Quand un point bloque, les co-facilitateurs proposent aux Parties concernées de se retrouver sans facilitation, sans le Secrétariat, pour chercher un terrain d''entente. On oublie les drapeaux ; les délégués s''appellent par leur nom. Tout ce qui s''y dit est rapporté au groupe de contact."}', 'We''ll continue in informal informals after the break.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000007', '00000000-0013-7000-8000-a00000000003', 'Guide des négociations, IIED', 35, 35, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000008', '00000000-0013-7000-8000-a00000000003', '« Au nom de ma délégation… », IISD', 38, 38, 2)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000004', t.id, 'huddle', NULL, ARRAY['huddles']::text[], '{"fr": "aparté"}', '{"fr": "Attroupement improvisé de quelques délégués, dans un coin de salle ou un couloir, pour débloquer une phrase. Sans statut, jamais programmé : on n''en apprend l''existence que par le réseau."}', 'Let''s huddle on paragraph 12.', 'draft'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000005', t.id, 'co-facilitators', NULL, ARRAY['co-facilitator', 'cofacilitators']::text[], '{"fr": "co-facilitateurs"}', '{"fr": "Paire de délégués nommée par la présidence pour conduire les consultations informelles sur un point : l''un d''un pays développé, l''autre d''un pays en développement."}', 'The co-facilitators will prepare a new iteration tonight.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000009', '00000000-0013-7000-8000-a00000000005', '« Au nom de ma délégation… », IISD', 38, 38, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000006', t.id, 'informal stocktaking plenary', NULL, '{}'::text[], '{"fr": "plénière informelle de bilan"}', '{"fr": "En deuxième semaine, la présidence réunit toutes les Parties pour entendre où en est chaque négociation et ce qu''elle prévoit ensuite."}', 'The Presidency convenes an informal stocktaking plenary at 6 p.m.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'meetings'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000010', '00000000-0013-7000-8000-a00000000006', '« Au nom de ma délégation… », IISD', 39, 39, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000007', t.id, 'bracketed text', NULL, ARRAY['bracketed', 'brackets']::text[], '{"fr": "texte entre crochets"}', '{"fr": "Un passage entre crochets n''a pas encore fait l''objet d''un accord : il est en cours de négociation. Les crochets se lèvent un à un ; ne jamais en céder un sans obtenir autre chose."}', 'We still have three brackets in paragraph 12.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000011', '00000000-0013-7000-8000-a00000000007', 'Guide des négociations, IIED', 41, 41, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000012', '00000000-0013-7000-8000-a00000000007', '« Au nom de ma délégation… », IISD', 107, 107, 2)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000008', t.id, 'agreed language', NULL, '{}'::text[], '{"fr": "formulation convenue"}', '{"fr": "Texte repris mot pour mot de la Convention, du Protocole, de l''Accord de Paris ou d''une décision antérieure, pour rédiger, garder un sujet vivant ou sortir d''une impasse."}', 'This is agreed language from decision 1/CP.21.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000013', '00000000-0013-7000-8000-a00000000008', 'Guide des négociations, IIED', 41, 41, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000009', t.id, 'ad ref.', NULL, ARRAY['ad referendum']::text[], '{"fr": "ad referendum"}', '{"fr": "Un alinéa cité « ad ref. » est finalisé alors que le reste du texte se négocie encore : il n''est plus ouvert."}', 'Paragraph 5 is agreed ad ref.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000014', '00000000-0013-7000-8000-a00000000009', 'Guide des négociations, IIED', 41, 41, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000010', t.id, 'PP / OP', NULL, ARRAY['PP', 'OP', 'preambular paragraph', 'operative paragraph']::text[], '{"fr": "alinéa du préambule / du dispositif"}', '{"fr": "Les alinéas du préambule se citent « PP » et ceux du dispositif « OP », suivis d''un numéro."}', 'I have a concern with OP3.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000015', '00000000-0013-7000-8000-a00000000010', 'Guide des négociations, IIED', 41, 41, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000011', t.id, 'bis, ter, alt', NULL, ARRAY['bis', 'ter', 'alt']::text[], '{"fr": "bis, ter, alt"}', '{"fr": "Pour insérer un alinéa après l''OP3, on propose un « OP3bis » ; après lui, un « OP3ter ». « Alt » propose un texte de remplacement : un « PP5alt » supprime le PP5 et le remplace."}', 'My delegation proposes an OP3bis.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000016', '00000000-0013-7000-8000-a00000000011', 'Guide des négociations, IIED', 41, 42, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000012', t.id, 'L document', NULL, ARRAY['L documents', 'L doc']::text[], '{"fr": "document L"}', '{"fr": "Projet de conclusions ou de décision, à diffusion limitée, soumis à l''adoption par l''organe compétent ; traduit dans les six langues, parfois après l''adoption. Sa cote commence par « L. »."}', 'The L document will be issued tonight.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000017', '00000000-0013-7000-8000-a00000000012', '« Au nom de ma délégation… », IISD', 52, 52, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000013', t.id, 'non-paper', NULL, ARRAY['nonpaper', 'non-papers']::text[], '{"fr": "non-papier, note informelle"}', '{"fr": "Texte sans statut officiel, diffusé par une présidence ou des co-facilitateurs pour faire avancer la discussion. Il n''engage personne."}', 'The co-facilitators circulated a non-paper this morning.', 'draft'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000014', t.id, 'landing zone', NULL, '{}'::text[], '{"fr": "zone d''atterrissage"}', '{"fr": "L''espace de compromis où un accord devient possible entre les positions en présence."}', 'I think we are close to a landing zone.', 'draft'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000015', t.id, 'conclusions', NULL, '{}'::text[], '{"fr": "conclusions"}', '{"fr": "Résultat des négociations de l''OSMOE et de l''OSCST sur un point de leur ordre du jour. Sans force juridique, elles recommandent à la CdP et montrent l''orientation prise."}', 'The SBI adopted conclusions on this item.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'texts'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, external_title, page_from, page_to, sort_order) VALUES
    ('00000000-0013-7000-8000-d00000000018', '00000000-0013-7000-8000-a00000000015', 'Guide des négociations, IIED', 42, 42, 1)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000016', t.id, 'global goal on adaptation', 'GGA', '{}'::text[], '{"fr": "objectif mondial en matière d''adaptation"}', '{"fr": "Objectif de l''Accord de Paris visant à élever l''adaptation au niveau de l''atténuation. À la CdP30, l''enjeu était l''adoption de cent indicateurs pour suivre les progrès collectifs."}', 'We need to finalise the GGA indicators.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'themes'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, document_id, external_title, page_from, page_to, sort_order)
SELECT '00000000-0013-7000-8000-d00000000019', '00000000-0013-7000-8000-a00000000016', e.guide_id, CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END, 59, 59, 1
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000017', t.id, 'nationally determined contribution', 'NDC', ARRAY['NDCs']::text[], '{"fr": "CDN — contribution déterminée au niveau national"}', '{"fr": "Engagement climatique que chaque pays fixe lui-même au titre de l''Accord de Paris, et révise tous les cinq ans."}', 'Our new NDC was submitted in September.', 'draft'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'themes'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000018', t.id, 'loss and damage', NULL, ARRAY['L&D']::text[], '{"fr": "pertes et préjudices"}', '{"fr": "Dommages du changement climatique que ni l''atténuation ni l''adaptation n''ont évités. Un fonds de réponse existe depuis la CdP28."}', 'The loss and damage fund needs replenishment.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'themes'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, document_id, external_title, page_from, page_to, sort_order)
SELECT '00000000-0013-7000-8000-d00000000020', '00000000-0013-7000-8000-a00000000018', e.guide_id, CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END, 62, 63, 1
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_entries (id, family_term_id, term, acronym, variants, translation, definition, heard_in_room, status)
SELECT '00000000-0013-7000-8000-a00000000019', t.id, 'global stocktake', 'GST', '{}'::text[], '{"fr": "bilan mondial"}', '{"fr": "Examen collectif, tous les cinq ans, des progrès vers les objectifs de l''Accord de Paris. Le premier s''est conclu à la CdP28."}', 'The GST outcome must guide the next NDCs.', 'published'
  FROM reference.taxonomy_terms t WHERE t.taxonomy_code = 'glossary_family' AND t.code = 'themes'
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.knowledge_sources (id, glossary_entry_id, document_id, external_title, page_from, page_to, sort_order)
SELECT '00000000-0013-7000-8000-d00000000021', '00000000-0013-7000-8000-a00000000019', e.guide_id, CASE WHEN e.guide_id IS NULL THEN 'Guide des négociations — CdP30, IFDD' END, 26, 26, 1
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.glossary_related (entry_id, related_id, sort_order) VALUES
    ('00000000-0013-7000-8000-a00000000001', '00000000-0013-7000-8000-a00000000002', 1),
    ('00000000-0013-7000-8000-a00000000001', '00000000-0013-7000-8000-a00000000003', 2),
    ('00000000-0013-7000-8000-a00000000001', '00000000-0013-7000-8000-a00000000005', 3),
    ('00000000-0013-7000-8000-a00000000002', '00000000-0013-7000-8000-a00000000001', 1),
    ('00000000-0013-7000-8000-a00000000003', '00000000-0013-7000-8000-a00000000001', 1),
    ('00000000-0013-7000-8000-a00000000005', '00000000-0013-7000-8000-a00000000002', 1),
    ('00000000-0013-7000-8000-a00000000007', '00000000-0013-7000-8000-a00000000008', 1),
    ('00000000-0013-7000-8000-a00000000007', '00000000-0013-7000-8000-a00000000011', 2),
    ('00000000-0013-7000-8000-a00000000010', '00000000-0013-7000-8000-a00000000011', 1),
    ('00000000-0013-7000-8000-a00000000011', '00000000-0013-7000-8000-a00000000010', 1),
    ('00000000-0013-7000-8000-a00000000012', '00000000-0013-7000-8000-a00000000015', 1),
    ('00000000-0013-7000-8000-a00000000015', '00000000-0013-7000-8000-a00000000012', 1),
    ('00000000-0013-7000-8000-a00000000016', '00000000-0013-7000-8000-a00000000017', 1),
    ('00000000-0013-7000-8000-a00000000016', '00000000-0013-7000-8000-a00000000019', 2),
    ('00000000-0013-7000-8000-a00000000019', '00000000-0013-7000-8000-a00000000017', 1),
    ('00000000-0013-7000-8000-a00000000018', '00000000-0013-7000-8000-a00000000016', 1)
ON CONFLICT DO NOTHING;

-- Parcours « Ma première COP »
INSERT INTO negotiation.pathway_groups (id, label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-b00000000001', '{"fr": "Avant de partir"}', 10, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000001', '00000000-0013-7000-8000-b00000000001', '{"fr": "Vérifier mon accréditation et ma lettre de nomination."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 10, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published)
SELECT '00000000-0013-7000-8000-c00000000002', '00000000-0013-7000-8000-b00000000001', '{"fr": "Télécharger le Guide des négociations et le Résumé pour les décideurs."}', NULL, NULL, CASE WHEN e.guide_id IS NOT NULL THEN 'document' END, e.guide_id,
       CASE WHEN e.guide_id IS NOT NULL THEN NULL END, NULL,
       CASE WHEN e.guide_id IS NOT NULL THEN '{"fr": "Documents"}'::platform.i18n_text END, 20, true
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published)
SELECT '00000000-0013-7000-8000-c00000000003', '00000000-0013-7000-8000-b00000000001', '{"fr": "Choisir mes thématiques et lire leurs fiches."}', NULL, NULL, CASE WHEN e.guide_id IS NOT NULL THEN 'document' END, e.guide_id,
       CASE WHEN e.guide_id IS NOT NULL THEN 'chapitre 3' END, NULL,
       CASE WHEN e.guide_id IS NOT NULL THEN '{"fr": "Guide, chapitre 3"}'::platform.i18n_text END, 30, true
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published)
SELECT '00000000-0013-7000-8000-c00000000004', '00000000-0013-7000-8000-b00000000001', '{"fr": "Trouver la coordonnatrice de mon groupe sur chaque thématique."}', NULL, NULL, CASE WHEN e.guide_id IS NOT NULL THEN 'document' END, e.guide_id,
       CASE WHEN e.guide_id IS NOT NULL THEN 'annexe A.5' END, NULL,
       CASE WHEN e.guide_id IS NOT NULL THEN '{"fr": "Guide, annexe A.5"}'::platform.i18n_text END, 40, true
  FROM essai e
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000005', '00000000-0013-7000-8000-b00000000001', '{"fr": "Préparer la position de mon pays avec mon point focal."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 50, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_groups (id, label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-b00000000002', '{"fr": "Le premier jour"}', 20, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000006', '00000000-0013-7000-8000-b00000000002', '{"fr": "Retirer mon badge et repérer les salles."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 10, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000007', '00000000-0013-7000-8000-b00000000002', '{"fr": "Assister à l''atelier préparatoire des négociateurs francophones."}', NULL, '{"fr": "Réunions de la Francophonie"}', NULL, NULL, NULL, NULL, NULL, 20, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000008', '00000000-0013-7000-8000-b00000000002', '{"fr": "Assister à la coordination de mon groupe."}', '{"fr": "Groupe africain 8:00, PMA 13:00"}', NULL, NULL, NULL, NULL, NULL, NULL, 30, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000009', '00000000-0013-7000-8000-b00000000002', '{"fr": "Me présenter à la coordonnatrice de ma thématique."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 40, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000010', '00000000-0013-7000-8000-b00000000002', '{"fr": "Régler mes notifications pour mes thématiques."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 50, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_groups (id, label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-b00000000003', '{"fr": "En salle"}', 30, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000011', '00000000-0013-7000-8000-b00000000003', '{"fr": "Vérifier la salle et l''heure juste avant : les sessions se déplacent."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 10, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000012', '00000000-0013-7000-8000-b00000000003', '{"fr": "Noter qui parle pour quel groupe."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 20, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000013', '00000000-0013-7000-8000-b00000000003', '{"fr": "Repérer les crochets et les options dans le texte."}', NULL, NULL, 'glossary', NULL, NULL, '00000000-0013-7000-8000-a00000000007', '{"fr": "Lexique, « bracketed text »"}', 30, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000014', '00000000-0013-7000-8000-b00000000003', '{"fr": "Signaler un changement constaté : annulation, déplacement, réunion non annoncée."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 40, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_groups (id, label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-b00000000004', '{"fr": "Le soir"}', 40, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000015', '00000000-0013-7000-8000-b00000000004', '{"fr": "Relire mes notes et rédiger ma restitution."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 10, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000016', '00000000-0013-7000-8000-b00000000004', '{"fr": "Lire le Bulletin des négociations de la Terre du jour."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 20, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000017', '00000000-0013-7000-8000-b00000000004', '{"fr": "Vérifier les sessions de demain pour mes thématiques."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 30, true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO negotiation.pathway_steps (id, group_id, label, detail, origin_label, link_kind, link_document_id, link_section, link_glossary_id, link_label, sort_order, is_published) VALUES
    ('00000000-0013-7000-8000-c00000000018', '00000000-0013-7000-8000-b00000000004', '{"fr": "Poser à un expert la question restée sans réponse."}', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 40, true)
ON CONFLICT (id) DO NOTHING;

COMMIT;

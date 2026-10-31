-- Textes types des courriels et courriels rattachés à un dossier (01/11). Rejouable.
-- Modèle : docs/database/110_engagement.sql § 12 (engagement.mail_texts,
-- engagement.mailings.mail_text_id et proposal_id, semence des neuf textes).
--
-- « Écrire au déposant » depuis la fiche d'un dossier ouvre la fenêtre de
-- rédaction sur un brouillon rattaché au dossier ; on y choisit un texte type,
-- dont les valeurs du dossier (activité, date, horaire, salle) se remplissent
-- seules. Additive : aucune colonne existante ne change.
-- Jouer AVANT de démarrer l'API et le worker, qui lisent ces colonnes.
BEGIN;

-- TEXTES TYPES (01/11/2026, demande du commanditaire). Un objet et un corps
-- prêts à l'emploi (« Confirmation de sélection », « Liste d'attente »…), qu'on
-- choisit dans la fenêtre de rédaction : il remplace l'objet et le corps du
-- brouillon. Notion distincte du gabarit, qui règle l'APPARENCE (mise en page,
-- images) et jamais le texte : un même texte part sous n'importe quel gabarit.
--
-- DÉFAUT V1 CORRIGÉ : les neuf textes de la v1 vivaient dans le code du site
-- (`activityEmailTemplates.js`), dates limites et adresses en copie écrites en
-- dur ; changer une virgule demandait un déploiement. Ici ce sont des lignes,
-- modifiables au back-office, et leurs valeurs viennent des variables.
--
-- Donnée modifiable par un administrateur, donc ni i18n ni traduction : un
-- courriel du back-office part dans une langue (comme `mailings`). Le corps est
-- assaini à l'écriture, avec la même liste blanche que les envois. Il cite les
-- variables du destinataire ({{prenom}}, {{edition}}…) et, pour un courriel
-- écrit depuis un dossier, celles du dossier ({{activite}}, {{date}},
-- {{heures}}, {{salle}}) : dans un envoi groupé, celles-ci n'ont pas de valeur
-- et l'envoi le refuse en les nommant. Une date limite ne s'écrit pas en dur :
-- le texte porte un passage à compléter entre crochets (« [date limite] »), que
-- la fenêtre de rédaction signale tant qu'il reste.
CREATE TABLE IF NOT EXISTS engagement.mail_texts (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    name            text        NOT NULL,
    -- Regroupement libre dans la fenêtre de rédaction (« Journées spéciales ») :
    -- un mot de l'équipe, pas un vocabulaire du modèle.
    category        text,
    subject         text        NOT NULL DEFAULT '',
    body_html       text        NOT NULL DEFAULT '',
    -- Ordre d'affichage dans la catégorie, puis le nom.
    sort_order      smallint    NOT NULL DEFAULT 0,
    -- Nul : commun à toutes les éditions (permission globale pour l'écrire),
    -- comme un gabarit.
    event_id        uuid        CONSTRAINT xmod_fk_mail_texts_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    created_by      uuid        CONSTRAINT xmod_fk_mail_texts_author
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_mail_texts_name CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    CONSTRAINT ck_mail_texts_category CHECK (category IS NULL OR length(btrim(category)) BETWEEN 1 AND 60),
    -- La borne de l'objet d'un envoi : un texte type doit pouvoir y entrer.
    CONSTRAINT ck_mail_texts_subject CHECK (length(subject) <= 250)
);

CREATE INDEX IF NOT EXISTS ix_mail_texts_event ON engagement.mail_texts (event_id);

DROP TRIGGER IF EXISTS tg_mail_texts_updated_at ON engagement.mail_texts;
CREATE TRIGGER tg_mail_texts_updated_at
    BEFORE UPDATE ON engagement.mail_texts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_mail_texts_audit ON engagement.mail_texts;
CREATE TRIGGER tg_mail_texts_audit
    AFTER INSERT OR UPDATE OR DELETE ON engagement.mail_texts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE engagement.mail_texts IS
    'Textes types des courriels du back-office : objet et corps prêts à l''emploi, choisis dans la fenêtre de rédaction ; variables du destinataire et, depuis un dossier, du dossier.';

ALTER TABLE engagement.mailings
    ADD COLUMN IF NOT EXISTS mail_text_id uuid
        REFERENCES engagement.mail_texts(id) ON DELETE SET NULL;

ALTER TABLE engagement.mailings
    ADD COLUMN IF NOT EXISTS proposal_id uuid;
ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS xmod_fk_mailings_proposal;
ALTER TABLE engagement.mailings ADD CONSTRAINT xmod_fk_mailings_proposal
    FOREIGN KEY (proposal_id) REFERENCES programme.proposals(id) ON DELETE SET NULL;

-- L'historique des courriels d'un dossier, sur sa fiche.
CREATE INDEX IF NOT EXISTS ix_mailings_proposal ON engagement.mailings (proposal_id, created_at DESC)
    WHERE proposal_id IS NOT NULL;

-- LES NEUF TEXTES TYPES DE LA V1 (01/11), communs à toutes les éditions. Repris
-- de `activityEmailTemplates.js` : l'édition en dur (« CdP31 ») devient
-- {{edition}}, le créneau confirmé {{activite}}, {{date}} et {{heures}} (fuseau
-- compris : « heure d'Antalya » n'est plus écrit) ; les dates limites et le
-- lien du formulaire d'engagement, propres à chaque édition, deviennent des
-- passages à compléter. Les deux adresses personnelles que la v1 mettait en
-- copie ne sont pas reprises : la copie de l'équipe est un réglage
-- (`engagement.mailing.team_cc`). Semés dans une table vide seulement : rejouer
-- ne recrée pas un texte que l'équipe a supprimé ou renommé.
INSERT INTO engagement.mail_texts (name, category, sort_order, subject, body_html)
SELECT v.name, v.category, v.sort_order, v.subject, v.body_html
FROM (VALUES
    ('Confirmation de sélection', 'Sélection et créneaux', 10,
     $t$Confirmation de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous avons le plaisir de vous confirmer que votre proposition d'activité a été retenue par notre comité de sélection pour être organisée au Pavillon de la Francophonie lors de la {{edition}}.</p><p>En raison du nombre important de propositions reçues, le programme a fait l'objet de plusieurs ajustements, et certains créneaux ont pu être modifiés par rapport aux demandes initiales. Nous vous invitons donc à vérifier attentivement le créneau retenu pour votre activité :</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Nous vous remercions de bien vouloir nous confirmer ce créneau par retour de courriel au plus tard le [date limite], afin que votre activité puisse figurer au programme définitif.</p><p>Pour confirmer votre acceptation de notre proposition, nous vous invitons à télécharger le formulaire d'engagement, puis à le remplir, le signer et le joindre à votre courriel.</p><p>Lien pour télécharger le formulaire : [lien du formulaire d'engagement]</p><p>Advenant que vous soumettiez des modifications, la prise en compte de cette demande pourrait occasionner des délais.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Réajustement du créneau', 'Sélection et créneaux', 20,
     $t$Réajustement du créneau de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>N'ayant pas reçu de réponse de votre part dans le délai indiqué dans notre précédent courriel, et compte tenu des contraintes de calendrier de certaines organisations, nous nous sommes permis de réajuster le créneau initialement proposé pour votre activité. Vous trouverez ci-dessous le nouveau créneau :</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Nous vous remercions de bien vouloir nous faire part de votre retour au plus tard le [date limite], afin que votre activité puisse figurer au programme définitif.</p><p>Pour confirmer votre acceptation de ce créneau, nous vous invitons à télécharger le formulaire d'engagement, puis à le remplir, le signer et le joindre à votre courriel.</p><p>Lien pour télécharger le formulaire : [lien du formulaire d'engagement]</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Nouvelle proposition de créneau', 'Sélection et créneaux', 30,
     $t$Proposition d'un nouveau créneau pour votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Compte tenu des contraintes de calendrier de votre organisation et des propositions que vous nous avez transmises, nous vous soumettons le nouveau créneau suivant :</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Nous vous remercions de bien vouloir nous faire part de votre retour au plus tard le [date limite], afin que votre activité puisse figurer au programme définitif.</p><p>Pour confirmer votre acceptation de ce créneau, nous vous invitons à télécharger le formulaire d'engagement, puis à le remplir, le signer et le joindre à votre courriel.</p><p>Lien pour télécharger le formulaire : [lien du formulaire d'engagement]</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Modification d''un créneau validé', 'Sélection et créneaux', 40,
     $t$Modification du créneau de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous vous remercions d'avoir confirmé votre participation au programme du Pavillon de la Francophonie lors de la {{edition}}.</p><p>En raison de contraintes d'organisation, nous avons dû déplacer le créneau de votre activité, qui avait pourtant été validé. Nous vous prions de bien vouloir nous excuser pour ce changement. Votre activité est désormais programmée comme suit :</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Nous vous remercions de bien vouloir nous confirmer, dans les meilleurs délais, que ce nouveau créneau vous convient.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Rectification du créneau', 'Sélection et créneaux', 50,
     $t$Rectification du créneau de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous revenons vers vous au sujet de votre activité au Pavillon de la Francophonie lors de la {{edition}}.</p><p>Une erreur s'est glissée dans le créneau qui vous a été communiqué dans notre précédent courriel, et nous vous prions de bien vouloir nous en excuser. Le créneau correct est le suivant :</p><p>Titre : {{activite}}<br>Date : {{date}}<br>Horaire : {{heures}}</p><p>Ce créneau remplace celui qui vous avait été indiqué précédemment. Nous vous remercions de bien vouloir nous confirmer ce créneau dans les meilleurs délais, afin que votre activité puisse figurer au programme définitif.</p><p>Pour confirmer votre acceptation de ce créneau, nous vous invitons à télécharger le formulaire d'engagement, puis à le remplir, le signer et le joindre à votre courriel.</p><p>Lien pour télécharger le formulaire : [lien du formulaire d'engagement]</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Liste d''attente', 'Liste d''attente et annulation', 60,
     $t$Programmation de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous vous remercions sincèrement pour l'intérêt que vous portez au Pavillon de la Francophonie et pour votre proposition d'activité.</p><p>Cette année, le Pavillon a reçu un très grand nombre de propositions, pour un nombre de créneaux limité. Malgré tous nos efforts, nous ne sommes pas encore parvenus à trouver un créneau qui s'accorde avec vos contraintes de calendrier.</p><p>Activité concernée :<br>Titre : {{activite}}</p><p>Nous avons donc inscrit votre activité sur notre liste d'attente, afin de pouvoir revenir vers vous dès qu'un créneau adapté se libérera.</p><p>Nous vous remercions de votre compréhension et espérons avoir le plaisir de vous accueillir au Pavillon.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Annulation sans réponse', 'Liste d''attente et annulation', 70,
     $t$Programmation de votre activité au Pavillon de la Francophonie ({{edition}}) - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous vous remercions pour l'intérêt que vous portez au Pavillon de la Francophonie.</p><p>Nous vous avions adressé un courriel vous invitant à confirmer le créneau retenu pour votre activité lors de la {{edition}}. Sauf erreur de notre part, nous n'avons pas reçu de retour dans le délai indiqué. Afin de pouvoir finaliser le programme, nous avons dû, à regret, libérer ce créneau, et votre activité ne figure donc plus au programme.</p><p>Activité concernée :<br>Titre : {{activite}}</p><p>Nous espérons avoir le plaisir de collaborer avec vous, lors de cette conférence ou d'une prochaine édition.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Journée Jeunesse et climat', 'Journées spéciales', 80,
     $t$Précision sur votre activité au Pavillon de la Francophonie ({{edition}}) - Journée spéciale Jeunesse et climat - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous tenons à vous préciser que votre activité a été intégrée à la Journée spéciale Jeunesse et climat, en raison de son lien avec les enjeux de la jeunesse.</p><p>Le créneau qui vous a été communiqué correspond à une plage horaire. En raison du nombre important d'activités programmées ce jour-là, la durée effective de votre intervention pourrait s'avérer plus courte que prévu.</p><p>Activité concernée :<br>Titre : {{activite}}<br>Date : {{date}}<br>Plage horaire : {{heures}}</p><p>L'horaire précis et la durée de votre intervention vous seront communiqués prochainement. Nous vous invitons d'ores et déjà à prévoir un format concis et adaptable.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$),
    ('Journée Finance climat', 'Journées spéciales', 90,
     $t$Précision sur votre activité au Pavillon de la Francophonie ({{edition}}) - Journée spéciale Finance climat - {{activite}}$t$,
     $t$<p>Madame, Monsieur,</p><p>Nous tenons à vous préciser que votre activité a été intégrée à la Journée spéciale Finance climat, en raison de son lien avec les enjeux du financement de l'action climatique.</p><p>Le créneau qui vous a été communiqué correspond à une plage horaire. En raison du nombre important d'activités programmées ce jour-là, la durée effective de votre intervention pourrait s'avérer plus courte que prévu.</p><p>Activité concernée :<br>Titre : {{activite}}<br>Date : {{date}}<br>Plage horaire : {{heures}}</p><p>L'horaire précis et la durée de votre intervention vous seront communiqués prochainement. Nous vous invitons d'ores et déjà à prévoir un format concis et adaptable.</p><p>Merci de répondre en conservant en copie l'ensemble des destinataires de ce courriel.</p>$t$)
) AS v(name, category, sort_order, subject, body_html)
WHERE NOT EXISTS (SELECT 1 FROM engagement.mail_texts);

COMMIT;

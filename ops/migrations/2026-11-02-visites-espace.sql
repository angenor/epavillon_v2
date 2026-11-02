-- Visites de l'espace d'une organisation (02/11/2026). Additive, rejouable.
-- Modèle : docs/database/040_organizations.sql § 8.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

CREATE TABLE IF NOT EXISTS org.space_visits (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    organization_id uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_space_visits_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    visit_on        date        NOT NULL DEFAULT ((now() AT TIME ZONE 'UTC')::date),
    proposal_id     uuid,
    first_at        timestamptz NOT NULL DEFAULT now(),
    last_at         timestamptz NOT NULL DEFAULT now(),
    open_count      integer     NOT NULL DEFAULT 1,
    CONSTRAINT ck_space_visits_count CHECK (open_count > 0),
    CONSTRAINT ck_space_visits_span  CHECK (last_at >= first_at),
    CONSTRAINT ux_space_visits UNIQUE NULLS NOT DISTINCT (organization_id, person_id, visit_on, proposal_id)
);

CREATE INDEX IF NOT EXISTS ix_space_visits_organization ON org.space_visits (organization_id, last_at DESC);
CREATE INDEX IF NOT EXISTS ix_space_visits_proposal     ON org.space_visits (proposal_id, last_at DESC)
    WHERE proposal_id IS NOT NULL;

COMMENT ON TABLE org.space_visits IS
    'Ouvertures de l''espace réservé d''une organisation par ses membres actifs : une ligne par personne, page et jour UTC. Lue par l''onglet « Visites » de la fiche du back-office.';
COMMENT ON COLUMN org.space_visits.proposal_id IS
    'Dossier ouvert (programme.proposals.id, sans clé étrangère), NULL pour la page « Mon organisation ».';
COMMENT ON COLUMN org.space_visits.open_count IS
    'Ouvertures de cette page ce jour-là, la première comprise.';

COMMIT;

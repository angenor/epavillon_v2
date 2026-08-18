/**
 * Données simulées du schéma `identity` — vingt-huit personnes et leurs
 * attributions de rôles.
 *
 * DEUX CAS À NE PAS PERDRE DE VUE, ce sont eux qui donnent sa valeur à ce jeu :
 *
 * 1. `PERSON.perretAdmin` est administratrice de la SEULE édition COP31
 *    (`scope_type: 'event'`). Toute liste du back-office doit se filtrer sur son
 *    périmètre, y compris quand elle forge une URL. Sans cette personne, le
 *    filtrage ne serait jamais éprouvé — et c'est la faille que la v1 a payée
 *    d'une page d'administration séparée, codée dans l'urgence.
 * 2. `PERSON.ilboudo` n'a AUCUNE attribution : `administered_events()` lui
 *    renvoie `{ is_global: false, event_ids: [] }`. C'est le cas « accès
 *    refusé », l'un des quatre états obligatoires de chaque écran.
 *
 * Les comptes (`identity.accounts`) ne sont pas simulés : ils ne portent que des
 * secrets et des dates de connexion, qui ne franchissent pas la frontière de
 * l'API. Les écrans du jalon n'en consomment aucun champ.
 */

import type { Person, PersonStatus, RoleAssignment } from '~/types/identity'
import { COUNTRY, EVENT, ORG, PERSON, ROLE_ASSIGNMENT } from './ids'

/** Raccourci d'écriture : les champs constants d'une personne active. */
function person(
  id: string,
  first_name: string,
  last_name: string,
  primary_email: string,
  fields: {
    civility?: Person['civility']
    job_title?: string | null
    country_id?: string | null
    city?: string | null
    timezone: string
    preferred_locale?: string
    organization?: string | null
    biography?: Person['biography']
    phone?: string | null
    status?: PersonStatus
    status_reason?: string | null
    suspended_until?: string | null
    is_directory_visible?: boolean
    /**
     * Adresse vérifiée ? Vrai par défaut. Passer `false` décrit un compte créé
     * dont le lien de vérification n'a pas encore été suivi — l'état dans lequel
     * se trouve toute personne entre l'inscription et son premier clic
     * (prompt A1). Sans ce cas, la connexion d'un compte non vérifié ne serait
     * jamais éprouvée.
     */
    email_verified?: boolean
    created_at: string
  },
): Person {
  const isVerified = fields.email_verified ?? fields.status !== 'suspended'

  return {
    id,
    primary_email,
    email_verified_at: isVerified ? fields.created_at : null,
    first_name,
    last_name,
    civility: fields.civility ?? null,
    display_name: `${first_name} ${last_name}`,
    phone: fields.phone ?? null,
    job_title: fields.job_title ?? null,
    biography: fields.biography ?? null,
    country_id: fields.country_id ?? null,
    city: fields.city ?? null,
    preferred_locale: fields.preferred_locale ?? 'fr',
    timezone: fields.timezone,
    primary_organization_id: fields.organization ?? null,
    status: fields.status ?? 'active',
    status_reason: fields.status_reason ?? null,
    status_changed_by: fields.status_reason ? PERSON.bakayoko : null,
    status_changed_at: fields.status_reason ? '2026-07-20T10:15:00Z' : null,
    suspended_until: fields.suspended_until ?? null,
    is_directory_visible: fields.is_directory_visible ?? true,
    created_at: fields.created_at,
    updated_at: fields.created_at,
  }
}

// ---------------------------------------------------------------------------
// Les personnes
// ---------------------------------------------------------------------------

export const people = [
  // --- Équipe de l'IFDD -----------------------------------------------------
  person(PERSON.adminPivot, 'Administration', 'ePavillon', 'admin@epavillonclimatique.francophonie.org', {
    job_title: 'Compte technique',
    country_id: COUNTRY.ca,
    city: 'Québec',
    timezone: 'America/Montreal',
    organization: ORG.ifdd,
    is_directory_visible: false,
    created_at: '2026-01-12T09:00:00Z',
  }),
  person(PERSON.bakayoko, 'Aminata', 'Bakayoko', 'a.bakayoko@ifdd.francophonie.org', {
    civility: 'mme',
    job_title: 'Responsable de la programmation',
    country_id: COUNTRY.ca,
    city: 'Québec',
    timezone: 'America/Montreal',
    organization: ORG.ifdd,
    phone: '+1 418 692 5727',
    biography: {
      fr: "Coordonne le pavillon de la Francophonie depuis quatre éditions et anime le comité de sélection des activités.",
    },
    created_at: '2026-01-14T08:30:00Z',
  }),
  person(PERSON.tremblay, 'Louis', 'Tremblay', 'l.tremblay@ifdd.francophonie.org', {
    civility: 'm',
    job_title: "Chargé de programme énergie et climat",
    country_id: COUNTRY.ca,
    city: 'Québec',
    timezone: 'America/Montreal',
    organization: ORG.ifdd,
    created_at: '2026-01-14T08:35:00Z',
  }),
  person(PERSON.perretAdmin, 'Claire', 'Perret', 'c.perret@francophonie.org', {
    civility: 'mme',
    job_title: 'Coordonnatrice de la COP31',
    country_id: COUNTRY.fr,
    city: 'Paris',
    timezone: 'Europe/Paris',
    organization: ORG.ifdd,
    biography: {
      fr: "Détachée auprès de l'IFDD pour la seule édition COP31 : son accès au back-office s'arrête à cet événement.",
    },
    created_at: '2026-02-02T09:00:00Z',
  }),
  person(PERSON.nkoDiop, 'Fatou', 'Nko Diop', 'f.nkodiop@ifdd.francophonie.org', {
    civility: 'mme',
    job_title: 'Programmatrice',
    country_id: COUNTRY.sn,
    city: 'Dakar',
    timezone: 'Africa/Dakar',
    organization: ORG.ifdd,
    created_at: '2026-02-05T11:20:00Z',
  }),

  // --- Comité de sélection --------------------------------------------------
  person(PERSON.lemoine, 'Hélène', 'Lemoine', 'h.lemoine@cudcm.ca', {
    civility: 'pr',
    job_title: 'Professeure de droit international de l’environnement',
    country_id: COUNTRY.ca,
    city: 'Montréal',
    timezone: 'America/Montreal',
    organization: ORG.cudcm,
    preferred_locale: 'fr',
    biography: {
      fr: "Titulaire de la chaire de droit climatique, elle préside le comité de sélection de la COP31.",
      en: 'Holder of the climate law chair, chairs the COP31 selection committee.',
    },
    created_at: '2026-03-02T13:00:00Z',
  }),
  person(PERSON.rasoanaivo, 'Mialy', 'Rasoanaivo', 'm.rasoanaivo@mediavert-oi.mg', {
    civility: 'mme',
    job_title: 'Rédactrice en chef',
    country_id: COUNTRY.mg,
    city: 'Antananarivo',
    timezone: 'Indian/Antananarivo',
    organization: ORG.mvoi,
    created_at: '2026-03-06T07:45:00Z',
  }),
  person(PERSON.benAmor, 'Slim', 'Ben Amor', 's.benamor@imre.ma', {
    civility: 'dr',
    job_title: 'Directeur de recherche',
    country_id: COUNTRY.ma,
    city: 'Rabat',
    timezone: 'Africa/Casablanca',
    organization: ORG.imre,
    created_at: '2026-03-09T10:10:00Z',
  }),
  person(PERSON.kabore, 'Alizeta', 'Kaboré', 'a.kabore@osed-sahel.org', {
    civility: 'dr',
    job_title: "Cheffe du département accès à l'énergie",
    country_id: COUNTRY.bf,
    city: 'Ouagadougou',
    timezone: 'Africa/Ouagadougou',
    organization: ORG.osed,
    created_at: '2026-03-12T09:05:00Z',
  }),
  person(PERSON.duchesne, 'Marc', 'Duchesne', 'm.duchesne@ifdd.francophonie.org', {
    civility: 'm',
    job_title: 'Expert finance climatique',
    country_id: COUNTRY.be,
    city: 'Bruxelles',
    timezone: 'Europe/Brussels',
    organization: ORG.ifdd,
    created_at: '2026-03-16T14:25:00Z',
  }),

  // --- Référents et membres d'organisations ---------------------------------
  person(PERSON.sowFall, 'Awa', 'Sow Fall', 'a.sowfall@roac-afrique.org', {
    civility: 'mme',
    job_title: 'Directrice exécutive',
    country_id: COUNTRY.sn,
    city: 'Dakar',
    timezone: 'Africa/Dakar',
    organization: ORG.roac,
    phone: '+221 77 512 08 44',
    biography: {
      fr: "Anime depuis 2019 le réseau des associations littorales d'Afrique de l'Ouest sur l'érosion et la relocalisation.",
    },
    created_at: '2026-02-19T11:05:00Z',
  }),
  person(PERSON.ouedraogo, 'Boureima', 'Ouédraogo', 'b.ouedraogo@osed-sahel.org', {
    civility: 'm',
    job_title: 'Directeur',
    country_id: COUNTRY.bf,
    city: 'Ouagadougou',
    timezone: 'Africa/Ouagadougou',
    organization: ORG.osed,
    created_at: '2026-02-24T16:30:00Z',
  }),
  person(PERSON.zinsou, 'Gilbert', 'Zinsou', 'g.zinsou@anteb.bj', {
    civility: 'm',
    job_title: 'Directeur de la transition écologique',
    country_id: COUNTRY.bj,
    city: 'Cotonou',
    timezone: 'Africa/Porto-Novo',
    organization: ORG.anteb,
    created_at: '2026-03-28T07:50:00Z',
  }),
  person(PERSON.ngoBassong, 'Estelle', 'Ngo Bassong', 'e.ngobassong@cofemac.org', {
    civility: 'mme',
    job_title: 'Coordonnatrice générale',
    country_id: COUNTRY.cm,
    city: 'Yaoundé',
    timezone: 'Africa/Douala',
    organization: ORG.cofemac,
    biography: {
      fr: "Représente les coopératives féminines du bassin du Congo dans les enceintes régionales de négociation.",
    },
    created_at: '2026-04-10T09:25:00Z',
  }),
  person(PERSON.elFassi, 'Nadia', 'El Fassi', 'n.elfassi@imre.ma', {
    civility: 'dr',
    job_title: 'Chercheuse principale',
    country_id: COUNTRY.ma,
    city: 'Rabat',
    timezone: 'Africa/Casablanca',
    organization: ORG.imre,
    created_at: '2026-04-20T13:15:00Z',
  }),
  person(PERSON.josephPierre, 'Wilner', 'Joseph-Pierre', 'w.josephpierre@fhrc-haiti.org', {
    civility: 'm',
    job_title: 'Responsable des programmes',
    country_id: COUNTRY.ht,
    city: 'Port-au-Prince',
    timezone: 'America/Port-au-Prince',
    organization: ORG.fhrc,
    created_at: '2026-05-06T17:40:00Z',
  }),
  person(PERSON.gagnon, 'Sophie', 'Gagnon', 's.gagnon@cudcm.ca', {
    civility: 'mme',
    job_title: 'Coordonnatrice de la chaire',
    country_id: COUNTRY.ca,
    city: 'Montréal',
    timezone: 'America/Montreal',
    organization: ORG.cudcm,
    created_at: '2026-05-11T08:05:00Z',
  }),
  person(PERSON.tranVanMinh, 'Minh', 'Tran Van', 'm.tranvan@cvdmf.org', {
    civility: 'm',
    job_title: 'Secrétaire général',
    country_id: COUNTRY.vn,
    city: 'Hô Chi Minh-Ville',
    timezone: 'Asia/Ho_Chi_Minh',
    organization: ORG.cvdmf,
    preferred_locale: 'fr',
    created_at: '2026-05-18T04:10:00Z',
  }),
  person(PERSON.moreau, 'Julien', 'Moreau', 'j.moreau@verdeo-solutions.fr', {
    civility: 'm',
    job_title: 'Associé fondateur',
    country_id: COUNTRY.fr,
    city: 'Lyon',
    timezone: 'Europe/Paris',
    organization: ORG.verdeo,
    created_at: '2026-06-02T09:55:00Z',
  }),
  person(PERSON.koffi, 'Yann', 'Koffi', 'y.koffi@ujfc.org', {
    civility: 'm',
    job_title: 'Président',
    country_id: COUNTRY.ci,
    city: 'Abidjan',
    timezone: 'Africa/Abidjan',
    organization: ORG.ujfc,
    biography: {
      fr: "Coordonne les délégations de jeunes francophones aux conférences des Parties depuis la COP28.",
    },
    created_at: '2026-06-09T10:20:00Z',
  }),
  person(PERSON.rakotomalala, 'Hery', 'Rakotomalala', 'h.rakotomalala@mediavert-oi.mg', {
    civility: 'm',
    job_title: 'Journaliste environnement',
    country_id: COUNTRY.mg,
    city: 'Antananarivo',
    timezone: 'Indian/Antananarivo',
    organization: ORG.mvoi,
    created_at: '2026-07-28T06:15:00Z',
  }),
  person(PERSON.mbayeNdiaye, 'Ousmane', 'Mbaye Ndiaye', 'o.mbayendiaye@roac-afrique.org', {
    civility: 'm',
    job_title: 'Chargé de projet littoral',
    country_id: COUNTRY.sn,
    city: 'Saint-Louis',
    timezone: 'Africa/Dakar',
    organization: ORG.roac,
    created_at: '2026-03-02T10:40:00Z',
  }),

  // --- Sans rôle particulier ------------------------------------------------
  person(PERSON.compaore, 'Salamata', 'Compaoré', 's.compaore@osed-sahel.org', {
    civility: 'mme',
    job_title: 'Chargée de projets',
    country_id: COUNTRY.bf,
    city: 'Ouagadougou',
    timezone: 'Africa/Ouagadougou',
    // Rattachée à la FICHE EN DOUBLON, celle qu'elle a créée elle-même.
    organization: ORG.osedSigle,
    created_at: '2026-06-11T13:45:00Z',
  }),
  person(PERSON.lambert, 'Céline', 'Lambert', 'celine.lambert@example.org', {
    civility: 'mme',
    job_title: 'Consultante indépendante',
    country_id: COUNTRY.be,
    city: 'Namur',
    timezone: 'Europe/Brussels',
    organization: null,
    status: 'suspended',
    status_reason: "Signalements répétés dans les fils de questions ; suspension temporaire.",
    suspended_until: '2026-09-20T00:00:00Z',
    is_directory_visible: false,
    created_at: '2026-05-29T18:05:00Z',
  }),
  person(PERSON.ilboudo, 'Karim', 'Ilboudo', 'karim.ilboudo@example.org', {
    civility: 'm',
    job_title: 'Étudiant en master environnement',
    country_id: COUNTRY.bf,
    city: 'Bobo-Dioulasso',
    timezone: 'Africa/Ouagadougou',
    organization: null,
    created_at: '2026-07-14T20:30:00Z',
  }),
  // Compte créé, adresse PAS ENCORE VÉRIFIÉE, aucune organisation : l'état exact
  // dans lequel le prompt A1 laisse une personne au sortir de l'inscription, et
  // celui que le prompt A2 reprendra pour le rattachement. Ajouté avec A1 —
  // sans lui, ni l'écran de vérification ni la connexion d'un compte non vérifié
  // n'auraient de donnée à montrer.
  person(PERSON.nouvelInscrit, 'Aïcha', 'Traoré', 'aicha.traore@example.org', {
    civility: 'mme',
    job_title: 'Chargée de plaidoyer climat',
    country_id: COUNTRY.ml,
    city: 'Bamako',
    timezone: 'Africa/Bamako',
    organization: null,
    email_verified: false,
    is_directory_visible: false,
    created_at: '2026-08-14T16:20:00Z',
  }),
  // Adresse VÉRIFIÉE, aucune organisation, et un domaine — `ujfc.org` — que
  // porte une fiche vérifiée en rattachement automatique. Ajoutée avec le prompt
  // A2 : c'est le seul compte du jeu qui puisse se connecter ET voir l'écran de
  // rattachement proposer une organisation d'après son adresse. Aïcha Traoré,
  // juste au-dessus, tient le cas inverse — une adresse générique qui ne révèle
  // rien —, mais elle ne peut pas se connecter tant qu'elle n'a pas vérifié la
  // sienne : les deux cas sont nécessaires, et ils ne tiennent pas dans une
  // seule personne.
  person(PERSON.sy, 'Fatoumata', 'Sy', 'fatoumata.sy@ujfc.org', {
    civility: 'mme',
    job_title: 'Coordonnatrice des campagnes jeunesse',
    country_id: COUNTRY.ci,
    city: 'Abidjan',
    timezone: 'Africa/Abidjan',
    organization: null,
    created_at: '2026-08-16T09:30:00Z',
  }),
  // INVITÉE PAR SON ORGANISATION, et pas encore inscrite : la fiche existe dans
  // `identity.people` sans qu'aucun compte n'y soit rattaché — c'est
  // précisément ce que la séparation personne / compte permet, et ce que la v1
  // ne savait pas faire. Ajoutée avec le prompt A5 : une invitation par adresse
  // crée la personne pour pouvoir créer l'adhésion, la clé étrangère de
  // `org.memberships` l'exigeant.
  person(PERSON.diallo, 'Aminata', 'Diallo', 'a.diallo@roac-afrique.org', {
    civility: 'mme',
    job_title: null,
    country_id: COUNTRY.sn,
    city: 'Saint-Louis',
    timezone: 'Africa/Dakar',
    organization: null,
    email_verified: false,
    is_directory_visible: false,
    created_at: '2026-08-14T10:05:00Z',
  }),
] satisfies Person[]

// ---------------------------------------------------------------------------
// Attributions de rôles
//
// Une attribution porte TOUJOURS une portée. Le code ne teste jamais un nom de
// rôle : il teste une permission (`has_permission`) avec sa portée. Ces lignes
// servent à composer les permissions effectives et à éprouver le filtrage du
// back-office.
// ---------------------------------------------------------------------------

/** Raccourci d'écriture : une attribution en cours de validité. */
function grant(
  n: number,
  person_id: string,
  role_code: string,
  scope_type: RoleAssignment['scope_type'],
  scope_id: string | null,
  granted_at: string,
  note: string | null = null,
): RoleAssignment {
  return {
    id: ROLE_ASSIGNMENT(n),
    person_id,
    role_code,
    scope_type,
    scope_id,
    granted_by: PERSON.adminPivot,
    granted_at,
    valid_from: granted_at,
    valid_until: null,
    revoked_at: null,
    // `ck_role_assignment_revocation` : ni auteur ni motif de retrait tant que
    // l'attribution est vivante. Les deux colonnes ont été ajoutées au modèle au
    // prompt A12 — `note` est le motif de l'OCTROI et ne pouvait pas servir deux
    // fois.
    revoked_by: null,
    revoked_reason: null,
    note,
  }
}

export const roleAssignments = [
  grant(1, PERSON.adminPivot, 'super_admin', 'global', null, '2026-01-12T09:00:00Z'),
  grant(2, PERSON.bakayoko, 'admin', 'global', null, '2026-01-14T08:40:00Z'),
  grant(3, PERSON.tremblay, 'admin', 'global', null, '2026-01-14T08:45:00Z'),

  // L'ADMINISTRATRICE D'UN SEUL ÉVÉNEMENT. Elle voit le même back-office que
  // les administrateurs globaux, restreint à la COP31 et à ce qui en dépend.
  grant(
    4,
    PERSON.perretAdmin,
    'admin',
    'event',
    EVENT.cop31,
    '2026-02-02T09:10:00Z',
    "Détachement pour la seule édition COP31, jusqu'au bilan de décembre 2027.",
  ),

  grant(5, PERSON.nkoDiop, 'programmer', 'event', EVENT.cop31, '2026-02-05T11:30:00Z'),
  grant(6, PERSON.duchesne, 'reviewer', 'event', EVENT.cop31, '2026-03-16T14:30:00Z'),

  // Comité de sélection : le rôle donne le droit d'évaluer, la table
  // `event.call_reviewers` dit qui siège effectivement sur cet appel.
  grant(7, PERSON.lemoine, 'reviewer', 'event', EVENT.cop31, '2026-03-02T13:10:00Z', 'Présidente du comité.'),
  grant(8, PERSON.rasoanaivo, 'reviewer', 'event', EVENT.cop31, '2026-03-06T07:50:00Z'),
  grant(9, PERSON.benAmor, 'reviewer', 'event', EVENT.cop31, '2026-03-09T10:15:00Z'),
  grant(10, PERSON.kabore, 'reviewer', 'event', EVENT.cop31, '2026-03-12T09:10:00Z'),

  // Référents d'organisation : portée `organization`.
  grant(11, PERSON.sowFall, 'org_manager', 'organization', ORG.roac, '2026-02-19T11:10:00Z'),
  grant(12, PERSON.mbayeNdiaye, 'org_member', 'organization', ORG.roac, '2026-03-02T10:45:00Z'),
  grant(13, PERSON.ouedraogo, 'org_manager', 'organization', ORG.osed, '2026-02-24T16:35:00Z'),
  grant(14, PERSON.kabore, 'org_member', 'organization', ORG.osed, '2026-03-12T09:12:00Z'),
  grant(15, PERSON.compaore, 'org_manager', 'organization', ORG.osedSigle, '2026-06-11T13:50:00Z'),
  grant(16, PERSON.zinsou, 'org_manager', 'organization', ORG.anteb, '2026-03-28T07:55:00Z'),
  grant(17, PERSON.ngoBassong, 'org_manager', 'organization', ORG.cofemac, '2026-04-10T09:30:00Z'),
  grant(18, PERSON.elFassi, 'org_manager', 'organization', ORG.imre, '2026-04-20T13:20:00Z'),
  grant(19, PERSON.benAmor, 'org_member', 'organization', ORG.imre, '2026-04-20T13:22:00Z'),
  grant(20, PERSON.josephPierre, 'org_manager', 'organization', ORG.fhrc, '2026-05-06T17:45:00Z'),
  grant(21, PERSON.gagnon, 'org_manager', 'organization', ORG.cudcm, '2026-05-11T08:10:00Z'),
  grant(22, PERSON.lemoine, 'org_member', 'organization', ORG.cudcm, '2026-05-11T08:12:00Z'),
  grant(23, PERSON.tranVanMinh, 'org_manager', 'organization', ORG.cvdmf, '2026-05-18T04:15:00Z'),
  grant(24, PERSON.moreau, 'org_manager', 'organization', ORG.verdeo, '2026-06-02T10:00:00Z'),
  grant(25, PERSON.koffi, 'org_manager', 'organization', ORG.ujfc, '2026-06-09T10:25:00Z'),
  grant(26, PERSON.rakotomalala, 'org_manager', 'organization', ORG.mvoi, '2026-07-28T06:20:00Z'),
  grant(27, PERSON.rasoanaivo, 'org_member', 'organization', ORG.mvoi, '2026-07-28T06:22:00Z'),
  grant(28, PERSON.tremblay, 'editor', 'global', null, '2026-04-01T09:00:00Z'),

  // Attribution ARRIVÉE À SON TERME : la personne a siégé au comité de la COP30
  // et ne doit plus rien voir. Un écran qui oublie `valid_until` la laisse entrer.
  {
    ...grant(29, PERSON.rasoanaivo, 'reviewer', 'global', null, '2025-09-01T08:00:00Z'),
    valid_until: '2025-12-31T23:59:59Z',
  },
  // Attribution RÉVOQUÉE avant terme, conservée pour la traçabilité. Depuis A12,
  // elle dit aussi QUI l'a retirée et POURQUOI : c'est la question qu'on pose
  // six mois plus tard, et une date nue n'y répondait pas.
  {
    ...grant(30, PERSON.lambert, 'org_member', 'organization', ORG.verdeo, '2026-05-29T18:10:00Z'),
    revoked_at: '2026-07-20T10:15:00Z',
    revoked_by: PERSON.bakayoko,
    revoked_reason: "Fin de mission de consultance, confirmée par le référent de l'organisation.",
    note: 'Consultance de six mois sur le volet finance.',
  },

  // ---------------------------------------------------------------------------
  // Ajoutées au prompt A12 — l'écran des rôles a besoin des quatre états d'une
  // attribution, et le jeu n'en portait que trois.
  // ---------------------------------------------------------------------------

  // LE CAS PACO, ENFIN REPRÉSENTÉ. « Confier un webinaire à un responsable qui
  // ne doit voir que son événement » : dans la v1, cela avait imposé une page
  // d'administration séparée, développée dans l'urgence. Ici, une ligne. Claire
  // Perret tenait déjà le cas d'une COP ; il manquait celui d'une édition qui
  // n'est pas une COP et dont le responsable n'appartient pas à l'IFDD — c'est
  // lui que le commanditaire décrit, et lui qui prouve que le back-office se
  // partage sans se dupliquer.
  grant(
    31,
    PERSON.ngoBassong,
    'admin',
    'event',
    EVENT.paco2027,
    '2026-08-03T08:20:00Z',
    "Responsable du cycle de webinaires PACO 2027. Accès borné à cette édition : ni la COP31, ni le référentiel des organisations.",
  ),

  // ATTRIBUTION À VENIR : la prise d'effet est postérieure à aujourd'hui.
  // `effective_permissions()` l'écarte — `valid_from <= now()` —, et un écran qui
  // n'afficherait que « attribué / révoqué » la donnerait pour active alors
  // qu'elle ne l'est pas encore.
  {
    ...grant(
      32,
      PERSON.gagnon,
      'reviewer',
      'event',
      EVENT.cop31,
      '2026-08-10T09:00:00Z',
      "Renfort du comité pour la vague d'évaluation d'octobre.",
    ),
    valid_from: '2026-10-01T00:00:00Z',
    valid_until: '2027-01-15T23:59:59Z',
  },

  // Une attribution retirée SUR QUELQU'UN QUI EN GARDE UNE AUTRE : la fiche de
  // Fatou Nko Diop porte donc à la fois un rôle en cours et un rôle révoqué.
  // Sans ce cas, l'historique d'une personne active resterait toujours vide.
  {
    ...grant(
      33,
      PERSON.nkoDiop,
      'programmer',
      'event',
      EVENT.cop30,
      '2025-06-12T10:00:00Z',
      'Programmation de la COP30.',
    ),
    revoked_at: '2026-01-30T16:45:00Z',
    revoked_by: PERSON.bakayoko,
    revoked_reason: 'Édition clôturée ; les accès de programmation sont retirés au bilan.',
  },
] satisfies RoleAssignment[]

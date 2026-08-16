/**
 * Données simulées de `programme.registrations` — soixante inscriptions.
 *
 * UNE SEULE COLONNE DE PERSONNE, quel que soit le profil : la personne existe
 * toujours dans `identity.people`, avec ou sans compte. C'est la fin de la
 * dualité utilisateur / invité de la v1, qui obligeait chaque écran à traiter
 * deux cas et chaque statistique à additionner deux tables.
 *
 * Les RÉPONSES sont un document JSON dont les clés sont les `code` des champs
 * actifs du formulaire (`mocks/registration-form.ts`). Aucun type figé ne peut
 * les décrire : c'est la conséquence assumée d'un formulaire configurable, pas
 * une lacune à corriger. La validation se fait contre le formulaire chargé.
 *
 * TROIS STATUTS SEULEMENT — inscrit, en liste d'attente, annulé. `attended` et
 * `no_show` supposent que la séance ait eu lieu ; l'édition simulée se tient en
 * novembre 2027. Les inventer ici produirait des taux de participation qui ne
 * veulent rien dire.
 *
 * LA LISTE D'ATTENTE se vérifie sur l'atelier de négociation du 13 novembre,
 * huit places pour onze demandes : les positions se suivent, sans trou, et
 * `waitlist_position` n'est renseigné que pour les personnes en attente
 * (`ck_registrations_waitlist`).
 *
 * LES CANAUX D'ACQUISITION sont variés à dessein : c'est la donnée qui dit à
 * l'IFDD où porter son effort de communication, et elle vient de la taxonomie
 * `referral_source`, jamais d'une liste écrite dans le frontend.
 */

import type { Registration, RegistrationStatus, RegistrationSource } from '~/types/programme/registration'
import { ORG, PERSON, REGISTRATION, SESSION } from './ids'

// ---------------------------------------------------------------------------
// Profils des inscrits
//
// Ce qu'une personne répond au formulaire ne change pas d'une séance à l'autre :
// on l'écrit une fois. Seuls le canal d'acquisition et les besoins particuliers
// varient d'une inscription à l'autre.
// ---------------------------------------------------------------------------

interface Profile {
  job_title: string
  organization: string
  /** Code ISO 3166-1 alpha-2, tel que le renvoie un champ de type `country`. */
  country: string
  badge_unfccc: boolean
  organizationId: string | null
}

const profiles: Record<string, Profile> = {
  [PERSON.sowFall]: { job_title: 'Directrice exécutive', organization: "Réseau ouest-africain pour l'adaptation côtière", country: 'SN', badge_unfccc: true, organizationId: ORG.roac },
  [PERSON.mbayeNdiaye]: { job_title: 'Chargé de projet littoral', organization: "Réseau ouest-africain pour l'adaptation côtière", country: 'SN', badge_unfccc: true, organizationId: ORG.roac },
  [PERSON.ouedraogo]: { job_title: 'Directeur', organization: "Observatoire du Sahel pour l'énergie durable", country: 'BF', badge_unfccc: true, organizationId: ORG.osed },
  [PERSON.kabore]: { job_title: "Cheffe du département accès à l'énergie", organization: "Observatoire du Sahel pour l'énergie durable", country: 'BF', badge_unfccc: true, organizationId: ORG.osed },
  [PERSON.compaore]: { job_title: 'Chargée de projets', organization: 'OSED', country: 'BF', badge_unfccc: false, organizationId: ORG.osedSigle },
  [PERSON.zinsou]: { job_title: 'Directeur de la transition écologique', organization: "Agence nationale de la transition écologique du Bénin", country: 'BJ', badge_unfccc: true, organizationId: ORG.anteb },
  [PERSON.ngoBassong]: { job_title: 'Coordonnatrice générale', organization: 'Coalition des femmes pour le climat en Afrique centrale', country: 'CM', badge_unfccc: true, organizationId: ORG.cofemac },
  [PERSON.elFassi]: { job_title: 'Chercheuse principale', organization: "Institut méditerranéen de recherche sur l'eau", country: 'MA', badge_unfccc: false, organizationId: ORG.imre },
  [PERSON.benAmor]: { job_title: 'Directeur de recherche', organization: "Institut méditerranéen de recherche sur l'eau", country: 'MA', badge_unfccc: true, organizationId: ORG.imre },
  [PERSON.josephPierre]: { job_title: 'Responsable des programmes', organization: 'Fonds haïtien pour la résilience communautaire', country: 'HT', badge_unfccc: true, organizationId: ORG.fhrc },
  [PERSON.gagnon]: { job_title: 'Coordonnatrice de la chaire', organization: 'Chaire universitaire de droit climatique de Montréal', country: 'CA', badge_unfccc: false, organizationId: ORG.cudcm },
  [PERSON.lemoine]: { job_title: 'Professeure de droit international de l’environnement', organization: 'Chaire universitaire de droit climatique de Montréal', country: 'CA', badge_unfccc: true, organizationId: ORG.cudcm },
  [PERSON.tranVanMinh]: { job_title: 'Secrétaire général', organization: 'Consortium des villes durables du Mékong francophone', country: 'VN', badge_unfccc: true, organizationId: ORG.cvdmf },
  [PERSON.moreau]: { job_title: 'Associé fondateur', organization: 'Verdéo Solutions', country: 'FR', badge_unfccc: false, organizationId: ORG.verdeo },
  [PERSON.koffi]: { job_title: 'Président', organization: 'Union des jeunes francophones pour le climat', country: 'CI', badge_unfccc: true, organizationId: ORG.ujfc },
  [PERSON.rakotomalala]: { job_title: 'Journaliste environnement', organization: 'Média Vert Océan Indien', country: 'MG', badge_unfccc: true, organizationId: ORG.mvoi },
  [PERSON.rasoanaivo]: { job_title: 'Rédactrice en chef', organization: 'Média Vert Océan Indien', country: 'MG', badge_unfccc: true, organizationId: ORG.mvoi },
  [PERSON.duchesne]: { job_title: 'Expert finance climatique', organization: 'Institut de la Francophonie pour le développement durable', country: 'BE', badge_unfccc: true, organizationId: ORG.ifdd },
  [PERSON.tremblay]: { job_title: 'Chargé de programme énergie et climat', organization: 'Institut de la Francophonie pour le développement durable', country: 'CA', badge_unfccc: true, organizationId: ORG.ifdd },
  [PERSON.nkoDiop]: { job_title: 'Programmatrice', organization: 'Institut de la Francophonie pour le développement durable', country: 'SN', badge_unfccc: true, organizationId: ORG.ifdd },
  [PERSON.bakayoko]: { job_title: 'Responsable de la programmation', organization: 'Institut de la Francophonie pour le développement durable', country: 'CA', badge_unfccc: true, organizationId: ORG.ifdd },
  [PERSON.perretAdmin]: { job_title: 'Coordonnatrice de la COP31', organization: 'Institut de la Francophonie pour le développement durable', country: 'FR', badge_unfccc: true, organizationId: ORG.ifdd },
  [PERSON.ilboudo]: { job_title: 'Étudiant en master environnement', organization: 'Université Nazi Boni', country: 'BF', badge_unfccc: false, organizationId: null },
  [PERSON.lambert]: { job_title: 'Consultante indépendante', organization: 'Indépendante', country: 'BE', badge_unfccc: false, organizationId: null },
}

// ---------------------------------------------------------------------------
// Fabrique
// ---------------------------------------------------------------------------

interface RegistrationFields {
  /** Code de la taxonomie `referral_source`. */
  referral: string
  status?: RegistrationStatus
  waitlistPosition?: number
  source?: RegistrationSource
  locale?: string
  accessNeeds?: string
  createdAt: string
  cancelledAt?: string
  cancelledReason?: string
}

function reg(n: number, session_id: string, person_id: string, fields: RegistrationFields): Registration {
  const profile = profiles[person_id]!
  const status = fields.status ?? 'registered'

  return {
    id: REGISTRATION(n),
    session_id,
    person_id,
    organization_id: profile.organizationId,
    status,
    answers: {
      job_title: profile.job_title,
      organization: profile.organization,
      country: profile.country,
      badge_unfccc: profile.badge_unfccc,
      referral_source: fields.referral,
      ...(fields.accessNeeds ? { access_needs: fields.accessNeeds } : {}),
    },
    locale: fields.locale ?? 'fr',
    waitlist_position: status === 'waitlisted' ? (fields.waitlistPosition ?? null) : null,
    joined_at: null,
    attendance_minutes: null,
    certificate_asset_id: null,
    source: fields.source ?? 'web',
    cancelled_at: fields.cancelledAt ?? null,
    cancelled_reason: fields.cancelledReason ?? null,
    created_at: fields.createdAt,
    updated_at: fields.cancelledAt ?? fields.createdAt,
  }
}

// ---------------------------------------------------------------------------
// Les inscriptions
// ---------------------------------------------------------------------------

export const registrations = [
  // --- Financer l'adaptation côtière (10 novembre) · 12 inscriptions --------
  reg(1, SESSION.adaptationCotiere, PERSON.zinsou, { referral: 'ifdd_website', createdAt: '2026-09-02T08:15:00Z' }),
  reg(2, SESSION.adaptationCotiere, PERSON.ngoBassong, { referral: 'word_of_mouth', createdAt: '2026-09-02T14:40:00Z' }),
  reg(3, SESSION.adaptationCotiere, PERSON.josephPierre, { referral: 'email_newsletter', createdAt: '2026-09-03T11:05:00Z' }),
  reg(4, SESSION.adaptationCotiere, PERSON.elFassi, { referral: 'ifdd_linkedin', createdAt: '2026-09-03T16:20:00Z' }),
  reg(5, SESSION.adaptationCotiere, PERSON.benAmor, { referral: 'ifdd_linkedin', createdAt: '2026-09-04T07:50:00Z' }),
  reg(6, SESSION.adaptationCotiere, PERSON.rakotomalala, {
    referral: 'ifdd_facebook',
    createdAt: '2026-09-05T06:30:00Z',
    accessNeeds: "Prévoir une place assise proche de la scène pour la prise de vue.",
  }),
  reg(7, SESSION.adaptationCotiere, PERSON.koffi, { referral: 'word_of_mouth', createdAt: '2026-09-05T10:10:00Z' }),
  reg(8, SESSION.adaptationCotiere, PERSON.moreau, { referral: 'ifdd_x', locale: 'fr', createdAt: '2026-09-06T09:00:00Z' }),
  reg(9, SESSION.adaptationCotiere, PERSON.tranVanMinh, { referral: 'email_newsletter', locale: 'en', createdAt: '2026-09-07T02:35:00Z' }),
  reg(10, SESSION.adaptationCotiere, PERSON.gagnon, { referral: 'ifdd_website', createdAt: '2026-09-08T15:25:00Z' }),
  reg(11, SESSION.adaptationCotiere, PERSON.ilboudo, {
    referral: 'ifdd_facebook',
    createdAt: '2026-09-09T19:45:00Z',
    accessNeeds: "Participation en ligne : je n'ai pas d'accréditation zone bleue.",
  }),
  reg(12, SESSION.adaptationCotiere, PERSON.lambert, {
    // ANNULÉE : la personne est par ailleurs suspendue de la plateforme.
    referral: 'other',
    status: 'cancelled',
    createdAt: '2026-09-10T12:00:00Z',
    cancelledAt: '2026-09-18T08:30:00Z',
    cancelledReason: "Annulation à la demande de la personne.",
  }),

  // --- Alerte précoce (11 novembre) · 7 inscriptions -----------------------
  reg(13, SESSION.alertePrecoce, PERSON.sowFall, { referral: 'word_of_mouth', createdAt: '2026-09-11T09:20:00Z' }),
  reg(14, SESSION.alertePrecoce, PERSON.zinsou, { referral: 'ifdd_website', createdAt: '2026-09-11T13:40:00Z' }),
  reg(15, SESSION.alertePrecoce, PERSON.mbayeNdiaye, { referral: 'word_of_mouth', createdAt: '2026-09-12T08:05:00Z' }),
  reg(16, SESSION.alertePrecoce, PERSON.ilboudo, { referral: 'ifdd_facebook', createdAt: '2026-09-12T21:15:00Z' }),
  reg(17, SESSION.alertePrecoce, PERSON.rakotomalala, { referral: 'ifdd_x', createdAt: '2026-09-14T06:50:00Z' }),
  reg(18, SESSION.alertePrecoce, PERSON.ngoBassong, { referral: 'email_newsletter', createdAt: '2026-09-15T10:30:00Z' }),
  reg(19, SESSION.alertePrecoce, PERSON.tranVanMinh, {
    referral: 'ifdd_linkedin',
    locale: 'en',
    source: 'partner',
    createdAt: '2026-09-16T03:10:00Z',
  }),

  // --- Mini-réseaux solaires (11 novembre) · 6 inscriptions ----------------
  reg(20, SESSION.miniReseaux, PERSON.zinsou, { referral: 'ifdd_website', createdAt: '2026-09-17T09:00:00Z' }),
  reg(21, SESSION.miniReseaux, PERSON.josephPierre, { referral: 'email_newsletter', createdAt: '2026-09-17T14:25:00Z' }),
  reg(22, SESSION.miniReseaux, PERSON.compaore, { referral: 'word_of_mouth', createdAt: '2026-09-18T08:40:00Z' }),
  reg(23, SESSION.miniReseaux, PERSON.ilboudo, { referral: 'ifdd_facebook', createdAt: '2026-09-18T20:05:00Z' }),
  reg(24, SESSION.miniReseaux, PERSON.tranVanMinh, { referral: 'ifdd_linkedin', locale: 'en', createdAt: '2026-09-19T04:15:00Z' }),
  reg(25, SESSION.miniReseaux, PERSON.moreau, { referral: 'other', source: 'admin', createdAt: '2026-09-20T11:35:00Z' }),

  // --- Pertes et préjudices (12 novembre) · 9 inscriptions -----------------
  reg(26, SESSION.pertesPrejudices, PERSON.sowFall, { referral: 'word_of_mouth', createdAt: '2026-09-21T08:00:00Z' }),
  reg(27, SESSION.pertesPrejudices, PERSON.ngoBassong, { referral: 'word_of_mouth', createdAt: '2026-09-21T09:30:00Z' }),
  reg(28, SESSION.pertesPrejudices, PERSON.lemoine, { referral: 'ifdd_website', createdAt: '2026-09-21T15:45:00Z' }),
  reg(29, SESSION.pertesPrejudices, PERSON.gagnon, { referral: 'ifdd_website', createdAt: '2026-09-22T10:20:00Z' }),
  reg(30, SESSION.pertesPrejudices, PERSON.zinsou, { referral: 'email_newsletter', createdAt: '2026-09-22T13:55:00Z' }),
  reg(31, SESSION.pertesPrejudices, PERSON.elFassi, { referral: 'ifdd_linkedin', createdAt: '2026-09-23T07:40:00Z' }),
  reg(32, SESSION.pertesPrejudices, PERSON.rasoanaivo, { referral: 'ifdd_x', createdAt: '2026-09-24T05:25:00Z' }),
  reg(33, SESSION.pertesPrejudices, PERSON.koffi, {
    referral: 'ifdd_facebook',
    createdAt: '2026-09-25T09:05:00Z',
    accessNeeds: "Interprétation vers l'anglais souhaitée pour deux membres de la délégation.",
  }),
  reg(34, SESSION.pertesPrejudices, PERSON.ilboudo, { referral: 'word_of_mouth', createdAt: '2026-09-26T18:50:00Z' }),

  // --- Marchés carbone, article 6 (12 novembre) · 6 inscriptions -----------
  reg(35, SESSION.article6, PERSON.duchesne, { referral: 'ifdd_website', source: 'admin', createdAt: '2026-09-27T08:10:00Z' }),
  reg(36, SESSION.article6, PERSON.benAmor, { referral: 'ifdd_linkedin', createdAt: '2026-09-27T11:30:00Z' }),
  reg(37, SESSION.article6, PERSON.zinsou, { referral: 'email_newsletter', createdAt: '2026-09-28T09:15:00Z' }),
  reg(38, SESSION.article6, PERSON.moreau, { referral: 'ifdd_x', createdAt: '2026-09-28T16:40:00Z' }),
  reg(39, SESSION.article6, PERSON.tranVanMinh, { referral: 'ifdd_linkedin', locale: 'en', createdAt: '2026-09-29T03:20:00Z' }),
  reg(40, SESSION.article6, PERSON.rasoanaivo, { referral: 'other', createdAt: '2026-09-30T06:05:00Z' }),

  // --- Accès au Fonds vert (12 novembre) · 7 inscriptions ------------------
  reg(41, SESSION.accesFondsVert, PERSON.josephPierre, { referral: 'word_of_mouth', createdAt: '2026-10-01T10:00:00Z' }),
  reg(42, SESSION.accesFondsVert, PERSON.zinsou, { referral: 'ifdd_website', createdAt: '2026-10-01T12:25:00Z' }),
  reg(43, SESSION.accesFondsVert, PERSON.sowFall, { referral: 'email_newsletter', createdAt: '2026-10-02T08:45:00Z' }),
  reg(44, SESSION.accesFondsVert, PERSON.ngoBassong, { referral: 'email_newsletter', createdAt: '2026-10-02T14:10:00Z' }),
  reg(45, SESSION.accesFondsVert, PERSON.elFassi, { referral: 'ifdd_linkedin', createdAt: '2026-10-03T09:35:00Z' }),
  reg(46, SESSION.accesFondsVert, PERSON.gagnon, { referral: 'ifdd_website', locale: 'en', createdAt: '2026-10-04T15:50:00Z' }),
  reg(47, SESSION.accesFondsVert, PERSON.compaore, {
    referral: 'ifdd_facebook',
    source: 'import',
    createdAt: '2026-10-05T07:20:00Z',
  }),

  // --- Atelier de négociation (13 novembre) · huit places, onze demandes ----
  reg(48, SESSION.atelierNegociation1, PERSON.sowFall, { referral: 'ifdd_website', createdAt: '2026-10-06T08:00:00Z' }),
  reg(49, SESSION.atelierNegociation1, PERSON.zinsou, { referral: 'ifdd_website', createdAt: '2026-10-06T08:12:00Z' }),
  reg(50, SESSION.atelierNegociation1, PERSON.ngoBassong, { referral: 'email_newsletter', createdAt: '2026-10-06T08:40:00Z' }),
  reg(51, SESSION.atelierNegociation1, PERSON.koffi, { referral: 'word_of_mouth', createdAt: '2026-10-06T09:05:00Z' }),
  reg(52, SESSION.atelierNegociation1, PERSON.josephPierre, { referral: 'email_newsletter', createdAt: '2026-10-06T09:30:00Z' }),
  reg(53, SESSION.atelierNegociation1, PERSON.elFassi, { referral: 'ifdd_linkedin', createdAt: '2026-10-06T10:15:00Z' }),
  reg(54, SESSION.atelierNegociation1, PERSON.tranVanMinh, { referral: 'ifdd_linkedin', locale: 'en', createdAt: '2026-10-06T11:00:00Z' }),
  reg(55, SESSION.atelierNegociation1, PERSON.compaore, { referral: 'word_of_mouth', createdAt: '2026-10-06T13:45:00Z' }),
  // Les huit places sont prises : les demandes suivantes passent en attente,
  // avec des positions qui se suivent.
  reg(56, SESSION.atelierNegociation1, PERSON.ilboudo, {
    referral: 'ifdd_facebook',
    status: 'waitlisted',
    waitlistPosition: 1,
    createdAt: '2026-10-06T18:20:00Z',
  }),
  reg(57, SESSION.atelierNegociation1, PERSON.rakotomalala, {
    referral: 'ifdd_x',
    status: 'waitlisted',
    waitlistPosition: 2,
    createdAt: '2026-10-07T05:40:00Z',
  }),
  reg(58, SESSION.atelierNegociation1, PERSON.mbayeNdiaye, {
    referral: 'word_of_mouth',
    status: 'waitlisted',
    waitlistPosition: 3,
    createdAt: '2026-10-07T09:10:00Z',
  }),

  // --- Agroécologie (13 novembre) · 2 inscriptions -------------------------
  reg(59, SESSION.agroecologie, PERSON.kabore, { referral: 'word_of_mouth', createdAt: '2026-10-08T08:30:00Z' }),
  reg(60, SESSION.agroecologie, PERSON.ouedraogo, { referral: 'ifdd_website', createdAt: '2026-10-08T09:00:00Z' }),
] satisfies Registration[]

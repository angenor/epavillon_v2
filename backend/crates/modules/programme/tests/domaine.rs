//! Les règles pures du module, éprouvées **sans base**.
//!
//! C'est ce qui justifie un `domain/` de dix fichiers : dix règles que la base
//! ne porte pas, chacune prouvable seule. Les tests d'intégration, eux, vivent
//! à côté et ouvrent une vraie base.

use programme::domain::{blind, draft, eligibility, facets, limits, sanitize, slug, transitions};
use time::macros::datetime;

// -----------------------------------------------------------------------------
// L'adresse d'URL — le repli et le suffixe (R5, écarts n° 95 et n° 96)
// -----------------------------------------------------------------------------

#[test]
fn adresse_durl_sur_titre_vide() {
    // `platform.slugify('')` rend NULL : c'est le cas du tout premier
    // enregistrement automatique, à la première frappe. Sans repli, la colonne
    // `NOT NULL` refuse et la fonctionnalité entière ne démarre pas.
    assert_eq!(slug::base(None), slug::REPLI);
    assert_eq!(slug::base(Some("")), slug::REPLI);
    assert_eq!(slug::base(Some("   ")), slug::REPLI);
}

#[test]
fn adresse_durl_sur_titre_accentue() {
    // La normalisation est faite EN BASE : ce que reçoit `base()` est déjà
    // passé par `platform.slugify()`. Ce test dit qu'on ne la refait pas.
    assert_eq!(
        slug::base(Some("energies-renouvelables-en-afrique")),
        "energies-renouvelables-en-afrique"
    );
}

#[test]
fn deux_dossiers_homonymes_recoivent_deux_adresses() {
    let base = slug::base(Some("atelier-adaptation"));
    assert_eq!(slug::tentative(&base, 0), "atelier-adaptation");
    assert_eq!(slug::tentative(&base, 1), "atelier-adaptation-2");
    assert_eq!(slug::tentative(&base, 2), "atelier-adaptation-3");
}

#[test]
fn ladresse_laisse_la_place_a_son_suffixe() {
    // `platform.slug` borne à 160 signes. Sans marge, la collision d'un titre
    // très long échouerait à la DEUXIÈME tentative seulement — c'est-à-dire en
    // production, longtemps après la mise en service.
    let long = "a".repeat(200);
    let base = slug::base(Some(&long));
    let derniere = slug::tentative(&base, slug::TENTATIVES_MAX);
    assert!(
        derniere.len() <= 160,
        "adresse trop longue : {}",
        derniere.len()
    );
}

#[test]
fn ladresse_ne_se_termine_jamais_par_un_tiret() {
    // Le domaine `platform.slug` refuse `^[a-z0-9]+(-[a-z0-9]+)*$` : un tiret
    // final le viole.
    let long = format!("{}-{}", "mot".repeat(60), "fin");
    let base = slug::base(Some(&long));
    assert!(!base.ends_with('-'), "adresse : {base}");
}

// -----------------------------------------------------------------------------
// L'assainissement du HTML (R14, écart n° 32)
// -----------------------------------------------------------------------------

#[test]
fn le_html_garde_ce_que_la_barre_doutils_produit() {
    let propre = sanitize::assainir(
        "<p>Un <strong>atelier</strong> et une <em>table ronde</em>.</p>\
         <h3>Objectifs</h3><ul><li>Premier</li></ul><blockquote>Cité</blockquote>",
    );
    assert!(propre.contains("<strong>atelier</strong>"));
    assert!(propre.contains("<em>table ronde</em>"));
    assert!(propre.contains("<h3>Objectifs</h3>"));
    assert!(propre.contains("<li>Premier</li>"));
    assert!(propre.contains("<blockquote>"));
}

#[test]
fn un_attribut_devenement_disparait() {
    let propre = sanitize::assainir(r#"<p onclick="voler()">Texte</p>"#);
    assert!(!propre.contains("onclick"), "{propre}");
    // Le TEXTE survit : supprimer le contenu ferait perdre un paragraphe entier
    // pour un attribut de trop.
    assert!(propre.contains("Texte"), "{propre}");
}

#[test]
fn un_lien_javascript_disparait() {
    let propre = sanitize::assainir(r#"<p><a href="javascript:alert(1)">Cliquer</a></p>"#);
    assert!(!propre.contains("javascript"), "{propre}");
    assert!(propre.contains("Cliquer"), "{propre}");
}

#[test]
fn un_lien_http_survit_et_part_protege() {
    let propre = sanitize::assainir(r#"<p><a href="https://ifdd.francophonie.org">IFDD</a></p>"#);
    assert!(propre.contains("https://ifdd.francophonie.org"));
    // Sans ces deux valeurs, la page ouverte accède à `window.opener`.
    assert!(propre.contains("noopener"), "{propre}");
}

#[test]
fn ni_police_ni_couleur_ne_passent() {
    let propre = sanitize::assainir(
        r#"<p style="color:#ff0000"><span style="font-family:Comic">Rouge</span></p>"#,
    );
    assert!(!propre.contains("style"), "{propre}");
    assert!(!propre.contains("span"), "{propre}");
    assert!(propre.contains("Rouge"));
}

#[test]
fn un_champ_vide_de_lediteur_est_reconnu_vide() {
    // L'éditeur rend `<p></p>` et non la chaîne vide. Sans cette
    // reconnaissance, tout champ « vide » compterait comme rempli.
    assert!(sanitize::est_vide(""));
    assert!(sanitize::est_vide("<p></p>"));
    assert!(sanitize::est_vide("<p>   </p>"));
    assert!(!sanitize::est_vide("<p>Un mot</p>"));
}

// -----------------------------------------------------------------------------
// Les longueurs (R15, écart n° 28)
// -----------------------------------------------------------------------------

#[test]
fn la_longueur_se_compte_en_caracteres_pas_en_octets() {
    // Refuser un résumé de 400 signes accentués parce qu'il fait 460 octets,
    // ce serait refuser le français.
    let quatre_cents = "é".repeat(400);
    assert!(limits::tient(&quatre_cents, &limits::RESUME));
    assert!(!limits::tient(&"é".repeat(401), &limits::RESUME));
}

#[test]
fn la_presentation_se_mesure_sur_le_texte_pas_sur_le_balisage() {
    // Le compteur du front affiche `getText().length`. Deux compteurs
    // divergents sur le même champ, c'est un envoi refusé sans que l'écran
    // l'ait annoncé.
    let html = format!("<p><strong>{}</strong></p>", "a".repeat(4000));
    assert_eq!(limits::longueur_du_texte(&html), 4000);
}

#[test]
fn les_bornes_sont_celles_du_front() {
    // Relevées sur `frontend/app/types/proposal-form.ts` — `TEXT_LIMITS`.
    assert_eq!(limits::TITRE.max, 180);
    assert_eq!(limits::RESUME.max, 400);
    assert_eq!(limits::OBJECTIFS.max, 1200);
    assert_eq!(limits::PRESENTATION.max, 4000);
    assert_eq!(limits::RESULTATS.max, 1200);
    assert_eq!(limits::PUBLIC_VISE.max, 600);
    assert_eq!(limits::CONTRAINTES.max, 500);
    assert_eq!(limits::BIOGRAPHIE.max, 800);
}

// -----------------------------------------------------------------------------
// Le voile de l'aveugle, dans ses quatre combinaisons (R4)
// -----------------------------------------------------------------------------

fn lecteur(appel_en_aveugle: bool, affecte: bool, revue_deposee: bool) -> blind::Lecteur {
    blind::Lecteur {
        appel_en_aveugle,
        affecte,
        revue_deposee,
    }
}

#[test]
fn le_voile_est_baisse_pour_qui_doit_encore_noter() {
    assert!(blind::voile_baisse(lecteur(true, true, false)));
}

#[test]
fn le_voile_se_leve_des_que_sa_revue_est_deposee() {
    assert!(!blind::voile_baisse(lecteur(true, true, true)));
}

#[test]
fn qui_decide_sans_noter_nest_pas_voile() {
    // Masquer les notes à qui doit trancher rendrait la décision impossible.
    assert!(!blind::voile_baisse(lecteur(true, false, false)));
}

#[test]
fn un_appel_a_visage_decouvert_ne_voile_personne() {
    assert!(!blind::voile_baisse(lecteur(false, true, false)));
}

// -----------------------------------------------------------------------------
// La recevabilité, classée avant l'écriture (R9)
// -----------------------------------------------------------------------------

fn appel_ouvert() -> eligibility::EtatDeLAppel {
    eligibility::EtatDeLAppel {
        statut: "open".to_owned(),
        ouvre_le: datetime!(2027-01-01 00:00 UTC),
        echeance: datetime!(2027-06-30 23:59 UTC),
        plafond_par_organisation: None,
        exige_organisation_verifiee: false,
    }
}

fn organisation(dossiers_comptes: i64, verifiee: bool) -> eligibility::EtatDeLOrganisation {
    eligibility::EtatDeLOrganisation {
        dossiers_comptes,
        verifiee,
    }
}

const PENDANT: time::OffsetDateTime = datetime!(2027-03-01 12:00 UTC);
const APRES: time::OffsetDateTime = datetime!(2027-07-01 12:00 UTC);

#[test]
fn un_depot_dans_la_fenetre_est_recevable() {
    let issue = eligibility::classer(&appel_ouvert(), organisation(0, false), true, PENDANT);
    assert!(matches!(issue, eligibility::Recevabilite::Recevable));
}

#[test]
fn un_appel_clos_rend_son_echeance_et_non_une_erreur() {
    // Le contrat attend une RÉPONSE portant une valeur : l'écran doit dire
    // QUAND l'appel a fermé, ce dont l'organisation a précisément besoin.
    let appel = appel_ouvert();
    let issue = eligibility::classer(&appel, organisation(0, false), true, APRES);
    match issue {
        eligibility::Recevabilite::CallClosed { deadline } => assert_eq!(deadline, appel.echeance),
        autre => panic!("attendu call_closed, reçu {autre:?}"),
    }
}

#[test]
fn la_fenetre_ne_sapplique_pas_a_un_renvoi() {
    // Le comité demande ses corrections APRÈS la clôture — c'est le cas normal.
    // Un contrôle indifférencié bloquerait définitivement un dossier que le
    // comité vient lui-même de réclamer (écart n° 38).
    let issue = eligibility::classer(&appel_ouvert(), organisation(0, false), false, APRES);
    assert!(matches!(issue, eligibility::Recevabilite::Recevable));
}

#[test]
fn le_plafond_rend_sa_valeur_et_sapplique_aux_deux_chemins() {
    let mut appel = appel_ouvert();
    appel.plafond_par_organisation = Some(2);

    for premier_depot in [true, false] {
        let issue = eligibility::classer(&appel, organisation(2, false), premier_depot, PENDANT);
        match issue {
            eligibility::Recevabilite::QuotaReached { max } => assert_eq!(max, 2),
            autre => panic!("attendu quota_reached, reçu {autre:?}"),
        }
    }
}

#[test]
fn lappel_clos_lemporte_sur_le_plafond_comme_le_declencheur() {
    // L'ordre du classement doit être celui du déclencheur : sans quoi le
    // contrôle préalable et le dernier mot diraient deux choses différentes du
    // même dépôt.
    let mut appel = appel_ouvert();
    appel.plafond_par_organisation = Some(1);
    let issue = eligibility::classer(&appel, organisation(5, false), true, APRES);
    assert!(matches!(
        issue,
        eligibility::Recevabilite::CallClosed { .. }
    ));
}

#[test]
fn lorganisation_non_verifiee_est_refusee_quand_lappel_lexige() {
    let mut appel = appel_ouvert();
    appel.exige_organisation_verifiee = true;
    let issue = eligibility::classer(&appel, organisation(0, false), true, PENDANT);
    assert!(matches!(
        issue,
        eligibility::Recevabilite::OrganizationNotVerified
    ));
}

// -----------------------------------------------------------------------------
// Les transitions offertes (R7, R8)
// -----------------------------------------------------------------------------

#[test]
fn le_motif_exige_se_lit_sur_ce_que_la_base_a_rendu() {
    use transitions::{AvailableTransition, ProposalStatus};

    let offertes = vec![
        AvailableTransition {
            to_status: ProposalStatus::Accepted,
            requires_reason: false,
        },
        AvailableTransition {
            to_status: ProposalStatus::Rejected,
            requires_reason: true,
        },
    ];

    assert_eq!(
        transitions::motif_exige(&offertes, ProposalStatus::Rejected),
        Some(true)
    );
    assert_eq!(
        transitions::motif_exige(&offertes, ProposalStatus::Accepted),
        Some(false)
    );
    // Non offerte : le service ne tranche pas, il tente et le déclencheur
    // refuse. C'est ce qui garde le graphe à un seul endroit.
    assert_eq!(
        transitions::motif_exige(&offertes, ProposalStatus::Cancelled),
        None
    );
}

#[test]
fn un_motif_despaces_nen_est_pas_un() {
    // Le déclencheur vérifie `btrim(NEW.decision_reason) <> ''`.
    assert!(!transitions::motif_fourni(None));
    assert!(!transitions::motif_fourni(Some("")));
    assert!(!transitions::motif_fourni(Some("   \n ")));
    assert!(transitions::motif_fourni(Some("Hors thématique")));
}

#[test]
fn les_trois_etats_clos_ne_sont_plus_modifiables() {
    use transitions::ProposalStatus::*;

    // **Le dossier RETENU en fait partie** (écart n° 110) : le contrat
    // d'erreurs décrit `PROPOSAL_NOT_EDITABLE` comme « dossier rejeté, retiré,
    // annulé, ou édition terminée », et le commanditaire a tranché le 17/08 —
    // « tant que l'événement n'est pas terminé, il peut modifier ». Une
    // organisation retenue qui repère une coquille trois jours avant sa séance
    // doit pouvoir la corriger ; ce qui reste interdit, c'est de propager la
    // correction à la SÉANCE.
    for etat in [Draft, Submitted, UnderReview, ChangesRequested, Accepted] {
        assert!(etat.est_modifiable(), "{}", etat.as_str());
    }
    for etat in [Rejected, Withdrawn, Cancelled] {
        assert!(!etat.est_modifiable(), "{}", etat.as_str());
    }
}

#[test]
fn les_huit_etats_font_laller_retour_avec_la_base() {
    use transitions::ProposalStatus::*;

    for etat in [
        Draft,
        Submitted,
        UnderReview,
        ChangesRequested,
        Accepted,
        Rejected,
        Withdrawn,
        Cancelled,
    ] {
        assert_eq!(
            transitions::ProposalStatus::from_db(etat.as_str()),
            Some(etat)
        );
    }
    assert_eq!(transitions::ProposalStatus::from_db("approved"), None);
}

// -----------------------------------------------------------------------------
// Les facettes, comptées sur les lignes déjà lues (R16)
// -----------------------------------------------------------------------------

#[test]
fn les_facettes_se_comptent_sur_les_lignes_et_gardent_leur_ordre() {
    // Un jeu de lignes écrit à la main : trois déposés, deux retenus, dans cet
    // ordre d'apparition.
    let lignes = [
        "submitted",
        "accepted",
        "submitted",
        "accepted",
        "submitted",
    ];

    let mut compteur = facets::Compteur::new();
    for statut in lignes {
        compteur.ajouter_code(statut);
    }
    let rendues = compteur.rendre();

    assert_eq!(rendues.len(), 2);
    assert_eq!(rendues[0].value, "submitted");
    assert_eq!(rendues[0].count, 3);
    assert_eq!(rendues[1].value, "accepted");
    assert_eq!(rendues[1].count, 2);
    // Un code d'énumération n'a pas de libellé : l'écran le traduit.
    assert!(rendues[0].label.is_none());
}

#[test]
fn une_facette_declaree_a_zero_reste_visible() {
    // « Retenu (0) » apprend qu'aucun dossier n'est retenu ; une ligne
    // manquante laisse croire que le filtre n'existe pas.
    let mut compteur = facets::Compteur::new();
    compteur.ajouter_code("submitted");
    compteur.declarer("rejected");

    let rendues = compteur.rendre();
    assert_eq!(rendues.len(), 2);
    assert_eq!(rendues[1].value, "rejected");
    assert_eq!(rendues[1].count, 0);
}

#[test]
fn le_libelle_dune_thematique_est_retenu_avec_sa_couleur() {
    let mut compteur = facets::Compteur::new();
    compteur.ajouter(
        "adaptation",
        Some(serde_json::json!({ "fr": "Adaptation" })),
        Some("#00A1E4".to_owned()),
    );
    compteur.ajouter("adaptation", None, None);

    let rendues = compteur.rendre();
    assert_eq!(rendues[0].count, 2);
    assert_eq!(rendues[0].color.as_deref(), Some("#00A1E4"));
    assert_eq!(
        rendues[0].label.as_ref().map(draft::fr).as_deref(),
        Some("Adaptation")
    );
}

// -----------------------------------------------------------------------------
// La recomposition du brouillon (R6)
// -----------------------------------------------------------------------------

#[test]
fn un_texte_vide_ne_devient_pas_un_document_multilingue_vide() {
    assert!(draft::i18n("").is_none());
    assert!(draft::i18n("   ").is_none());
    assert_eq!(
        draft::i18n("Un résumé"),
        Some(serde_json::json!({ "fr": "Un résumé" }))
    );
}

#[test]
fn les_publics_vises_perdent_leurs_entrees_vides() {
    // L'exigence de français porte élément par élément : une ligne blanche
    // laissée par l'écran ferait échouer l'enregistrement entier.
    let entrees = vec![
        "Ministères".to_owned(),
        "  ".to_owned(),
        "Journalistes".to_owned(),
    ];
    let rendues = draft::i18n_liste(&entrees);
    assert_eq!(rendues.len(), 2);
    assert_eq!(draft::fr(&rendues[1]), "Journalistes");
}

#[test]
fn lheure_murale_se_decoupe_sans_calendrier() {
    assert_eq!(
        draft::heure_murale("2027-11-12T14:30"),
        Some(("2027-11-12", "14:30"))
    );
    // Le front peut envoyer les secondes ; on n'en garde pas.
    assert_eq!(
        draft::heure_murale("2027-11-12T14:30:00"),
        Some(("2027-11-12", "14:30"))
    );
    assert_eq!(draft::heure_murale("2027-11-12"), None);
    assert_eq!(draft::heure_murale("12/11/2027T14:30"), None);
}

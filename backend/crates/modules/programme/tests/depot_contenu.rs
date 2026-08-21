//! **Ce que le dossier PORTE** — son texte, sa classification, ses intervenants
//! et ses co-organisations.
//!
//! Séparé de `depot.rs`, qui éprouve le **parcours** : l'enregistrement, l'état,
//! la recevabilité et les bornes de l'appel. Les deux fichiers partagent la même
//! fabrique et les mêmes gestes ; ce qui les distingue est la question posée —
//! *le dossier avance-t-il ?* d'un côté, *ce qui est écrit est-il juste ?* de
//! l'autre.
//!
//! Le découpage n'est pas un confort : le garde-fou de mille lignes de
//! `CLAUDE.md` vaut pour les tests comme pour le reste, et un fichier de test
//! trop long est aussi pénible à relire qu'un service trop long.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::{draft_write, submit};

/// Un dossier prêt à être déposé : deux intervenants, dans les bornes de
/// l'appel par défaut (1 à 10).
fn complet(terrain: &Terrain, titre: &str) -> programme::domain::draft::ProposalDraft {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.speakers = vec![
        commun::intervenant("awa.sow@example.org", "Awa", "Sow"),
        commun::intervenant("karim.ilboudo@example.org", "Karim", "Ilboudo"),
    ];
    brouillon
}

// -----------------------------------------------------------------------------
// T057 — ce qui est stocké est propre (écart n° 32)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_html_stocke_est_assaini_a_lecriture() {
    // Assainir à l'AFFICHAGE obligerait à le refaire dans chaque écran, chaque
    // courriel et chaque export — et le premier oubli serait une injection.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut piege = complet(&terrain, "Présentation piégée");
    piege.detailed_presentation = concat!(
        "<p onclick=\"voler()\">Un <strong>atelier</strong>.</p>",
        "<script>alert(1)</script>",
        "<p><a href=\"javascript:alert(2)\">Cliquer</a></p>",
        "<p style=\"color:#ff0000\">Rouge</p>",
    )
    .to_owned();

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, piege),
    )
    .await
    .expect("création");

    let stocke = commun::ligne(&bac, cree.proposal_id).await.presentation_fr;
    assert!(!stocke.contains("onclick"), "{stocke}");
    assert!(!stocke.contains("script"), "{stocke}");
    assert!(!stocke.contains("javascript"), "{stocke}");
    assert!(!stocke.contains("style"), "{stocke}");
    // Le texte survit : supprimer le contenu ferait perdre un paragraphe entier
    // pour un attribut de trop.
    assert!(stocke.contains("<strong>atelier</strong>"), "{stocke}");
    assert!(stocke.contains("Cliquer"), "{stocke}");
    assert!(stocke.contains("Rouge"), "{stocke}");
}

// -----------------------------------------------------------------------------
// T058 — un texte au-delà de sa borne (écart n° 28)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_texte_trop_long_est_refuse_en_nommant_son_champ_et_sa_limite() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut bavard = complet(&terrain, "Titre correct");
    bavard.summary = "é".repeat(401);

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, bavard),
    )
    .await
    .expect_err("quatre cent un caractères dépassent la borne du résumé");

    assert_eq!(refus.code, ErrorCode::ProposalTextTooLong);
    assert_eq!(refus.field.as_deref(), Some("summary"));
    assert!(refus.message.contains("400"), "{}", refus.message);

    // Et quatre cents caractères ACCENTUÉS passent : la borne se compte en
    // caractères, pas en octets — les compter en octets reviendrait à refuser
    // le français.
    let mut juste = complet(&terrain, "Titre correct");
    juste.summary = "é".repeat(400);
    draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, juste),
    )
    .await
    .expect("quatre cents caractères accentués tiennent dans la borne");
}

// -----------------------------------------------------------------------------
// T059 — l'heure murale, aller-retour (R6)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_creneau_saisi_a_14h30_se_relit_a_14h30() {
    // Belém est à trois heures derrière l'UTC. Un créneau saisi à 14:30 qui se
    // rouvrirait à 11:30 pour qui corrige depuis Dakar ne serait signalé par
    // rien : c'est le défaut le plus discret du module.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut avec_creneau = complet(&terrain, "Créneau souhaité");
    avec_creneau.preferred_start_at = Some("2027-11-12T14:30".to_owned());
    avec_creneau.duration_minutes = Some(60);

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, avec_creneau),
    )
    .await
    .expect("création");

    let relu = commun::creneau_mural(&bac, cree.proposal_id, commun::FUSEAU_COP31).await;
    assert_eq!(relu.as_deref(), Some("2027-11-12T14:30"));

    // Et en UTC, il est bien à 17:30 : c'est la preuve que la conversion a eu
    // lieu, et qu'on n'a pas simplement stocké la chaîne telle quelle.
    let en_utc = commun::creneau_mural(&bac, cree.proposal_id, "UTC").await;
    assert_eq!(en_utc.as_deref(), Some("2027-11-12T17:30"));

    assert_eq!(
        commun::ligne(&bac, cree.proposal_id).await.duration_minutes,
        Some(60)
    );
}

// -----------------------------------------------------------------------------
// T060 — les thématiques, leur triplet et les codes inconnus (écarts n° 3, 94)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_triplet_dentite_est_ecrit_par_le_service_et_jamais_recu() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut classe = complet(&terrain, "Dossier classé");
    classe.theme_codes = vec!["adaptation".to_owned(), "climate_finance".to_owned()];

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, classe),
    )
    .await
    .expect("création");

    let posees = commun::thematiques(&bac, cree.proposal_id).await;
    assert_eq!(posees.len(), 2);
    // **Le triplet est écrit littéralement.** Le contrat ne le porte pas, et un
    // client qui l'enverrait ne pourrait pas rattacher une thématique à
    // n'importe quelle entité de n'importe quel schéma (écart n° 3).
    for (schema, table, _) in &posees {
        assert_eq!(schema, "programme");
        assert_eq!(table, "proposals");
    }
    assert_eq!(posees[0].2, "adaptation");
}

#[tokio::test]
async fn un_code_de_thematique_inconnu_est_refuse_en_le_nommant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut inconnu = complet(&terrain, "Thématique périmée");
    inconnu.theme_codes = vec!["adaptation".to_owned(), "hydrogene_vert".to_owned()];

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, inconnu),
    )
    .await
    .expect_err("un code hors taxonomie ne s'accepte pas en silence");

    assert_eq!(refus.code, ErrorCode::ProposalUnknownTerm);
    assert_eq!(refus.field.as_deref(), Some("theme_codes"));
    // Nommer le code refusé : l'écran doit pouvoir retirer LA BONNE pastille.
    assert!(
        refus.message.contains("hydrogene_vert"),
        "{}",
        refus.message
    );
}

#[tokio::test]
async fn les_thematiques_retirees_disparaissent() {
    // Aucune contrainte référentielle ne les purge, et la fonction de nettoyage
    // que le modèle annonce n'existe pas (écart n° 94).
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut deux = complet(&terrain, "Deux thématiques");
    deux.theme_codes = vec!["adaptation".to_owned(), "gender".to_owned()];
    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, deux),
    )
    .await
    .expect("création");

    let mut une = complet(&terrain, "Deux thématiques");
    une.theme_codes = vec!["gender".to_owned()];
    let mut charge = commun::charge(&terrain, une);
    charge.proposal_id = Some(cree.proposal_id);
    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect("mise à jour");

    let restantes = commun::thematiques(&bac, cree.proposal_id).await;
    assert_eq!(restantes.len(), 1);
    assert_eq!(restantes[0].2, "gender");
}

// -----------------------------------------------------------------------------
// T061 — les intervenants (écarts n° 26 et n° 31)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_intervenant_inconnu_cree_la_personne_avec_le_nom_saisi() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut avec = complet(&terrain, "Avec un invité");
    avec.speakers = vec![commun::intervenant(
        "a.diallo@example.org",
        "Aminata",
        "Diallo",
    )];

    draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, avec),
    )
    .await
    .expect("création");

    let creee = commun::fiche(&bac, "a.diallo@example.org")
        .await
        .expect("la personne inconnue a été créée");
    // **Ni prénom ni nom ne sont déduits de l'adresse** : un « a.diallo »
    // extrait d'un courriel est un nom que plus personne ne corrigera, et il
    // s'afficherait sur toutes ses participations futures (FR-026).
    assert_eq!(creee.1, "Aminata");
    assert_eq!(creee.2, "Diallo");

    // Ni compte, ni rôle : l'écriture est bornée à la fiche.
    let comptes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.accounts WHERE person_id = $1"#,
        creee.0
    )
    .fetch_one(bac.pool())
    .await
    .expect("décompte des comptes");
    assert_eq!(comptes, 0);
}

#[tokio::test]
async fn lidentite_dune_personne_avec_compte_est_verrouillee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let titulaire = commun::personne(&bac, "awa.sowfall@example.org", "Awa", "Sow Fall").await;
    commun::donner_un_compte(&bac, titulaire).await;

    let mut renomme = complet(&terrain, "Renommage abusif");
    renomme.speakers = vec![commun::intervenant(
        "awa.sowfall@example.org",
        "A.",
        "Sowfall",
    )];

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, renomme),
    )
    .await
    .expect_err("un déposant ne réécrit pas la fiche de quelqu'un d'autre");

    assert_eq!(refus.code, ErrorCode::ProposalSpeakerIdentityLocked);
    assert_eq!(refus.field.as_deref(), Some("speakers"));
    // Le refus NOMME la personne : l'écran reverrouille le champ et dit de qui
    // il s'agit.
    assert!(refus.message.contains("Awa"), "{}", refus.message);

    // La fiche n'a pas bougé.
    let apres = commun::fiche(&bac, "awa.sowfall@example.org")
        .await
        .unwrap();
    assert_eq!(apres.1, "Awa");
    assert_eq!(apres.2, "Sow Fall");
}

#[tokio::test]
async fn les_instantanes_restent_modifiables_et_lidentite_sans_compte_aussi() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let connue = commun::personne(&bac, "moussa.ba@example.org", "Moussa", "Ba").await;

    let mut corrige = complet(&terrain, "Correction d'un nom");
    let mut intervenant = commun::intervenant("moussa.ba@example.org", "Moussa", "Bâ");
    intervenant.job_title = "Directeur général".to_owned();
    corrige.speakers = vec![intervenant];

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, corrige),
    )
    .await
    .expect("une personne sans compte reste corrigeable");

    // La correction est ÉCRITE : un enregistrement qui réussit sans rien
    // changer serait pire qu'un refus.
    let apres = commun::fiche(&bac, "moussa.ba@example.org").await.unwrap();
    assert_eq!(apres.0, connue);
    assert_eq!(apres.2, "Bâ");

    // Et l'instantané de fonction vit sur la LIGNE D'INTERVENANT, pas sur la
    // fiche : « une personne change d'employeur, l'archive de la COP28 ne doit
    // pas être réécrite pour autant ».
    let fonction = sqlx::query_scalar!(
        "SELECT job_title_snapshot FROM programme.proposal_speakers
          WHERE proposal_id = $1 AND person_id = $2",
        cree.proposal_id,
        connue
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'instantané");
    assert_eq!(fonction.as_deref(), Some("Directeur général"));

    let sur_la_fiche = sqlx::query_scalar!(
        "SELECT job_title FROM identity.people WHERE id = $1",
        connue
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la fiche");
    assert_eq!(sur_la_fiche, None, "la fiche ne reçoit pas l'instantané");
}

// -----------------------------------------------------------------------------
// T062 — les co-organisations et leur annonce
// -----------------------------------------------------------------------------

#[tokio::test]
async fn chaque_coorganisation_ajoutee_est_annoncee_une_fois() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let partenaire = commun::organisation_verifiee(&bac, "Agence régionale", "ARE").await;
    let soutien = commun::organisation_verifiee(&bac, "Ministère de l'Environnement", "MEV").await;

    let mut avec = complet(&terrain, "Dossier co-organisé");
    avec.co_organizations = vec![
        commun::coorganisation(partenaire, "co_organizer"),
        commun::coorganisation(soutien, "sponsor"),
    ];

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, avec.clone()),
    )
    .await
    .expect("création");

    // La ligne du porteur est posée par DÉCLENCHEUR : le service ne l'écrit
    // jamais, et elle est là quand même.
    let associations = commun::associations(&bac, cree.proposal_id).await;
    assert_eq!(associations.len(), 3);
    assert_eq!(associations[0], (terrain.organisation, "lead".to_owned()));

    let emis = commun::evenements_emis(&bac, cree.proposal_id).await;
    let annonces = emis
        .iter()
        .filter(|t| *t == "programme.coorganization.requested")
        .count();
    assert_eq!(annonces, 2, "un événement PAR organisation ajoutée");

    // Un brouillon s'enregistre toutes les deux secondes : réannoncer à chaque
    // fois inviterait la même organisation cent fois.
    let mut encore = commun::charge(&terrain, avec);
    encore.proposal_id = Some(cree.proposal_id);
    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, encore)
        .await
        .expect("second enregistrement");

    let emis = commun::evenements_emis(&bac, cree.proposal_id).await;
    let annonces = emis
        .iter()
        .filter(|t| *t == "programme.coorganization.requested")
        .count();
    assert_eq!(
        annonces, 2,
        "aucune annonce de plus au second enregistrement"
    );
}

#[tokio::test]
async fn le_porteur_ne_peut_pas_etre_son_propre_coorganisateur() {
    // Le `ON CONFLICT` du déclencheur de synchronisation ferait basculer la
    // ligne en `lead` au prochain enregistrement, EN SILENCE, et le dossier
    // perdrait un co-organisateur sans qu'aucune erreur ne le dise.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut lui_meme = complet(&terrain, "Porteur en double");
    lui_meme.co_organizations = vec![commun::coorganisation(terrain.organisation, "co_organizer")];

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, lui_meme),
    )
    .await
    .expect_err("l'organisation porteuse ne se co-organise pas elle-même");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("co_organizations"));
}

// -----------------------------------------------------------------------------
// Ce que le dépôt refuse en plus, et que rien d'autre ne dirait
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_dossier_encore_provisoire_ne_part_pas_au_comite() {
    // Les trois textes obligatoires naissent avec un repli — `i18n_text` refuse
    // un français vide et les colonnes sont NOT NULL. Les laisser passer
    // enverrait au comité un dossier intitulé « Dossier sans titre ».
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut sans_titre = complet(&terrain, "");
    sans_titre.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, sans_titre.clone()),
    )
    .await
    .expect("le brouillon s'enregistre");

    let refus = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(&terrain, sans_titre),
    )
    .await
    .expect_err("un dossier sans titre ne se dépose pas");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("title"));
}

#[tokio::test]
async fn une_adhesion_en_attente_ne_permet_pas_de_deposer() {
    // `pending` n'est pas `active` : une demande d'adhésion non approuvée
    // donnerait à quiconque connaît le nom d'une organisation le droit d'écrire
    // en son nom.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let etrangere = commun::personne(&bac, "etrangere@example.org", "Fatou", "Ndiaye").await;
    commun::adherer(&bac, terrain.organisation, etrangere, "pending").await;

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        etrangere,
        commun::charge(&terrain, complet(&terrain, "Au nom d'une autre")),
    )
    .await
    .expect_err("une adhésion en attente n'autorise rien");

    // **Un `NOT_FOUND`, pas un `FORBIDDEN`** : un dossier d'une organisation
    // dont on n'est pas membre ne doit pas se distinguer d'un dossier
    // inexistant.
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn un_dossier_clos_nest_plus_modifiable() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "Dossier retiré")),
    )
    .await
    .expect("création");

    // Le retrait passe par la machine à états (phase 4) ; ici on fabrique
    // l'état terminal directement, `submitted_at` comprise —
    // `ck_proposals_submitted_at` exige qu'un dossier hors brouillon porte sa
    // date de dépôt, et c'est le déclencheur qui la pose d'ordinaire.
    sqlx::query!(
        "UPDATE programme.proposals SET status = 'withdrawn', submitted_at = now()
          WHERE id = $1",
        cree.proposal_id
    )
    .execute(bac.pool())
    .await
    .expect("retrait direct en base");

    let mut charge = commun::charge(&terrain, complet(&terrain, "Dossier retiré"));
    charge.proposal_id = Some(cree.proposal_id);

    let refus = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect_err("un dossier retiré ne se corrige plus");

    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
}

// -----------------------------------------------------------------------------
// Le contexte du formulaire
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_contexte_du_formulaire_exclut_le_brouillon_en_cours() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let contexte = programme::repo::proposals::contexte_du_formulaire(
        bac.pool(),
        &[terrain.organisation],
        None,
    )
    .await
    .expect("contexte");

    assert_eq!(contexte.call_id, Some(terrain.appel));
    assert_eq!(contexte.event_id, Some(terrain.edition));
    assert_eq!(contexte.counted_proposals, 0);

    // Un brouillon ne compte pas dans le plafond — c'est l'un des trois états
    // que le déclencheur écarte.
    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "Un brouillon")),
    )
    .await
    .expect("création");

    let contexte = programme::repo::proposals::contexte_du_formulaire(
        bac.pool(),
        &[terrain.organisation],
        Some(ProposalId(cree.proposal_id)),
    )
    .await
    .expect("contexte");
    assert_eq!(contexte.counted_proposals, 0);

    // Une fois déposé, il compte.
    submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(&terrain, complet(&terrain, "Un brouillon")),
    )
    .await
    .expect("dépôt");

    let contexte = programme::repo::proposals::contexte_du_formulaire(
        bac.pool(),
        &[terrain.organisation],
        None,
    )
    .await
    .expect("contexte");
    assert_eq!(contexte.counted_proposals, 1);
}

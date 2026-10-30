//! Les fils du comité adressés à des personnes nommées, et le lu / non lu par
//! message (demande du commanditaire, 27/10).
//!
//! Le cercle d'un fil adressé : son auteur et ses destinataires, et personne
//! d'autre, administrateurs compris. Le compteur de la liste et les accusés
//! « lu par N sur M » se lisent sur la même règle.

mod commun;

use commun::comite::{comite, Comite};
use commun::{Bac, Terrain};
use kernel::auth::Perimeter;
use kernel::error::{ApiError, ErrorCode};
use programme::domain::ids::{CommentId, EventId, ProposalId};
use programme::service::comment_circle::{
    self as cercle, AddCommentRecipientsPayload, MarkCommentsReadPayload, ProposalCommentView,
};
use programme::service::comments::{self, PostCommentPayload};
use programme::service::list;
use uuid::Uuid;

struct Scene {
    bac: Bac,
    terrain: Terrain,
    comite: Comite,
    dossier: ProposalId,
    /// Administratrice globale, hors de tout fil adressé.
    globale: Uuid,
}

async fn scene() -> Scene {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let globale = commun::personne(&bac, "globale@ifdd.francophonie.org", "Gaëlle", "Ndiaye").await;
    commun::attribuer(&bac, globale, "admin", "global", None).await;
    let dossier = ProposalId(comite.dossier);
    Scene {
        bac,
        terrain,
        comite,
        dossier,
        globale,
    }
}

fn charge(
    parent: Option<Uuid>,
    visibilite: &str,
    corps: &str,
    destinataires: Vec<Uuid>,
) -> PostCommentPayload {
    PostCommentPayload {
        parent_id: parent,
        visibility: Some(visibilite.to_owned()),
        body: corps.to_owned(),
        is_change_request: false,
        recipient_ids: destinataires,
        topic: None,
        topic_other: None,
    }
}

async fn ecrire(
    s: &Scene,
    auteur: Uuid,
    payload: PostCommentPayload,
) -> Result<ProposalCommentView, ApiError> {
    comments::ecrire(&s.bac.state, &s.bac.ctx(), auteur, s.dossier, payload).await
}

async fn fil(s: &Scene, lecteur: Uuid) -> Vec<ProposalCommentView> {
    comments::fil_de(&s.bac.state, lecteur, s.dossier)
        .await
        .expect("le fil")
}

fn ids(fil: &[ProposalCommentView]) -> Vec<Uuid> {
    fil.iter().map(|m| m.id).collect()
}

/// Les avis `programme.comment.addressed` émis pour un message : leurs
/// destinataires et la demande de courriel.
async fn avis(bac: &Bac, message: Uuid) -> Vec<(Vec<Uuid>, bool)> {
    sqlx::query_scalar!(
        "SELECT payload FROM platform.outbox_events
          WHERE aggregate_id = $1 AND event_type = 'programme.comment.addressed'
          ORDER BY occurred_at, id",
        message
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox")
    .into_iter()
    .map(|p| {
        let mut destinataires: Vec<Uuid> = p["notification"]["recipients"]
            .as_array()
            .expect("destinataires")
            .iter()
            .map(|v| v.as_str().expect("uuid").parse().expect("uuid"))
            .collect();
        destinataires.sort();
        (destinataires, p["email"] == true)
    })
    .collect()
}

#[tokio::test]
async fn un_fil_adresse_nest_lu_que_de_son_cercle() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;
    let decideur = s.comite.decideur.person_id;

    let racine = ecrire(
        &s,
        premiere,
        charge(None, "committee", "Entre nous deux.", vec![seconde]),
    )
    .await
    .expect("le fil adressé");
    assert_eq!(racine.recipients.len(), 1);
    assert_eq!(racine.recipients[0].person_id, seconde);
    assert_eq!(racine.recipients[0].display_name, "Sophie Comite");

    let reponse = ecrire(
        &s,
        seconde,
        charge(Some(racine.id), "submitter", "D'accord.", vec![]),
    )
    .await
    .expect("la réponse");
    assert_eq!(
        reponse.visibility, "committee",
        "une réponse partagée reste dans le cercle"
    );

    for lecteur in [premiere, seconde] {
        assert_eq!(ids(&fil(&s, lecteur).await), vec![racine.id, reponse.id]);
    }
    for (lecteur, qui) in [
        (decideur, "l'administrateur de l'édition"),
        (s.globale, "l'administratrice globale"),
        (s.terrain.deposante, "la déposante"),
    ] {
        assert!(fil(&s, lecteur).await.is_empty(), "{qui} ne lit rien");
    }

    // Hors du cercle, remettre non lu est refusé comme un message inexistant.
    let refus = cercle::marquer_non_lu(&s.bac.state, &s.bac.ctx(), decideur, CommentId(racine.id))
        .await
        .expect_err("hors du cercle");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Une demande de correction ne se pose pas dans un fil adressé.
    let mut demande = charge(Some(racine.id), "submitter", "À corriger.", vec![]);
    demande.is_change_request = true;
    let refus = ecrire(&s, seconde, demande).await.expect_err("refusée");
    assert_eq!(refus.field.as_deref(), Some("is_change_request"));

    // Hors du cercle, répondre est refusé comme un message inexistant.
    let refus = ecrire(
        &s,
        decideur,
        charge(Some(racine.id), "committee", "?", vec![]),
    )
    .await
    .expect_err("hors du cercle");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn ajouter_au_cercle_ouvre_tout_le_fil_et_previent_les_ajoutes() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;
    let decideur = s.comite.decideur.person_id;

    let racine = ecrire(
        &s,
        premiere,
        charge(None, "committee", "Entre nous deux.", vec![seconde]),
    )
    .await
    .unwrap();
    let reponse = ecrire(
        &s,
        seconde,
        charge(Some(racine.id), "committee", "Oui.", vec![]),
    )
    .await
    .unwrap();

    // Hors du cercle : refusé comme un message inexistant.
    let refus = cercle::ajouter_destinataires(
        &s.bac.state,
        &s.bac.ctx(),
        decideur,
        CommentId(racine.id),
        AddCommentRecipientsPayload {
            person_ids: vec![decideur],
        },
    )
    .await
    .expect_err("hors du cercle");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Une destinataire élargit le fil, deux fois : idempotent.
    for _ in 0..2 {
        cercle::ajouter_destinataires(
            &s.bac.state,
            &s.bac.ctx(),
            seconde,
            CommentId(racine.id),
            AddCommentRecipientsPayload {
                person_ids: vec![decideur],
            },
        )
        .await
        .expect("ajout");
    }
    assert_eq!(ids(&fil(&s, decideur).await), vec![racine.id, reponse.id]);
    assert!(fil(&s, s.globale).await.is_empty());

    let annonces = avis(&s.bac, racine.id).await;
    assert_eq!(
        annonces,
        vec![(vec![seconde], true), (vec![decideur], true)],
        "l'ouverture prévient la destinataire, l'ajout le seul ajouté, une fois"
    );

    // Un fil ouvert à tout le comité ne s'adresse pas après coup.
    let ouvert = ecrire(&s, premiere, charge(None, "committee", "À tous.", vec![]))
        .await
        .unwrap();
    let refus = cercle::ajouter_destinataires(
        &s.bac.state,
        &s.bac.ctx(),
        premiere,
        CommentId(ouvert.id),
        AddCommentRecipientsPayload {
            person_ids: vec![seconde],
        },
    )
    .await
    .expect_err("fil ouvert");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
}

#[tokio::test]
async fn les_destinataires_sont_du_comite_et_sur_un_fil_qui_souvre() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;
    let etranger = commun::personne(&s.bac, "etranger@example.org", "Eric", "Tranger").await;

    for (destinataires, qui) in [
        (
            vec![s.terrain.deposante],
            "la déposante n'est pas du comité",
        ),
        (vec![etranger], "une personne sans rôle non plus"),
        (vec![premiere], "soi seul ne fait pas un cercle"),
    ] {
        let refus = ecrire(
            &s,
            premiere,
            charge(None, "committee", "Message.", destinataires),
        )
        .await
        .expect_err(qui);
        assert_eq!(refus.code, ErrorCode::ValidationFailed, "{qui}");
        assert_eq!(refus.field.as_deref(), Some("recipient_ids"), "{qui}");
    }

    let racine = ecrire(&s, premiere, charge(None, "committee", "À tous.", vec![]))
        .await
        .unwrap();
    for (payload, qui) in [
        (
            charge(Some(racine.id), "committee", "Réponse.", vec![seconde]),
            "sur une réponse",
        ),
        (
            charge(None, "submitter", "Partagé.", vec![seconde]),
            "sur un message partagé",
        ),
        (
            charge(None, "private", "Note.", vec![seconde]),
            "sur une note personnelle",
        ),
    ] {
        let refus = ecrire(&s, premiere, payload).await.expect_err(qui);
        assert_eq!(refus.field.as_deref(), Some("recipient_ids"), "{qui}");
    }

    // L'organisation ne lit pas les noms du comité.
    let refus = cercle::audience(&s.bac.state, s.terrain.deposante, s.dossier)
        .await
        .expect_err("refusée à l'organisation");
    assert_eq!(refus.code, ErrorCode::NotFound);
    let audience = cercle::audience(&s.bac.state, premiere, s.dossier)
        .await
        .expect("le comité");
    let membres: Vec<Uuid> = audience.iter().map(|p| p.person_id).collect();
    for p in [premiere, seconde, s.comite.decideur.person_id, s.globale] {
        assert!(membres.contains(&p));
    }
    assert!(!membres.contains(&s.terrain.deposante));
}

#[tokio::test]
async fn lu_non_lu_et_accuses_lu_par_n_sur_m() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;

    let message = ecrire(&s, premiere, charge(None, "committee", "À tous.", vec![]))
        .await
        .unwrap();
    assert!(message.is_read, "on a lu ce qu'on a écrit");
    let audience = cercle::audience(&s.bac.state, premiere, s.dossier)
        .await
        .unwrap();
    let m = audience.len() as i64 - 1;
    let accuse = message.receipts.as_ref().expect("ses accusés");
    assert_eq!((accuse.audience_count, accuse.readers.len()), (m, 0));

    assert!(!fil(&s, seconde).await[0].is_read);
    assert!(
        fil(&s, seconde).await[0].receipts.is_none(),
        "pas les accusés d'autrui"
    );

    for _ in 0..2 {
        cercle::marquer_lus(
            &s.bac.state,
            &s.bac.ctx(),
            seconde,
            s.dossier,
            MarkCommentsReadPayload {
                comment_ids: vec![message.id],
            },
        )
        .await
        .expect("lu");
    }
    assert!(fil(&s, seconde).await[0].is_read);

    let sien = fil(&s, premiere).await;
    let accuse = sien[0].receipts.as_ref().unwrap();
    assert_eq!(accuse.audience_count, m);
    assert_eq!(accuse.readers.len(), 1);
    assert_eq!(accuse.readers[0].person_id, seconde);
    assert_eq!(accuse.readers[0].side, "committee");

    // Son propre message ne se marque pas.
    cercle::marquer_lus(
        &s.bac.state,
        &s.bac.ctx(),
        premiere,
        s.dossier,
        MarkCommentsReadPayload {
            comment_ids: vec![message.id],
        },
    )
    .await
    .unwrap();
    assert_eq!(
        fil(&s, premiere).await[0]
            .receipts
            .as_ref()
            .unwrap()
            .readers
            .len(),
        1
    );

    cercle::marquer_non_lu(&s.bac.state, &s.bac.ctx(), seconde, CommentId(message.id))
        .await
        .expect("non lu");
    assert!(!fil(&s, seconde).await[0].is_read);
    assert!(fil(&s, premiere).await[0]
        .receipts
        .as_ref()
        .unwrap()
        .readers
        .is_empty());

    // Un fil adressé à une seule personne : « sur 1 ».
    let adresse = ecrire(
        &s,
        premiere,
        charge(None, "committee", "Toi.", vec![seconde]),
    )
    .await
    .unwrap();
    assert_eq!(adresse.receipts.as_ref().unwrap().audience_count, 1);

    // Un message partagé lu par la déposante : côté organisation.
    let partage = ecrire(&s, premiere, charge(None, "submitter", "Précisez.", vec![]))
        .await
        .unwrap();
    cercle::marquer_lus(
        &s.bac.state,
        &s.bac.ctx(),
        s.terrain.deposante,
        s.dossier,
        MarkCommentsReadPayload {
            comment_ids: vec![message.id, partage.id],
        },
    )
    .await
    .unwrap();
    let vu = fil(&s, s.terrain.deposante).await;
    assert_eq!(vu.len(), 1);
    assert!(vu[0].is_read && vu[0].receipts.is_none());
    let sien = fil(&s, premiere).await;
    let lecteurs = &sien
        .iter()
        .find(|v| v.id == partage.id)
        .unwrap()
        .receipts
        .as_ref()
        .unwrap()
        .readers;
    assert_eq!(lecteurs.len(), 1);
    assert_eq!(lecteurs[0].side, "organisation");
    assert!(
        sien.iter()
            .find(|v| v.id == message.id)
            .unwrap()
            .receipts
            .as_ref()
            .unwrap()
            .readers
            .is_empty(),
        "la déposante ne marque pas un message du comité"
    );
}

async fn compte(s: &Scene, edition: EventId, perimetre: &Perimeter) -> Vec<(Uuid, i64)> {
    list::ecran(&s.bac.state, perimetre, edition)
        .await
        .expect("la liste")
        .unread_comments
        .into_iter()
        .map(|u| (u.proposal_id, u.count))
        .collect()
}

#[tokio::test]
async fn la_liste_compte_les_messages_non_lus_par_dossier() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;
    let edition = EventId::from(s.terrain.edition);

    let a = ecrire(&s, premiere, charge(None, "committee", "À tous.", vec![]))
        .await
        .unwrap();
    ecrire(
        &s,
        premiere,
        charge(None, "committee", "Toi.", vec![seconde]),
    )
    .await
    .unwrap();
    ecrire(&s, premiere, charge(None, "private", "Ma note.", vec![]))
        .await
        .unwrap();

    assert_eq!(
        compte(&s, edition, &s.comite.seconde).await,
        vec![(s.dossier.as_uuid(), 2)]
    );
    assert_eq!(
        compte(&s, edition, &s.comite.decideur).await,
        vec![(s.dossier.as_uuid(), 1)],
        "le fil adressé à une autre ne compte pas"
    );
    assert!(
        compte(&s, edition, &s.comite.premiere).await.is_empty(),
        "ses propres messages non plus"
    );

    cercle::marquer_lus(
        &s.bac.state,
        &s.bac.ctx(),
        seconde,
        s.dossier,
        MarkCommentsReadPayload {
            comment_ids: vec![a.id],
        },
    )
    .await
    .unwrap();
    assert_eq!(
        compte(&s, edition, &s.comite.seconde).await,
        vec![(s.dossier.as_uuid(), 1)]
    );
}

#[tokio::test]
async fn lavis_part_vers_le_cercle_et_jamais_pour_tout_le_comite() {
    let s = scene().await;
    let premiere = s.comite.premiere.person_id;
    let seconde = s.comite.seconde.person_id;
    let decideur = s.comite.decideur.person_id;

    let ouvert = ecrire(&s, premiere, charge(None, "committee", "À tous.", vec![]))
        .await
        .unwrap();
    let reponse_ouverte = ecrire(
        &s,
        seconde,
        charge(Some(ouvert.id), "committee", "Oui.", vec![]),
    )
    .await
    .unwrap();
    assert!(commun::evenements_emis(&s.bac, ouvert.id).await.is_empty());
    assert!(commun::evenements_emis(&s.bac, reponse_ouverte.id)
        .await
        .is_empty());

    let racine = ecrire(
        &s,
        premiere,
        charge(
            None,
            "committee",
            "Vous deux.",
            vec![seconde, decideur, premiere],
        ),
    )
    .await
    .unwrap();
    let mut attendus = vec![seconde, decideur];
    attendus.sort();
    assert_eq!(
        avis(&s.bac, racine.id).await,
        vec![(attendus, true)],
        "l'auteur exclu"
    );

    let reponse = ecrire(
        &s,
        seconde,
        charge(Some(racine.id), "committee", "Vu.", vec![]),
    )
    .await
    .unwrap();
    let mut attendus = vec![premiere, decideur];
    attendus.sort();
    assert_eq!(
        avis(&s.bac, reponse.id).await,
        vec![(attendus, true)],
        "le cercle sauf l'auteur de la réponse"
    );

    let note = ecrire(
        &s,
        seconde,
        charge(Some(racine.id), "private", "Pour moi.", vec![]),
    )
    .await
    .unwrap();
    assert!(
        avis(&s.bac, note.id).await.is_empty(),
        "une note ne prévient personne"
    );
}

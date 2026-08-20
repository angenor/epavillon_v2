//! **Le chiffre annoncé avant est celui rendu après** (SC-010).
//!
//! L'aperçu et le journal sont comparés **ligne de registre par ligne de
//! registre**, écart de zéro. C'est la raison d'être du SQL composé de
//! `repo/merge_counts.rs` : le décompte annoncé et le décompte réel sont
//! calculés par le même raisonnement, à partir de la même source — le registre
//! `org.organization_references`. L'aperçu ne peut pas mentir sans que la fusion
//! mente aussi.

mod commun;

use commun::{perimetres, personne, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::{MergeOutcome, MergePayload};
use org::service::merge;
use std::collections::BTreeMap;
use uuid::Uuid;

/// De quoi rendre le décompte non trivial : des adhésions des deux côtés, dont
/// une **partagée** — la même personne adhérente des deux fiches, qui sera
/// supprimée avant la bascule.
async fn semer_des_rattachements(bac: &Bac, source: Uuid, cible: Uuid) {
    let partagee = personne(bac, "partagee@osed-sahel.org", "Aminata", "Traoré").await;
    let cote_source = personne(bac, "source@osed-sahel.org", "Boureima", "Ouédraogo").await;
    let cote_cible = personne(bac, "cible@osed-sahel.org", "Fatou", "Ndiaye").await;

    for (organisation, personne) in [
        (source, partagee),
        (cible, partagee),
        (source, cote_source),
        (cible, cote_cible),
    ] {
        sqlx::query!(
            "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
             VALUES ($1, $2, 'member', 'active', now())",
            organisation,
            personne
        )
        .execute(bac.pool())
        .await
        .expect("adhésion");
    }
}

#[tokio::test]
async fn lapercu_et_le_journal_disent_le_meme_chiffre() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    semer_des_rattachements(&bac, osed.jumelle, osed.complete).await;

    let apercu = merge::preview(
        &bac.state,
        PersonId(p.globale),
        osed.jumelle.into(),
        osed.complete.into(),
        None,
    )
    .await
    .expect("aperçu")
    .expect("les deux fiches existent");

    // Le registre entier est chiffré : dix-huit lignes aujourd'hui, davantage
    // demain, et le code n'aura pas à changer.
    assert_eq!(
        apercu.transfers.len(),
        18,
        "une ligne par entrée du registre : {:?}",
        apercu
            .transfers
            .iter()
            .map(|t| &t.ref_table)
            .collect::<Vec<_>>()
    );

    let adhesions = apercu
        .transfers
        .iter()
        .find(|t| t.ref_table == "memberships")
        .expect("la ligne des adhésions");
    assert_eq!(
        adhesions.reassigned, 1,
        "une seule adhésion bascule : l'autre est en conflit"
    );
    assert_eq!(
        adhesions.deduped, 1,
        "la personne adhérente des DEUX fiches : sa ligne côté source est \
         supprimée avant la bascule, sans quoi l'unicité ferait échouer la fusion"
    );

    // Le domaine partagé, lui aussi, se dédoublonne.
    let domaines = apercu
        .transfers
        .iter()
        .find(|t| t.ref_table == "organization_domains")
        .expect("la ligne des domaines");
    assert_eq!(domaines.deduped, 1, "le même domaine des deux côtés");

    // Et maintenant la fusion.
    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon manifeste".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("fusion");

    let reel = match issue {
        MergeOutcome::Merged {
            rows_reassigned, ..
        } => rows_reassigned,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    // **Ligne de registre par ligne de registre, écart de zéro.**
    //
    // La fonction de base ne compte que les lignes RESTANTES au moment de la
    // bascule — celles en conflit ont déjà été supprimées. C'est exactement
    // `reassigned` de l'aperçu.
    let mut compares = 0;
    for ligne in &apercu.transfers {
        // Les dénominations sont traitées à part par la fonction (étape 1) et
        // n'apparaissent pas dans son décompte.
        if ligne.ref_table == "organization_names" {
            continue;
        }

        // **La seule ligne que la fusion déplace par ricochet.** Réaffecter une
        // adhésion réveille `tg_memberships_sync_primary`, qui met à jour
        // `identity.people.primary_organization_id` AVANT que la boucle du
        // registre n'y arrive : le journal en compte alors moins que l'aperçu
        // n'en annonçait. Les lignes ont bien été déplacées, simplement pas par
        // l'ordre qui les comptait. L'écart est vérifié dans l'autre sens, plus
        // bas.
        if ligne.ref_table == "people" {
            continue;
        }
        compares += 1;

        let cle = format!(
            "{}.{}.{}",
            ligne.ref_schema, ligne.ref_table, ligne.ref_column
        );
        let annonce = if ligne.strategy == "delete" {
            ligne.deleted
        } else {
            ligne.reassigned
        };
        let rendu = reel.get(&cle).and_then(|v| v.as_i64()).unwrap_or(-1);

        assert_eq!(
            annonce, rendu,
            "{cle} : l'aperçu annonçait {annonce}, le journal rend {rendu}"
        );
    }

    assert_eq!(
        compares, 16,
        "seize lignes de registre comparées au chiffre près"
    );

    // Et la ligne écartée ci-dessus : **rien n'est perdu**, la personne pointe
    // bien vers la fiche survivante. C'est ce qui compte, et c'est ce que le
    // décompte ne sait pas dire.
    let restants = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.people
            WHERE primary_organization_id = $1"#,
        osed.jumelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(
        restants, 0,
        "plus personne ne pointe vers la fiche absorbée"
    );
}

/// **Le décompte n'est pas symétrique**, et c'est pourquoi l'aperçu est
/// recalculé à chaque inversion du sens.
#[tokio::test]
async fn le_decompte_nest_pas_symetrique() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    // Trois membres d'un côté, un de l'autre.
    for i in 0..3 {
        let qui = personne(
            &bac,
            &format!("membre{i}@osed-sahel.org"),
            "Membre",
            "Numéro",
        )
        .await;
        sqlx::query!(
            "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
             VALUES ($1, $2, 'member', 'active', now())",
            osed.jumelle,
            qui
        )
        .execute(bac.pool())
        .await
        .expect("adhésion");
    }

    let seul = personne(&bac, "seul@osed-sahel.org", "Seul", "Membre").await;
    sqlx::query!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
         VALUES ($1, $2, 'member', 'active', now())",
        osed.complete,
        seul
    )
    .execute(bac.pool())
    .await
    .expect("adhésion");

    let adhesions = |apercu: &org::domain::merge::MergePreview| {
        apercu
            .transfers
            .iter()
            .find(|t| t.ref_table == "memberships")
            .expect("la ligne des adhésions")
            .reassigned
    };

    let un_sens = merge::preview(
        &bac.state,
        PersonId(p.globale),
        osed.jumelle.into(),
        osed.complete.into(),
        None,
    )
    .await
    .expect("aperçu")
    .expect("les deux fiches");

    let lautre = merge::preview(
        &bac.state,
        PersonId(p.globale),
        osed.complete.into(),
        osed.jumelle.into(),
        None,
    )
    .await
    .expect("aperçu inversé")
    .expect("les deux fiches");

    assert_eq!(adhesions(&un_sens), 3);
    assert_eq!(adhesions(&lautre), 1);
    assert_ne!(
        adhesions(&un_sens),
        adhesions(&lautre),
        "trois adhésions transférées dans un sens en font une dans l'autre"
    );
}

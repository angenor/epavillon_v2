//! **Un relais qui compresse ne rend pas un choix périmé.** Apache ajoute
//! « -br » ou « -gzip » à l'`ETag`, nginx l'affaiblit en `W/` : l'empreinte
//! revient ainsi en `If-Match`, et elle désigne toujours le même état.
//!
//! Ni le développement ni la recette ne compressent : sans ce test, le défaut ne
//! paraîtrait que le jour où le relais institutionnel serait branché.

mod commun;

use commun::{codes_de, lignes_de_suivi, mes_thematiques, personne, suivre, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn une_empreinte_reecrite_par_un_relais_est_acceptee() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    // Le gabarit de ce que le relais fait de l'empreinte ; `{}` est l'empreinte nue.
    for (codes, gabarit) in [
        (&["adaptation"][..], "\"{}-br\""),
        (&["adaptation", "gender"][..], "\"{}-gzip\""),
        (&["finance"][..], "W/\"{}\""),
    ] {
        let courante = mes_thematiques(&bac, awa).await.empreinte();
        let presentee = gabarit.replace("{}", courante.trim_matches('"'));
        let rendu = suivre(&bac, awa, codes, Some(&presentee))
            .await
            .expect("même état, autre habillage");
        assert_eq!(codes_de(&rendu), codes);
    }
}

#[tokio::test]
async fn une_autre_empreinte_ou_une_valeur_illisible_rend_412_sans_rien_ecrire() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;
    suivre(&bac, awa, &["adaptation"], None)
        .await
        .expect("état initial");
    let avant = lignes_de_suivi(&bac, awa).await;

    let autre = negotiation::domain::themes::empreinte_des_codes(["finance"]);
    let autre_br = format!("\"{}-br\"", autre.trim_matches('"'));
    for presentee in [
        autre.as_str(),
        autre_br.as_str(),
        "\"pas-une-empreinte\"",
        "*",
        "",
    ] {
        let refus = suivre(&bac, awa, &["gender"], Some(presentee))
            .await
            .expect_err("rien ne désigne l'état courant");
        assert_eq!(
            refus.code,
            ErrorCode::NegotiationThemesStale,
            "« {presentee} »"
        );
    }

    assert_eq!(lignes_de_suivi(&bac, awa).await, avant, "aucune écriture");
}

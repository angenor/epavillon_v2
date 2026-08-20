//! Les deux semis des tests — **hors de `docs/database/`**, ce ne sont pas des
//! données du modèle.
//!
//! Le premier est la **paire de jumelles OSED** : deux fiches qui désignent la
//! même maison, l'une par son nom complet, l'autre par son sigle développé, et
//! qui déclarent le même domaine. C'est le cas d'école du défaut n° 1 de la v1,
//! et la matière de la file des doublons comme de l'écran de fusion.
//!
//! Le second est le **référentiel de cinq mille fiches** sur lequel se mesure la
//! recherche. Sa distribution n'est pas décorative : cinq mille noms tirés au
//! hasard n'ont presque aucun trigramme commun, le parcours d'index rendrait une
//! poignée de lignes, et la mesure serait excellente et fausse. Les noms sont
//! donc composés depuis un petit corpus francophone, de sorte que beaucoup
//! partagent leurs premiers mots — le cas défavorable, celui du vrai
//! référentiel (research.md § R3).

#![allow(dead_code)]

use uuid::Uuid;

use super::{pays, Bac};

pub struct PaireOsed {
    /// Fiche complète, vérifiée, portant le domaine vérifié.
    pub complete: Uuid,
    /// Fiche jumelle, créée par quelqu'un qui cherchait le sigle. Le même
    /// domaine, non vérifié — l'unicité ne porte que sur les domaines vérifiés.
    pub jumelle: Uuid,
    pub domaine: &'static str,
}

pub const DOMAINE_OSED: &str = "osed-sahel.org";

/// Sème les deux fiches OSED et leurs signaux de rapprochement.
pub async fn paire_osed(bac: &Bac) -> PaireOsed {
    let burkina = pays(bac, "BFA").await;

    let complete = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, country_id, city,
                status, verified_at, website)
           VALUES ('Observatoire du Sahel pour l''environnement et le développement',
                   'OSED', 'osed'::platform.slug, 'ngo_association', $1, 'Ouagadougou',
                   'active', now(), 'https://www.osed-sahel.org'::platform.url)
        RETURNING id"#,
        burkina
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la fiche OSED complète");

    let jumelle = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, country_id, city,
                status, contact_email)
           VALUES ('OSED Sahel', 'OSED-S', 'osed-sahel'::platform.slug,
                   'ngo_association', $1, 'Ouagadougou', 'candidate',
                   'contact@osed-sahel.org'::platform.email)
        RETURNING id"#,
        burkina
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la fiche OSED jumelle");

    // Le même domaine des deux côtés : c'est le signal qui les fait remonter.
    // Un seul est vérifié — `ux_organization_domains_verified` ne tolère pas le
    // second, et c'est justement ce que la fusion viendra réparer.
    sqlx::query!(
        "INSERT INTO org.organization_domains
             (organization_id, domain, verified_at, verification_method, auto_join)
         VALUES ($1, $3, now(), 'manual', true),
                ($2, $3, NULL, NULL, false)",
        complete,
        jumelle,
        DOMAINE_OSED
    )
    .execute(bac.pool())
    .await
    .expect("insertion des domaines OSED");

    // Une dénomination de plus sur la fiche complète : « l'OSED du Sahel », la
    // formulation qu'emploient les courriels et que personne ne saisirait comme
    // nom légal.
    sqlx::query!(
        "INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
         VALUES ($1, 'Observatoire du Sahel', 'short', true)",
        complete
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la dénomination courte");

    PaireOsed {
        complete,
        jumelle,
        domaine: DOMAINE_OSED,
    }
}

// -----------------------------------------------------------------------------
// Le référentiel de mesure
// -----------------------------------------------------------------------------

/// Natures d'organisation les plus courantes du référentiel réel. Beaucoup de
/// fiches commencent par le même mot : c'est ce qui rend la mesure honnête.
const NATURES: &[&str] = &[
    "Réseau",
    "Institut",
    "Observatoire",
    "Union",
    "Association",
    "Fondation",
    "Agence",
    "Centre",
    "Coalition",
    "Alliance",
];

const QUALIFIANTS: &[&str] = &[
    "national",
    "régional",
    "africain",
    "panafricain",
    "international",
    "francophone",
    "sahélien",
    "caribéen",
];

const DOMAINES: &[&str] = &[
    "du climat",
    "de la désertification",
    "de la biodiversité",
    "des énergies renouvelables",
    "de l'eau",
    "des forêts",
    "du développement durable",
    "de l'économie circulaire",
    "de l'adaptation côtière",
    "de la transition écologique",
];

const LIEUX: &[&str] = &[
    "du Sénégal",
    "du Burkina Faso",
    "du Cameroun",
    "de Côte d'Ivoire",
    "du Bénin",
    "du Niger",
    "du Mali",
    "de Madagascar",
    "d'Haïti",
    "du Tchad",
];

pub const TAILLE_REFERENTIEL: usize = 5_000;

/// Sème `TAILLE_REFERENTIEL` fiches à distribution réaliste, plus un sigle une
/// fois sur deux.
///
/// L'insertion se fait en un seul ordre, par génération de séries : cinq mille
/// allers-retours coûteraient une minute là où celui-ci coûte deux secondes, et
/// c'est du temps de mesure qu'on perdrait à préparer la mesure.
///
/// Le tirage est **déterministe** — `setseed` avant `random()` : une mesure qui
/// change de jeu à chaque exécution ne se compare pas d'une fois sur l'autre.
pub async fn referentiel_de_mesure(bac: &Bac) {
    sqlx::query!("SELECT setseed(0.42)")
        .execute(bac.pool())
        .await
        .expect("graine du tirage");

    sqlx::query(
        r#"
        INSERT INTO org.organizations
            (legal_name, acronym, slug, organization_type_code, country_id, city, status)
        SELECT
            nom,
            CASE WHEN i % 2 = 0 THEN sigle END,
            (platform.slugify(nom) || '-' || i)::platform.slug,
            'ngo_association',
            NULL,
            'Ville ' || (i % 40),
            CASE WHEN i % 7 = 0 THEN 'candidate' ELSE 'active' END::org.organization_status
        FROM generate_series(1, $1) AS i
        CROSS JOIN LATERAL (
            SELECT
                nature || ' ' || qualifiant || ' ' || domaine || ' ' || lieu || ' ' || i AS nom,
                upper(left(nature, 1) || left(qualifiant, 1) || left(domaine, 3)) || i AS sigle
            FROM (
                SELECT
                    (ARRAY['Réseau','Institut','Observatoire','Union','Association',
                           'Fondation','Agence','Centre','Coalition','Alliance'])[1 + (i % 10)]  AS nature,
                    (ARRAY['national','régional','africain','panafricain','international',
                           'francophone','sahélien','caribéen'])[1 + ((i / 10) % 8)]             AS qualifiant,
                    (ARRAY['du climat','de la désertification','de la biodiversité',
                           'des énergies renouvelables','de l''eau','des forêts',
                           'du développement durable','de l''économie circulaire',
                           'de l''adaptation côtière','de la transition écologique'])[1 + ((i / 80) % 10)] AS domaine,
                    (ARRAY['du Sénégal','du Burkina Faso','du Cameroun','de Côte d''Ivoire',
                           'du Bénin','du Niger','du Mali','de Madagascar','d''Haïti',
                           'du Tchad'])[1 + ((i / 800) % 10)]                                    AS lieu
            ) AS parties
        ) AS compose
        "#,
    )
    .bind(TAILLE_REFERENTIEL as i32)
    .execute(bac.pool())
    .await
    .expect("semis du référentiel de mesure");

    // `ANALYZE` n'est pas une commodité : sans statistiques fraîches, le
    // planificateur travaille sur une table qu'il croit vide et choisit un
    // parcours séquentiel. La mesure porterait alors sur un plan que la
    // production ne verrait jamais.
    sqlx::query("ANALYZE org.organizations, org.organization_names")
        .execute(bac.pool())
        .await
        .expect("statistiques du référentiel");
}

/// Les formes de recherche que la mesure exerce : sigle, début de nom, deux
/// lettres, mot du milieu, terme sans résultat. Elles ne coûtent pas la même
/// chose, et c'est précisément pour cela qu'on les mélange.
pub const FORMES_DE_RECHERCHE: &[&str] = &[
    "IFDD",
    "institut",
    "in",
    "réseau national",
    "désertification",
    "observatoire du sahel",
    "biodiversité",
    "agence",
    "francophonie",
    "zzzzquelquechosequinexistepas",
];

//! **Un fichier se rattache à ce qu'il illustre, et jamais à autre chose.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace : les trois
//! déclinaisons d'une édition partent **en un geste**, une valeur nulle en
//! retire **une seule**, et **détacher ne détruit pas l'objet** — vérifié en le
//! relisant après coup, pas en relisant le code.
//!
//! # Pourquoi la mesure de B3 est ici en SQL, et à part en HTTP
//!
//! T091 veut que `GET /events/{slug}` serve de mesure. Ce crate ne peut pas le
//! frapper : monter l'application demanderait une dépendance de développement
//! vers `api`, et le contrôle bloquant du jalon —
//! `cargo tree -p media` sans arête vers un autre module — la verrait.
//!
//! La mesure est donc **dédoublée**, et les deux moitiés comptent : ici, les
//! **trois appels exacts** que `event/src/repo/cross.rs` fait à
//! `media.attached_image()` ; et dans `crates/api/tests/routes_media_edition_images.rs`,
//! la route elle-même, sur la vraie application.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::{AttachmentAssignment, AttachmentBatch, AttachmentPayload};
use media::service::attach;
use uuid::Uuid;

/// Les trois rôles d'une édition, dans l'ordre où le formulaire les présente.
const ROLES_DEDITION: [&str; 3] = ["banner", "cover", "thumbnail"];

/// Dépose un fichier **et le rend servable** : c'est l'état dans lequel une
/// lecture publique le trouve.
async fn objet_servable(bac: &Bac, acteur: Uuid, fichier: &commun::Fichier) -> Uuid {
    let depose = commun::deposer(bac, acteur, fichier, commun::metadonnees(fichier))
        .await
        .expect("dépôt");
    commun::passer_le_worker(bac).await;
    depose.asset.id
}

fn lot(edition: Uuid, affectations: Vec<AttachmentAssignment>) -> AttachmentBatch {
    AttachmentBatch {
        owner_schema: "event".to_owned(),
        owner_table: "events".to_owned(),
        owner_id: edition,
        assignments: affectations,
    }
}

fn affectation(role: &str, asset_id: Option<Uuid>) -> AttachmentAssignment {
    AttachmentAssignment {
        role: role.to_owned(),
        asset_id,
        alt_text_override: None,
    }
}

/// Les trois déclinaisons **telles que B3 les lit** : les trois appels exacts
/// que son dépôt de données fait à la fonction du modèle.
async fn images_de_ledition(bac: &Bac, edition: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
                     'banner',    media.attached_image('event', 'events', $1, 'banner'),
                     'cover',     media.attached_image('event', 'events', $1, 'cover'),
                     'thumbnail', media.attached_image('event', 'events', $1, 'thumbnail')
                  ) AS "images!""#,
        edition
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des trois déclinaisons")
}

/// **LES TROIS DÉCLINAISONS D'UNE ÉDITION S'ENREGISTRENT EN UN GESTE.**
///
/// C'est l'obligation que B3 avait laissée : son formulaire accepte trois
/// identifiants d'image **sans les poser**, parce que le rattachement appartient
/// à un autre schéma. Il est refermé ici, et **sans qu'une ligne du module
/// Événements change** : la lecture de B3 les trouve résolues.
#[tokio::test]
async fn les_trois_declinaisons_dune_edition_senregistrent_en_un_geste() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let bandeau = objet_servable(&bac, terrain.administratrice, &commun::bandeau_32_9()).await;
    let couverture =
        objet_servable(&bac, terrain.administratrice, &commun::couverture_16_9()).await;
    let vignette = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    // **Avant** : la lecture de B3 rend trois nuls, et c'est le cas courant.
    let avant = images_de_ledition(&bac, terrain.edition).await;
    for role in ROLES_DEDITION {
        assert!(avant[role].is_null(), "{role} devrait être vide au départ");
    }

    let poses = attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &lot(
            terrain.edition,
            vec![
                affectation("banner", Some(bandeau)),
                affectation("cover", Some(couverture)),
                affectation("thumbnail", Some(vignette)),
            ],
        ),
    )
    .await
    .expect("les trois déclinaisons en un geste");

    assert_eq!(poses.len(), 3);

    let apres = images_de_ledition(&bac, terrain.edition).await;
    for role in ROLES_DEDITION {
        let image = &apres[role];
        assert!(!image.is_null(), "{role} devrait être résolue");
        // **L'adresse est composée, jamais la clé nue** — et elle porte
        // toujours l'original, servable dès le dépôt.
        assert!(image["url"].as_str().expect("adresse").starts_with("http"));
        assert!(
            !image["sources"].is_null(),
            "les déclinaisons sont présentes"
        );
    }
    assert_eq!(apres["banner"]["asset_id"], serde_json::json!(bandeau));
    assert_eq!(apres["thumbnail"]["asset_id"], serde_json::json!(vignette));
}

/// **Une valeur nulle retire UNE déclinaison sans toucher aux deux autres.**
///
/// C'est le geste courant du formulaire : on remplace la vignette et on laisse
/// le reste. Un lot qui viderait tout ferait disparaître deux images qu'on
/// n'avait pas touchées, et personne ne le verrait avant la mise en ligne.
#[tokio::test]
async fn une_valeur_nulle_retire_une_declinaison_sans_toucher_aux_autres() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let bandeau = objet_servable(&bac, terrain.administratrice, &commun::bandeau_32_9()).await;
    let couverture =
        objet_servable(&bac, terrain.administratrice, &commun::couverture_16_9()).await;
    let vignette = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &lot(
            terrain.edition,
            vec![
                affectation("banner", Some(bandeau)),
                affectation("cover", Some(couverture)),
                affectation("thumbnail", Some(vignette)),
            ],
        ),
    )
    .await
    .expect("les trois");

    attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &lot(terrain.edition, vec![affectation("thumbnail", None)]),
    )
    .await
    .expect("le retrait de la seule vignette");

    let apres = images_de_ledition(&bac, terrain.edition).await;
    assert!(apres["thumbnail"].is_null(), "la vignette devait partir");
    assert_eq!(apres["banner"]["asset_id"], serde_json::json!(bandeau));
    assert_eq!(apres["cover"]["asset_id"], serde_json::json!(couverture));
}

/// **Un carré comme bandeau est refusé, en citant le rapport reçu et le rapport
/// attendu** (FR-037).
///
/// « Les dimensions ne correspondent pas » n'apprend rien à qui doit recadrer :
/// il faut savoir ce qu'on a envoyé, ce qui était attendu, et de combien on est
/// à côté.
#[tokio::test]
async fn un_carre_comme_bandeau_est_refuse_en_citant_les_rapports() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let carre = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    let erreur = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            owner_schema: "event".to_owned(),
            owner_table: "events".to_owned(),
            owner_id: terrain.edition,
            role: "banner".to_owned(),
            asset_id: carre,
            sort_order: None,
            alt_text_override: None,
        },
    )
    .await
    .expect_err("un carré n'est pas un bandeau panoramique");

    assert_eq!(erreur.code, ErrorCode::MediaAspectRatio);
    assert_eq!(erreur.field.as_deref(), Some("file"));
    // Les quatre nombres sont **dans le message**, que l'écran affiche tel quel.
    assert!(erreur.message.contains("800 × 800"), "{}", erreur.message);
    assert!(
        erreur.message.contains("1,0000") || erreur.message.contains("1.0000"),
        "le rapport obtenu doit être cité : {}",
        erreur.message
    );
    assert!(
        erreur.message.contains("3,5556") || erreur.message.contains("3.5556"),
        "le rapport attendu doit être cité : {}",
        erreur.message
    );
    assert!(
        erreur.message.contains('%'),
        "la tolérance doit être citée : {}",
        erreur.message
    );
}

/// **Un objet image SANS dimensions relevées est accepté sur un rôle imposant
/// une forme** (FR-036).
///
/// C'est le relevé qui a échoué, pas le cadrage. Refuser ici transformerait une
/// panne de traitement en refus de téléversement — et c'est mot pour mot la
/// règle que `tg_validate_attachment` applique.
#[tokio::test]
async fn un_objet_sans_dimensions_relevees_est_accepte_sur_un_role_de_forme() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Déposé, **pas traité** : ni largeur, ni hauteur.
    let fichier = commun::vignette_1_1();
    let depose = commun::deposer(
        &bac,
        terrain.administratrice,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");
    assert_eq!((depose.asset.width, depose.asset.height), (None, None));

    attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            owner_schema: "event".to_owned(),
            owner_table: "events".to_owned(),
            owner_id: terrain.edition,
            role: "banner".to_owned(),
            asset_id: depose.asset.id,
            sort_order: None,
            alt_text_override: None,
        },
    )
    .await
    .expect("un objet non mesuré passe : le relevé a échoué, pas le cadrage");
}

/// **Un second objet AJOUTÉ sur un rôle exclusif est refusé ; REMPLACÉ, il
/// passe, et l'ancien rattachement a disparu.**
#[tokio::test]
async fn un_second_objet_sur_un_role_exclusif_est_refuse_mais_remplace_il_passe() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let premiere = objet_servable(&bac, terrain.administratrice, &commun::couverture_16_9()).await;
    // **Un autre contenu**, et non le même sous un autre nom : la déduplication
    // rendrait le même objet, et le test ne mesurerait rien.
    let seconde = objet_servable(
        &bac,
        terrain.administratrice,
        &commun::image("autre-couverture.png", 1600, 900),
    )
    .await;
    assert_ne!(premiere, seconde);

    let ajout = |asset_id: Uuid| AttachmentPayload {
        owner_schema: "event".to_owned(),
        owner_table: "events".to_owned(),
        owner_id: terrain.edition,
        role: "cover".to_owned(),
        asset_id,
        sort_order: None,
        alt_text_override: None,
    };

    let pose = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ajout(premiere),
    )
    .await
    .expect("la première couverture");

    let erreur = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ajout(seconde),
    )
    .await
    .expect_err("un rôle exclusif n'accepte pas un second objet");
    assert_eq!(erreur.code, ErrorCode::MediaRoleExclusive);
    assert_eq!(erreur.field.as_deref(), Some("role"));

    // Remplacée, elle passe — et l'ancien rattachement n'existe plus.
    let apres = attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &lot(terrain.edition, vec![affectation("cover", Some(seconde))]),
    )
    .await
    .expect("le remplacement");

    assert_eq!(apres.len(), 1);
    assert_eq!(apres[0].asset_id, seconde);
    assert_ne!(
        apres[0].attachment_id, pose.attachment_id,
        "l'ancien rattachement devait disparaître"
    );
}

/// **Une combinaison non déclarée est refusée en nommant l'entité et le rôle,
/// jamais en 500.**
///
/// La garde existe pour `event.events` ; c'est le **rôle** qui n'y est pas
/// déclaré. Un 500 dirait « panne » là où la demande est simplement hors de la
/// table blanche.
#[tokio::test]
async fn une_combinaison_non_declaree_est_refusee_en_nommant_lentite_et_le_role() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let image = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    let erreur = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            owner_schema: "event".to_owned(),
            owner_table: "events".to_owned(),
            owner_id: terrain.edition,
            // Une édition n'a pas de logo : la table blanche ne le déclare pas.
            role: "logo".to_owned(),
            asset_id: image,
            sort_order: None,
            alt_text_override: None,
        },
    )
    .await
    .expect_err("un rôle non déclaré pour cette entité");

    assert_eq!(erreur.code, ErrorCode::MediaRoleNotDeclared);
    assert_eq!(erreur.field.as_deref(), Some("role"));
    let detail = erreur.detail.unwrap_or_default();
    assert!(
        detail.contains("logo"),
        "le rôle doit être nommé : {detail}"
    );
    assert!(
        detail.contains("event.events"),
        "l'entité doit être nommée : {detail}"
    );
}

/// **DÉTACHER NE DÉTRUIT PAS L'OBJET.**
///
/// L'objet est relu **après** le détachement, et il est toujours là — en base
/// comme sur le stockage. Un même fichier illustre souvent plusieurs entités,
/// la déduplication le garantissant (écart n° 128).
#[tokio::test]
async fn detacher_ne_detruit_pas_lobjet() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let fichier = commun::couverture_16_9();
    let depose = commun::deposer(
        &bac,
        terrain.administratrice,
        &fichier,
        commun::metadonnees(&fichier),
    )
    .await
    .expect("dépôt");
    commun::passer_le_worker(&bac).await;

    let pose = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &AttachmentPayload {
            owner_schema: "event".to_owned(),
            owner_table: "events".to_owned(),
            owner_id: terrain.edition,
            role: "cover".to_owned(),
            asset_id: depose.asset.id,
            sort_order: None,
            alt_text_override: None,
        },
    )
    .await
    .expect("pose");

    attach::detacher(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        pose.attachment_id,
    )
    .await
    .expect("détachement");

    let objet = media::service::read::objet(&bac.state, depose.asset.id)
        .await
        .expect("l'objet est toujours là");
    assert_eq!(objet.status, "ready");
    assert!(
        commun::lire_sur_le_stockage(&bac, &objet.object_key)
            .await
            .is_some(),
        "le fichier doit être encore sur le stockage"
    );
    assert!(images_de_ledition(&bac, terrain.edition).await["cover"].is_null());
}

/// **Un rôle multiple rend ses objets dans l'ordre de tri, et l'ordre est
/// modifiable.**
///
/// Aucune route de réordonnancement n'existe, et il n'en faut pas : renvoyer la
/// même liste dans un autre ordre suffit.
#[tokio::test]
async fn un_role_multiple_rend_ses_objets_dans_lordre_et_lordre_est_modifiable() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let premiere = objet_servable(
        &bac,
        terrain.administratrice,
        &commun::image("g1.png", 900, 600),
    )
    .await;
    let deuxieme = objet_servable(
        &bac,
        terrain.administratrice,
        &commun::image("g2.png", 901, 600),
    )
    .await;
    let troisieme = objet_servable(
        &bac,
        terrain.administratrice,
        &commun::image("g3.png", 902, 600),
    )
    .await;

    let galerie = |ordre: [Uuid; 3]| {
        lot(
            terrain.edition,
            ordre
                .iter()
                .map(|id| affectation("gallery", Some(*id)))
                .collect(),
        )
    };

    let poses = attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &galerie([premiere, deuxieme, troisieme]),
    )
    .await
    .expect("trois images dans une galerie");

    assert_eq!(
        poses.iter().map(|m| m.asset_id).collect::<Vec<_>>(),
        vec![premiere, deuxieme, troisieme]
    );
    assert_eq!(
        poses.iter().map(|m| m.sort_order).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let reordonnees = attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &galerie([troisieme, premiere, deuxieme]),
    )
    .await
    .expect("le même lot, dans un autre ordre");

    assert_eq!(
        reordonnees.iter().map(|m| m.asset_id).collect::<Vec<_>>(),
        vec![troisieme, premiere, deuxieme]
    );
}

/// **Le texte alternatif propre à un usage prime sur celui de l'objet, sans le
/// modifier** (FR-040).
///
/// Un objet dédupliqué sert plusieurs fiches : le texte pertinent n'y est pas le
/// même. Le repli est résolu **en base**, par la fonction du modèle.
#[tokio::test]
async fn le_texte_alternatif_dun_usage_prime_sans_modifier_lobjet() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let image = objet_servable(&bac, terrain.administratrice, &commun::couverture_16_9()).await;

    attach::remplacer(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &lot(
            terrain.edition,
            vec![AttachmentAssignment {
                role: "cover".to_owned(),
                asset_id: Some(image),
                alt_text_override: Some(serde_json::json!({
                    "fr": "La salle plénière de la COP31",
                    "en": "The COP31 plenary room"
                })),
            }],
        ),
    )
    .await
    .expect("pose avec surcharge");

    let resolue = images_de_ledition(&bac, terrain.edition).await;
    assert_eq!(
        resolue["cover"]["alt_text"]["fr"],
        serde_json::json!("La salle plénière de la COP31")
    );

    // **L'objet n'a pas bougé** : c'est le rattachement qui porte la surcharge.
    let objet = media::service::read::objet(&bac.state, image)
        .await
        .expect("lecture de l'objet");
    assert_eq!(
        objet.alt_text.expect("texte de l'objet")["fr"],
        serde_json::json!("Une image d'épreuve")
    );
}

/// **Un compte détaché sur une édition ne peut rattacher à AUCUNE autre**, et
/// six identifiants forgés mènent au même refus qu'un identifiant inexistant.
///
/// Règle métier n° 8, y compris quand l'utilisateur forge une URL. Le refus
/// prend la forme d'une absence : un 403 dirait à qui forge que l'entité existe.
#[tokio::test]
async fn un_compte_detache_ne_rattache_a_aucune_autre_entite() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let image = objet_servable(&bac, terrain.administratrice, &commun::vignette_1_1()).await;

    // Une seconde édition, qu'elle n'administre pas.
    let autre_edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, has_pavilion)
           SELECT s.id, 'COP32', 2028,
                  '{"fr":"COP32","en":"COP32"}'::jsonb, 'COP32', 'cop32'::platform.slug,
                  '{"fr":"Édition suivante.","en":"Next edition."}'::jsonb,
                  'announced', 'online', 'UTC'::platform.timezone_name,
                  now() + interval '2 years', now() + interval '2 years 10 days', false
             FROM event.event_series s WHERE s.code = 'cop_climate'
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("seconde édition");

    let vise = |schema: &str, table: &str, id: Uuid, role: &str| AttachmentPayload {
        owner_schema: schema.to_owned(),
        owner_table: table.to_owned(),
        owner_id: id,
        role: role.to_owned(),
        asset_id: image,
        sort_order: None,
        alt_text_override: None,
    };

    let voisine = attach::poser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &vise("event", "events", autre_edition, "thumbnail"),
    )
    .await
    .expect_err("une édition d'à côté n'est pas administrée");
    assert_eq!(voisine.code, ErrorCode::NotFound);

    // Les six entités porteuses de la table blanche, chacune sur un identifiant
    // forgé : **le même refus, mot pour mot**.
    for (schema, table, role) in [
        ("org", "organizations", "logo"),
        ("event", "events", "thumbnail"),
        ("programme", "proposals", "cover"),
        ("programme", "sessions", "cover"),
        ("identity", "people", "avatar"),
        ("content", "highlights", "cover"),
    ] {
        let forge = attach::poser(
            &bac.state,
            &bac.ctx(),
            terrain.administratrice,
            &vise(schema, table, Uuid::now_v7(), role),
        )
        .await
        .expect_err("un identifiant forgé ne mène nulle part");

        assert_eq!(forge.code, ErrorCode::NotFound, "{schema}.{table}");
        assert_eq!(forge.message, voisine.message, "{schema}.{table}");
        assert_eq!(forge.field, voisine.field, "{schema}.{table}");
    }
}

/// **La table blanche s'annonce, forme attendue comprise.**
///
/// Sans le rapport et sa tolérance, l'écran apprend la forme par le refus —
/// c'est-à-dire après que le fichier a traversé le réseau.
#[tokio::test]
async fn les_regles_dune_entite_annoncent_la_forme_attendue() {
    let bac = Bac::monter().await;

    let regles = attach::roles(&bac.state, "event", "events")
        .await
        .expect("les règles d'une édition");

    let bandeau = regles
        .iter()
        .find(|r| r.role == "banner")
        .expect("le rôle bandeau");
    assert!(!bandeau.is_multiple);
    assert_eq!(bandeau.expected_aspect_ratio.as_deref(), Some("3.5556"));
    assert_eq!(bandeau.aspect_ratio_tolerance, "0.020");
    assert_eq!(bandeau.allowed_mime_prefixes, vec!["image/*".to_owned()]);

    let galerie = regles
        .iter()
        .find(|r| r.role == "gallery")
        .expect("le rôle galerie");
    assert!(galerie.is_multiple);
    assert!(galerie.expected_aspect_ratio.is_none());
}

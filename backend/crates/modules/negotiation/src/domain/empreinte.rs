//! L'empreinte d'un état — et **la seule façon de comparer celle qu'on nous
//! présente**, en `If-Match` comme en `If-None-Match`.
//!
//! Un relais qui compresse réécrit l'`ETag` : Apache y ajoute « -gzip » ou
//! « -br » par défaut (`DeflateAlterETag`, `BrotliAlterETag`), nginx l'affaiblit
//! en `W/`. L'égalité brute rendrait alors `412` à chaque choix fait hors
//! connexion, et l'intention serait abandonnée avec un message faux. On ne tient
//! la configuration d'aucun hébergeur, et le prochain relais referait la même
//! chose : l'API compare donc ce qu'elle a émis — les 32 caractères
//! hexadécimaux — et laisse tomber le reste.

const LONGUEUR: usize = 32;

/// 16 octets de `token_hash` en hexadécimal, entre guillemets comme l'exige
/// l'en-tête `ETag`.
pub fn de(texte: &str) -> String {
    let octets = kernel::crypto::token_hash(texte);
    let mut hexa = String::with_capacity(LONGUEUR + 2);
    hexa.push('"');
    for octet in &octets[..LONGUEUR / 2] {
        hexa.push_str(&format!("{octet:02x}"));
    }
    hexa.push('"');
    hexa
}

/// Les 32 caractères hexadécimaux d'une valeur d'en-tête, `W/`, guillemets et
/// suffixe de relais retirés. Nul si la valeur n'en porte pas : elle est
/// illisible, et ne correspond à rien.
fn noyau(valeur: &str) -> Option<&str> {
    let valeur = valeur.trim();
    let valeur = valeur
        .strip_prefix("W/")
        .unwrap_or(valeur)
        .trim_matches('"');
    let noyau = valeur.get(..LONGUEUR)?;
    let suite_hexa = valeur[LONGUEUR..]
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_hexdigit());
    (noyau.bytes().all(|b| b.is_ascii_hexdigit()) && !suite_hexa).then_some(noyau)
}

/// L'empreinte présentée désigne-t-elle l'état courant ? Une liste — que
/// `If-None-Match` autorise — correspond si l'un de ses éléments correspond.
pub fn correspond(presentee: &str, courante: &str) -> bool {
    let Some(courante) = noyau(courante) else {
        return false;
    };
    presentee
        .split(',')
        .filter_map(noyau)
        .any(|n| n.eq_ignore_ascii_case(courante))
}

#[cfg(test)]
mod tests {
    use super::{correspond, de};

    #[test]
    fn un_relais_qui_compresse_ne_change_pas_letat_designe() {
        let x = de("adaptation\ngender");
        let nu = x.trim_matches('"');
        assert!(correspond(&x, &x));
        assert!(correspond(&format!("\"{nu}-br\""), &x), "Apache, brotli");
        assert!(correspond(&format!("\"{nu}-gzip\""), &x), "Apache, gzip");
        assert!(correspond(&format!("W/{x}"), &x), "nginx affaiblit");
        assert!(
            correspond(&format!("W/\"{nu}-gzip\""), &x),
            "les deux à la fois"
        );
        assert!(correspond(&format!("\"autre\", {x}"), &x), "une liste");
    }

    #[test]
    fn une_autre_empreinte_ou_une_valeur_illisible_ne_correspond_pas() {
        let x = de("adaptation");
        let nu = x.trim_matches('"');
        assert!(!correspond(&de("finance"), &x));
        for illisible in ["", "*", "\"\"", "\"pas-une-empreinte\"", "W/", &nu[..20]] {
            assert!(!correspond(illisible, &x), "« {illisible} »");
        }
        // Plus long que ce que l'API émet : une autre empreinte, pas un suffixe.
        assert!(!correspond(&format!("\"{nu}0\""), &x));
    }
}

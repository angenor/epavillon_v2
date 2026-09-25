//! L'import des sessions officielles de la CCNUCC (Guide Négo, étape 3a) : un
//! lecteur rend la forme pivot, les vocabulaires trient et rattachent, le
//! comparateur dit les écarts, et `repo::import` n'écrit qu'eux.

pub mod archive;
pub mod ccnucc;
pub mod comparaison;
pub mod denominations;
pub mod reel;
pub mod source;
pub mod traduction;

//! rust-nano — l'éditeur CLI de rust-man.
//!
//! Tab = 4 espaces. Complétion IA en texte fantôme. Zéro bruit.

use std::path::PathBuf;

fn main() {
    let file = std::env::args().nth(1).map(PathBuf::from);
    if matches!(
        std::env::args().nth(1).as_deref(),
        Some("-h") | Some("--help") | Some("-V") | Some("--version")
    ) {
        println!(
            "rust-nano — l'éditeur CLI de rust-man\n\n\
             Usage: rust-nano [fichier]\n\n\
             Tab        4 espaces (convention Rust)\n\
             Ctrl+S     sauvegarder\n\
             Ctrl+Q     quitter\n\
             Ctrl+K/U   couper / coller une ligne\n\
             Ctrl+Espace  complétion IA (texte fantôme)\n\
             Alt+→      accepter la suggestion IA\n\
             Échap      refuser la suggestion"
        );
        return;
    }
    if let Err(e) = rust_man::editor::run(file) {
        eprintln!("rust-nano: {e}");
        std::process::exit(1);
    }
}

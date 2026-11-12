//! Script de compilation : Cargo l'exécute AVANT de compiler l'appli.
//! Ici, il intègre l'icône dans l'exe Windows (ressource « icône » lue par l'explorateur).

fn main() {
    // `CARGO_CFG_TARGET_OS` : le système visé par la compilation (pas celui qui compile).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon/icon.ico");
        // Échec (outil `rc.exe` introuvable…) : on arrête la compilation avec le message.
        resource
            .compile()
            .expect("impossible d'intégrer l'icône dans l'exe");
    }
    // Recompiler seulement si l'icône change.
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");
}

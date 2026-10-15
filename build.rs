fn main() {
    // Icone de l'executable (ressource n°1, celle que GPUI charge pour la fenetre et la barre des taches).
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=app.rc");
    embed_resource::compile("app.rc", embed_resource::NONE).manifest_optional().unwrap();
}

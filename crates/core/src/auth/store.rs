use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Zapis {
    pub refresh_token: String,
    pub nick: String,
}

pub fn load(path: &Path) -> Option<Zapis> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn save(path: &Path, z: &Zapis) -> std::io::Result<()> {
    if let Some(rodzic) = path.parent() {
        std::fs::create_dir_all(rodzic)?;
    }
    let tresc = serde_json::to_vec_pretty(z)?;
    std::fs::write(path, tresc)?;
    // Plik zawiera token odswiezania, wiec na Linuksie zawezamy uprawnienia.
    // Na Windowsie polegamy na uprawnieniach katalogu uzytkownika.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

use base64::Engine;
use color_eyre::eyre::{Result, bail};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

use crate::core::ROOT_MANIFEST_FILENAME;

pub const STORE_DIRECTORY_ENV: &str = "TAKT_STORE_DIR";
pub const STORE_VERSION: &str = "v1";
pub const STORE_FILES_SUBDIR: &str = "files";
pub const STORE_INDEX_SUBDIR: &str = "index";
pub const STORE_VIEWS_SUBDIR: &str = "virtual-store";
pub const CACHE_DIRECTORY_NAME: &str = "takt";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    pub hex_hash: String,
    pub store_path: PathBuf,
    pub executable: bool,
    pub size: u64,
}

pub type PackageIndex = BTreeMap<String, StoredFile>;

pub fn import_npm_tarball_into_store(store_root: &Path, bytes: &[u8]) -> Result<PackageIndex> {
    fs::create_dir_all(store_root)?;
    let decoder = GzDecoder::new(bytes);
    let mut archive = Archive::new(decoder);
    let mut index = PackageIndex::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir()
            || matches!(
                entry_type,
                tar::EntryType::XGlobalHeader
                    | tar::EntryType::XHeader
                    | tar::EntryType::GNULongName
                    | tar::EntryType::GNULongLink
            )
        {
            continue;
        }
        if !matches!(
            entry_type,
            tar::EntryType::Regular | tar::EntryType::Continuous
        ) {
            bail!("tarball entry type {entry_type:?} is not supported");
        }

        let raw_path = entry.path()?.to_path_buf();
        let Some(relative) = normalize_tar_entry_path(&raw_path)? else {
            continue;
        };

        let mode = entry.header().mode().unwrap_or(0o644);
        let executable = mode & 0o111 != 0;
        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;

        let stored = import_store_file(store_root, &content, executable)?;
        if index.insert(relative.clone(), stored).is_some() {
            bail!("tarball contains duplicate path '{}'", relative);
        }
    }

    Ok(index)
}

pub fn load_cached_package_index(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
) -> Option<PackageIndex> {
    let index_path = package_index_path(store_root, name, version, integrity).ok()?;
    let bytes = fs::read(&index_path).ok()?;
    let index: PackageIndex = serde_json::from_slice(&bytes).ok()?;
    if package_index_is_stale(&index) {
        let _ = fs::remove_file(index_path);
        return None;
    }
    Some(index)
}

pub fn save_cached_package_index(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
    index: &PackageIndex,
) -> Result<()> {
    let index_path = package_index_path(store_root, name, version, integrity)?;
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(index_path, serde_json::to_vec_pretty(index)?)?;
    Ok(())
}

pub fn materialize_package_view(
    cache_root: &Path,
    package: &str,
    version: &str,
    integrity: &str,
    index: &PackageIndex,
) -> Result<PathBuf> {
    let package_path = materialized_package_view_path(cache_root, package, version, integrity);
    if package_path.join(ROOT_MANIFEST_FILENAME).exists() {
        return Ok(package_path);
    }
    if package_path.exists() {
        fs::remove_dir_all(&package_path)?;
    }

    let staging_path = package_path.with_extension(format!("tmp-{}", now_unix_ms()?));
    if staging_path.exists() {
        fs::remove_dir_all(&staging_path)?;
    }
    fs::create_dir_all(&staging_path)?;

    for (relative_path, stored) in index {
        let target_path = staging_path.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        link_or_copy_file(&stored.store_path, &target_path, stored.executable)?;
    }

    if !staging_path.join(ROOT_MANIFEST_FILENAME).exists() {
        bail!(
            "cached package '{}' at version '{}' does not contain {}",
            package,
            version,
            ROOT_MANIFEST_FILENAME
        );
    }

    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(&staging_path, &package_path) {
        Ok(()) => Ok(package_path),
        Err(_error) if package_path.exists() => {
            let _ = fs::remove_dir_all(&staging_path);
            Ok(package_path)
        }
        Err(error) => Err(error.into()),
    }
}

pub fn resolve_store_root(repo_root: &Path) -> PathBuf {
    let repo_local_store = repo_root
        .join(".takt")
        .join("store")
        .join(STORE_VERSION)
        .join(STORE_FILES_SUBDIR);
    if repo_local_store.exists() {
        return repo_local_store;
    }

    resolve_store_root_from(
        Some(repo_root),
        env::var_os(STORE_DIRECTORY_ENV).map(PathBuf::from),
        env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

pub fn resolve_store_root_from(
    repo_root: Option<&Path>,
    configured_store_root: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
    _local_app_data: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = configured_store_root.filter(|path| !path.as_os_str().is_empty()) {
        return path;
    }

    #[cfg(windows)]
    if let Some(path) = _local_app_data.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    if let Some(path) = xdg_data_home.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    if let Some(path) = home.filter(|path| !path.as_os_str().is_empty()) {
        return path
            .join(".local")
            .join("share")
            .join(CACHE_DIRECTORY_NAME)
            .join("store")
            .join(STORE_VERSION)
            .join(STORE_FILES_SUBDIR);
    }

    repo_root
        .map(|root| {
            root.join(".takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        })
        .unwrap_or_else(|| {
            PathBuf::from(".takt")
                .join("store")
                .join(STORE_VERSION)
                .join(STORE_FILES_SUBDIR)
        })
}

pub fn resolve_cache_root(repo_root: &Path) -> PathBuf {
    let repo_local_cache = repo_root.join(".takt").join("cache");
    if repo_local_cache.exists() {
        return repo_local_cache;
    }

    resolve_cache_root_from(
        env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
        env::var_os("HOME").map(PathBuf::from),
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
    )
}

pub fn resolve_cache_root_from(
    xdg_cache_home: Option<PathBuf>,
    home: Option<PathBuf>,
    _local_app_data: Option<PathBuf>,
) -> PathBuf {
    #[cfg(windows)]
    if let Some(path) = _local_app_data.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(CACHE_DIRECTORY_NAME);
    }

    if let Some(path) = xdg_cache_home.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(CACHE_DIRECTORY_NAME);
    }

    if let Some(path) = home.filter(|path| !path.as_os_str().is_empty()) {
        return path.join(".cache").join(CACHE_DIRECTORY_NAME);
    }

    PathBuf::from(".takt").join("cache")
}

pub fn virtual_store_root(cache_root: &Path) -> PathBuf {
    cache_root.join(STORE_VIEWS_SUBDIR)
}

pub fn import_store_file(
    store_root: &Path,
    content: &[u8],
    executable: bool,
) -> Result<StoredFile> {
    let hex_hash = blake3::hash(content).to_hex().to_string();
    let store_path = store_file_path(store_root, &hex_hash);
    if let Some(parent) = store_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if store_path.exists() && store_path.metadata()?.len() != content.len() as u64 {
        fs::remove_file(&store_path)?;
    }

    if !store_path.exists() {
        use std::io::Write;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&store_path)
        {
            Ok(mut file) => file.write_all(content)?,
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && store_path.metadata()?.len() == content.len() as u64 => {}
            Err(error) => return Err(error.into()),
        }
    }

    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = store_path.metadata()?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&store_path, permissions)?;
    }

    Ok(StoredFile {
        hex_hash,
        store_path,
        executable,
        size: content.len() as u64,
    })
}

fn normalize_tar_entry_path(path: &Path) -> Result<Option<String>> {
    let mut components = path.components();
    let Some(first) = components.next() else {
        return Ok(None);
    };
    if first.as_os_str() != "package" {
        return Ok(None);
    }

    let relative = components.as_path();
    if relative.as_os_str().is_empty() {
        return Ok(None);
    }
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        bail!("tarball contains an unsafe path '{}'", relative.display());
    }

    Ok(Some(relative.to_string_lossy().replace('\\', "/")))
}

fn store_file_path(store_root: &Path, hex_hash: &str) -> PathBuf {
    let (shard, suffix) = hex_hash.split_at(2);
    store_root.join(shard).join(suffix)
}

fn package_index_is_stale(index: &PackageIndex) -> bool {
    !index.values().all(|stored| {
        stored
            .store_path
            .metadata()
            .map(|metadata| metadata.len() == stored.size)
            .unwrap_or(false)
    })
}

fn package_index_path(
    store_root: &Path,
    name: &str,
    version: &str,
    integrity: Option<&str>,
) -> Result<PathBuf> {
    let filename = format!(
        "{}@{}.json",
        encode_store_component(name),
        encode_store_component(version)
    );
    let index_root = store_index_root(store_root);
    if let Some(integrity) = integrity {
        let hex = integrity_to_hex(integrity)?;
        let shard = &hex[..16.min(hex.len())];
        Ok(index_root.join(shard).join(filename))
    } else {
        Ok(index_root.join(filename))
    }
}

fn store_index_root(store_root: &Path) -> PathBuf {
    store_root
        .parent()
        .unwrap_or(store_root)
        .join(STORE_INDEX_SUBDIR)
}

fn materialized_package_view_path(
    cache_root: &Path,
    package: &str,
    version: &str,
    integrity: &str,
) -> PathBuf {
    let key = blake3::hash(format!("{package}\n{version}\n{integrity}\n").as_bytes())
        .to_hex()
        .to_string();
    virtual_store_root(cache_root).join(format!("{}-{}", encode_store_component(package), key))
}

fn integrity_to_hex(integrity: &str) -> Result<String> {
    let Some((_, digest)) = integrity.split_once('-') else {
        bail!("unsupported integrity string '{integrity}'");
    };
    Ok(base64::engine::general_purpose::STANDARD
        .decode(digest)?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn encode_store_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let character = byte as char;
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
            encoded.push(character);
        } else {
            encoded.push('_');
            encoded.push_str(&format!("{byte:02x}"));
        }
    }
    encoded
}

fn link_or_copy_file(source: &Path, destination: &Path, executable: bool) -> Result<()> {
    if fs::hard_link(source, destination).is_err() {
        fs::copy(source, destination)?;
    }

    #[cfg(unix)]
    if executable {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = destination.metadata()?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions)?;
    }

    Ok(())
}

fn now_unix_ms() -> Result<u64> {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| color_eyre::eyre::eyre!("system clock is before UNIX_EPOCH: {error}"))?;
    Ok(duration.as_millis() as u64)
}

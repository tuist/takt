use color_eyre::eyre::{Result, bail, eyre};
use reqwest::blocking::Client;
use semver::{Version, VersionReq};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct NpmPackageDocument {
    #[serde(rename = "dist-tags", default)]
    pub dist_tags: BTreeMap<String, String>,
    #[serde(default)]
    pub versions: BTreeMap<String, NpmVersionDocument>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmVersionDocument {
    pub dist: NpmDistDocument,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NpmDistDocument {
    pub tarball: String,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub default_registry: String,
    pub scoped_registries: BTreeMap<String, String>,
    pub auth_tokens: BTreeMap<String, String>,
}

pub fn load_registry_config(repo_root: &Path) -> Result<RegistryConfig> {
    let mut config = RegistryConfig {
        default_registry: normalize_registry_url("https://registry.npmjs.org/"),
        scoped_registries: BTreeMap::new(),
        auth_tokens: BTreeMap::new(),
    };

    if let Some(home) = env::var_os("HOME") {
        let user_npmrc = PathBuf::from(home).join(".npmrc");
        if user_npmrc.exists() {
            merge_npmrc_file(&mut config, &user_npmrc)?;
        }
    }

    let project_npmrc = repo_root.join(".npmrc");
    if project_npmrc.exists() {
        merge_npmrc_file(&mut config, &project_npmrc)?;
    }

    Ok(config)
}

pub fn auth_token_for_url<'a>(config: &'a RegistryConfig, url: &str) -> Option<&'a str> {
    let key = registry_auth_key(url);
    config
        .auth_tokens
        .iter()
        .filter(|(candidate, _)| key.starts_with(candidate.as_str()))
        .max_by_key(|(candidate, _)| candidate.len())
        .map(|(_, token)| token.as_str())
}

pub fn fetch_registry_package_document(
    config: &RegistryConfig,
    package: &str,
) -> Result<NpmPackageDocument> {
    let registry = registry_url_for_package(config, package);
    let url = format!("{registry}{}", escape_registry_package_name(package));
    let client = Client::builder().build()?;
    let mut request = client.get(&url).header(
        "accept",
        "application/vnd.npm.install-v1+json, application/json",
    );
    if let Some(token) = auth_token_for_url(config, registry) {
        request = request.bearer_auth(token);
    }
    let response = request.send()?;
    if !response.status().is_success() {
        bail!(
            "failed to resolve dependency '{}': registry returned {}",
            package,
            response.status()
        );
    }
    Ok(response.json()?)
}

pub fn select_dependency_version(
    package: &str,
    specifier: &str,
    document: &NpmPackageDocument,
) -> Result<String> {
    if let Some(version) = document.dist_tags.get(specifier) {
        return Ok(version.clone());
    }

    if document.versions.contains_key(specifier) {
        return Ok(specifier.to_string());
    }

    let requirement = VersionReq::parse(specifier).map_err(|error| {
        eyre!(
            "dependency '{}' has unsupported specifier '{}': {error}",
            package,
            specifier
        )
    })?;

    let selected = document
        .versions
        .keys()
        .filter_map(|version| Version::parse(version).ok())
        .filter(|version| requirement.matches(version))
        .max()
        .ok_or_else(|| {
            eyre!(
                "dependency '{}' has no versions matching '{}'",
                package,
                specifier
            )
        })?;

    Ok(selected.to_string())
}

fn merge_npmrc_file(config: &mut RegistryConfig, path: &Path) -> Result<()> {
    for raw_line in fs::read_to_string(path)?.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = raw_key.trim();
        let value = interpolate_env_vars(raw_value.trim())?;

        if key == "registry" {
            config.default_registry = normalize_registry_url(&value);
        } else if key.starts_with('@') && key.ends_with(":registry") {
            let scope = key.trim_end_matches(":registry");
            config
                .scoped_registries
                .insert(scope.to_string(), normalize_registry_url(&value));
        } else if key.starts_with("//") && key.ends_with(":_authToken") {
            let host = key
                .trim_start_matches("//")
                .trim_end_matches(":_authToken")
                .to_string();
            config.auth_tokens.insert(host, value);
        }
    }

    Ok(())
}

fn interpolate_env_vars(value: &str) -> Result<String> {
    let mut rendered = value.to_string();
    while let Some(start) = rendered.find("${") {
        let rest = &rendered[start + 2..];
        let Some(end_offset) = rest.find('}') else {
            bail!("unterminated environment variable in npmrc value '{value}'");
        };
        let key = &rest[..end_offset];
        let env_value = env::var(key)
            .map_err(|_| eyre!("environment variable '{key}' referenced in .npmrc is not set"))?;
        rendered.replace_range(start..start + 3 + end_offset, &env_value);
    }
    Ok(rendered)
}

fn normalize_registry_url(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

fn registry_url_for_package<'a>(config: &'a RegistryConfig, package: &str) -> &'a str {
    package
        .split_once('/')
        .and_then(|(scope, _)| config.scoped_registries.get(scope))
        .map(String::as_str)
        .unwrap_or(&config.default_registry)
}

fn registry_auth_key(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

fn escape_registry_package_name(name: &str) -> String {
    name.replace('/', "%2f")
}

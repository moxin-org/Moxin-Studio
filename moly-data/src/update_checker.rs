use serde::Deserialize;

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/moxin-org/Moxin-Studio/releases/latest";

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub html_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub fn check_for_update(current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Moxin-Studio")
        .build()
        .map_err(|e| e.to_string())?;

    let release: GitHubRelease = client
        .get(GITHUB_RELEASES_URL)
        .send()
        .map_err(|e| e.to_string())?
        .json()
        .map_err(|e| e.to_string())?;

    let latest = release.tag_name.trim_start_matches('v');
    if !is_newer(latest, current_version) {
        return Ok(None);
    }

    let download_url = release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".dmg"))
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| release.html_url.clone());

    Ok(Some(UpdateInfo {
        version: latest.to_string(),
        download_url,
        html_url: release.html_url,
    }))
}

fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let l = parse(latest);
    let c = parse(current);
    for i in 0..l.len().max(c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

use super::super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenverseAsset {
    id: String,
    title: String,
    media_url: String,
    source_url: String,
    thumbnail_url: Option<String>,
    creator: Option<String>,
    license: Option<String>,
    license_url: Option<String>,
}

#[derive(Deserialize)]
struct OpenverseResponse {
    #[serde(default)]
    results: Vec<Value>,
}

#[tauri::command]
pub async fn search_openverse(
    query: String,
    media_type: String,
) -> Result<Vec<OpenverseAsset>, String> {
    let query = query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 200 {
        return Err("Enter a search phrase up to 200 characters".into());
    }
    let endpoint = match media_type.as_str() {
        "images" => "https://api.openverse.org/v1/images/",
        "audio" => "https://api.openverse.org/v1/audio/",
        _ => return Err("Unsupported asset type".into()),
    };
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|error| format!("Could not create API client: {error}"))?
        .get(endpoint)
        .query(&[("q", query), ("page_size", "24".into())])
        .send()
        .await
        .map_err(|error| format!("Openverse search failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Openverse search returned an error: {error}"))?
        .json::<OpenverseResponse>()
        .await
        .map_err(|error| format!("Could not read Openverse results: {error}"))?;
    Ok(response
        .results
        .into_iter()
        .filter_map(|item| {
            let media_url = item.get("url")?.as_str()?.to_owned();
            Some(OpenverseAsset {
                id: item
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                title: item
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Untitled media")
                    .to_owned(),
                media_url,
                source_url: item
                    .get("foreign_landing_url")
                    .or_else(|| item.get("detail_url"))?
                    .as_str()?
                    .to_owned(),
                thumbnail_url: item
                    .get("thumbnail")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                creator: item
                    .get("creator")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license: item
                    .get("license")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                license_url: item
                    .get("license_url")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn download_openverse_asset(
    app: AppHandle,
    asset: OpenverseAsset,
) -> Result<String, String> {
    let url = reqwest::Url::parse(&asset.media_url)
        .map_err(|error| format!("Invalid media URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("Only HTTPS media downloads are allowed".into());
    }
    let mut response = net::fetch_public_http(url.clone())
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Media provider returned an error: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > 512 * 1024 * 1024)
    {
        return Err("This asset is larger than Easy Edits Pro's 512 MiB download limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Media download failed: {error}"))?
    {
        if bytes.len().saturating_add(chunk.len()) > 512 * 1024 * 1024 {
            return Err("This asset is larger than Easy Edits Pro's 512 MiB download limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate the app data folder: {error}"))?
        .join("assets");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create asset folder: {error}"))?;
    let extension = url
        .path_segments()
        .and_then(|mut parts| parts.next_back())
        .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
        .filter(|ext| ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("bin");
    let title = asset
        .title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut target = directory.join(format!("{}.{}", title.trim_matches('-'), extension));
    let mut suffix = 1;
    while target.exists() {
        target = directory.join(format!("{}-{suffix}.{extension}", title.trim_matches('-')));
        suffix += 1;
    }
    std::fs::write(&target, &bytes)
        .map_err(|error| format!("Could not save downloaded asset: {error}"))?;
    let attribution_path = target.with_extension(format!("{extension}.attribution.json"));
    let attribution = json!({
        "provider": "Openverse",
        "title": asset.title,
        "creator": asset.creator,
        "license": asset.license,
        "licenseUrl": asset.license_url,
        "sourceUrl": asset.source_url,
        "mediaUrl": asset.media_url
    });
    std::fs::write(
        attribution_path,
        serde_json::to_vec_pretty(&attribution).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Could not save asset attribution: {error}"))?;
    let _ = app.asset_protocol_scope().allow_file(&target);
    Ok(target.to_string_lossy().into_owned())
}

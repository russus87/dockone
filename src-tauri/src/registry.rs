//! Minimal Docker Registry v2 client: given an image reference, fetch the
//! current content digest of its tag so we can compare it against the locally
//! pulled digest and tell whether an update is available. Supports Docker Hub
//! and any registry that speaks the standard bearer-token auth flow.

const ACCEPT: &str = "application/vnd.docker.distribution.manifest.list.v2+json, \
     application/vnd.oci.image.index.v1+json, \
     application/vnd.docker.distribution.manifest.v2+json, \
     application/vnd.oci.image.manifest.v1+json";

/// Split an image reference into `(registry_host, repository, tag)`.
pub fn parse_ref(image: &str) -> (String, String, String) {
    let image = image.split('@').next().unwrap_or(image);

    // separate the tag (a ':' that is not part of a registry host:port)
    let (name, tag) = match image.rfind(':') {
        Some(idx) if !image[idx + 1..].contains('/') => {
            (&image[..idx], image[idx + 1..].to_string())
        }
        _ => (image, "latest".to_string()),
    };

    match name.find('/') {
        Some(idx)
            if name[..idx].contains('.')
                || name[..idx].contains(':')
                || &name[..idx] == "localhost" =>
        {
            (name[..idx].to_string(), name[idx + 1..].to_string(), tag)
        }
        _ => {
            let repo = if name.contains('/') {
                name.to_string()
            } else {
                format!("library/{name}")
            };
            ("registry-1.docker.io".to_string(), repo, tag)
        }
    }
}

/// Fetch the remote content digest (`sha256:…`) for an image's tag.
pub async fn latest_digest(image: &str) -> Result<String, String> {
    let (registry, repo, tag) = parse_ref(image);
    let client = reqwest::Client::builder()
        .user_agent("DockOne")
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("https://{registry}/v2/{repo}/manifests/{tag}");

    let mut resp = client
        .get(&url)
        .header("Accept", ACCEPT)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        let www = resp
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let token = fetch_token(&client, &www).await?;
        resp = client
            .get(&url)
            .header("Accept", ACCEPT)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }

    if !resp.status().is_success() {
        return Err(format!("registry {}", resp.status().as_u16()));
    }
    resp.headers()
        .get("docker-content-digest")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| "digest non disponibile".into())
}

async fn fetch_token(client: &reqwest::Client, www_auth: &str) -> Result<String, String> {
    let realm = extract(www_auth, "realm").ok_or("realm mancante")?;
    let mut url = reqwest::Url::parse(&realm).map_err(|e| e.to_string())?;
    {
        let mut q = url.query_pairs_mut();
        if let Some(s) = extract(www_auth, "service") {
            q.append_pair("service", &s);
        }
        if let Some(s) = extract(www_auth, "scope") {
            q.append_pair("scope", &s);
        }
    }
    let json: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    json.get("token")
        .or_else(|| json.get("access_token"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "token non ottenuto".into())
}

fn extract(header: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let start = header.find(&pat)? + pat.len();
    let end = header[start..].find('"')? + start;
    Some(header[start..end].to_string())
}

mod docker;
mod ssh;
mod state;

use bollard::Docker;
use futures_util::future::join_all;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

use docker::{
    ContainerDto, DeploySpec, DfDto, EventDto, HostSummary, ImageDto, NetworkDto, PruneResult,
    StatDto, VolumeDto,
};
use state::{AppState, Host, PersistedData, Settings, Tunnel};

// ---------------------------------------------------------------------------
// connection resolution
// ---------------------------------------------------------------------------

/// Resolve a live Docker handle for `host_id`, reusing the cached connection.
/// For `ssh://` hosts this first ensures an SSH tunnel is up and connects to
/// the resulting local endpoint.
async fn resolve(app_state: &AppState, host_id: &str) -> Result<Docker, String> {
    let host = {
        let d = app_state.data.lock().map_err(|_| "stato bloccato")?;
        d.hosts
            .iter()
            .find(|h| h.id == host_id)
            .cloned()
            .ok_or_else(|| format!("host «{host_id}» non trovato"))?
    };

    // Resolve the effective endpoint (SSH hosts tunnel to a local tcp port).
    let endpoint = if host.endpoint.trim().starts_with("ssh://") {
        ensure_tunnel(app_state, &host).await?
    } else {
        host.endpoint.clone()
    };

    // fast path: already connected (ensure_tunnel invalidates this on respawn)
    if let Ok(cache) = app_state.conns.lock() {
        if let Some(d) = cache.get(host_id) {
            return Ok(d.clone());
        }
    }

    let effective = Host {
        id: host.id.clone(),
        name: host.name.clone(),
        endpoint,
        favorite: host.favorite,
    };
    let docker = docker::connect(&effective)?;
    if let Ok(mut cache) = app_state.conns.lock() {
        cache.insert(host_id.to_string(), docker.clone());
    }
    Ok(docker)
}

/// Ensure a live SSH tunnel exists for `host` and return its local `tcp://`
/// endpoint, (re)spawning `ssh` if needed.
async fn ensure_tunnel(app_state: &AppState, host: &Host) -> Result<String, String> {
    // reuse an existing, still-running tunnel
    if let Ok(mut tunnels) = app_state.tunnels.lock() {
        if let Some(t) = tunnels.get_mut(&host.id) {
            match t.child.try_wait() {
                Ok(None) => return Ok(t.endpoint.clone()), // still alive
                _ => {
                    // dead → drop it and invalidate the cached connection
                    tunnels.remove(&host.id);
                    if let Ok(mut c) = app_state.conns.lock() {
                        c.remove(&host.id);
                    }
                }
            }
        }
    }

    let ep = host.endpoint.clone();
    let (child, endpoint) = tokio::task::spawn_blocking(move || ssh::open(&ep))
        .await
        .map_err(|e| e.to_string())??;

    if let Ok(mut tunnels) = app_state.tunnels.lock() {
        tunnels.insert(
            host.id.clone(),
            Tunnel {
                child,
                endpoint: endpoint.clone(),
            },
        );
    }
    if let Ok(mut c) = app_state.conns.lock() {
        c.remove(&host.id);
    }
    Ok(endpoint)
}

fn ensure_writable(app_state: &AppState) -> Result<(), String> {
    let ro = app_state
        .data
        .lock()
        .map(|d| d.settings.read_only)
        .unwrap_or(false);
    if ro {
        Err("Modalità sola lettura attiva".into())
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// host management
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_hosts(st: State<AppState>) -> Result<Vec<Host>, String> {
    Ok(st.data.lock().map_err(|_| "stato bloccato")?.hosts.clone())
}

/// Try to reach a Docker endpoint without persisting it. When a `tcp://` /
/// `http://` endpoint has no explicit port, the default 2375 is assumed.
#[tauri::command]
async fn test_host(name: Option<String>, endpoint: String) -> Result<String, String> {
    let display = name.unwrap_or_else(|| "test".into());

    // SSH: open a throwaway tunnel, ping through it, then tear it down.
    if endpoint.trim().starts_with("ssh://") {
        let ep = endpoint.trim().to_string();
        let (mut child, tcp) = tokio::task::spawn_blocking(move || ssh::open(&ep))
            .await
            .map_err(|e| e.to_string())??;
        let host = Host {
            id: "__probe__".into(),
            name: display,
            endpoint: tcp,
            favorite: false,
        };
        let res = docker::ping(&host).await;
        let _ = child.kill();
        return res;
    }

    let host = Host {
        id: "__probe__".into(),
        name: display,
        endpoint: normalize_probe(&endpoint),
        favorite: false,
    };
    docker::ping(&host).await
}

fn normalize_probe(ep: &str) -> String {
    let e = ep.trim();
    for scheme in ["tcp://", "http://"] {
        if let Some(rest) = e.strip_prefix(scheme) {
            // add the default plain-HTTP Docker port when none was given
            if !rest.contains(':') && !rest.is_empty() {
                return format!("{scheme}{rest}:2375");
            }
        }
    }
    e.to_string()
}

#[tauri::command]
fn add_host(st: State<AppState>, name: String, endpoint: String) -> Result<Vec<Host>, String> {
    let name = name.trim().to_string();
    let endpoint = endpoint.trim().to_string();
    if name.is_empty() || endpoint.is_empty() {
        return Err("Nome ed endpoint sono obbligatori".into());
    }
    let id = slug(&name);
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        if d.hosts.iter().any(|h| h.id == id) {
            return Err("Esiste già un host con questo nome".into());
        }
        d.hosts.push(Host {
            id,
            name,
            endpoint,
            favorite: false,
        });
    }
    st.save();
    list_hosts(st)
}

#[tauri::command]
fn remove_host(st: State<AppState>, id: String) -> Result<Vec<Host>, String> {
    if id == "local" {
        return Err("L'host locale non può essere rimosso".into());
    }
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        d.hosts.retain(|h| h.id != id);
    }
    if let Ok(mut c) = st.conns.lock() {
        c.remove(&id);
    }
    if let Ok(mut t) = st.tunnels.lock() {
        if let Some(mut tun) = t.remove(&id) {
            let _ = tun.child.kill();
        }
    }
    st.save();
    list_hosts(st)
}

#[tauri::command]
fn toggle_host_favorite(st: State<AppState>, id: String) -> Result<Vec<Host>, String> {
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        if let Some(h) = d.hosts.iter_mut().find(|h| h.id == id) {
            h.favorite = !h.favorite;
        }
    }
    st.save();
    list_hosts(st)
}

// ---------------------------------------------------------------------------
// settings
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_settings(st: State<AppState>) -> Result<Settings, String> {
    Ok(st.data.lock().map_err(|_| "stato bloccato")?.settings.clone())
}

#[tauri::command]
fn save_settings(st: State<AppState>, settings: Settings) -> Result<(), String> {
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        d.settings = settings;
    }
    st.save();
    Ok(())
}

// ---------------------------------------------------------------------------
// backup: export / import config
// ---------------------------------------------------------------------------

/// Write the full DockOne configuration (hosts + settings) to `path` as JSON.
#[tauri::command]
fn export_config(st: State<AppState>, path: String) -> Result<(), String> {
    let json = {
        let d = st.data.lock().map_err(|_| "stato bloccato")?;
        serde_json::to_string_pretty(&*d).map_err(|e| e.to_string())?
    };
    std::fs::write(&path, json).map_err(|e| format!("scrittura fallita: {e}"))
}

/// Replace the current configuration with the one stored in the file at `path`.
#[tauri::command]
fn import_config(st: State<AppState>, path: String) -> Result<(), String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("lettura fallita: {e}"))?;
    let mut parsed: PersistedData =
        serde_json::from_str(&raw).map_err(|e| format!("JSON non valido: {e}"))?;

    // never end up without the local host
    if !parsed.hosts.iter().any(|h| h.id == "local") {
        parsed.hosts.insert(
            0,
            Host {
                id: "local".into(),
                name: "Local".into(),
                endpoint: "local".into(),
                favorite: true,
            },
        );
    }

    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        *d = parsed;
    }
    if let Ok(mut c) = st.conns.lock() {
        c.clear();
    }
    if let Ok(mut m) = st.last_states.lock() {
        m.clear();
    }
    st.save();
    Ok(())
}

#[tauri::command]
fn toggle_favorite_container(st: State<AppState>, key: String) -> Result<Settings, String> {
    {
        let mut d = st.data.lock().map_err(|_| "stato bloccato")?;
        let favs = &mut d.settings.favorite_containers;
        if let Some(pos) = favs.iter().position(|k| k == &key) {
            favs.remove(pos);
        } else {
            favs.push(key);
        }
    }
    st.save();
    get_settings(st)
}

// ---------------------------------------------------------------------------
// docker queries
// ---------------------------------------------------------------------------

#[tauri::command]
async fn dashboard(app: tauri::AppHandle) -> Result<Vec<HostSummary>, String> {
    let st = app.state::<AppState>();
    let hosts = st.data.lock().map_err(|_| "stato bloccato")?.hosts.clone();

    let futures = hosts.into_iter().map(|host| {
        let st = app.state::<AppState>();
        async move {
            match resolve(&st, &host.id).await {
                Ok(docker) => docker::host_summary(&host, &docker).await,
                Err(e) => HostSummary {
                    id: host.id.clone(),
                    name: host.name.clone(),
                    online: false,
                    error: Some(e),
                    containers: 0,
                    running: 0,
                    stopped: 0,
                    paused: 0,
                    images: 0,
                    cpus: 0,
                    mem_total: 0,
                    docker_version: None,
                    os: None,
                },
            }
        }
    });

    Ok(join_all(futures).await)
}

#[tauri::command]
async fn get_containers(app: tauri::AppHandle, host_id: String) -> Result<Vec<ContainerDto>, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::list_containers(&docker).await
}

#[tauri::command]
async fn get_images(app: tauri::AppHandle, host_id: String) -> Result<Vec<ImageDto>, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::list_images(&docker).await
}

#[tauri::command]
async fn get_volumes(app: tauri::AppHandle, host_id: String) -> Result<Vec<VolumeDto>, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::list_volumes(&docker).await
}

#[tauri::command]
async fn get_networks(app: tauri::AppHandle, host_id: String) -> Result<Vec<NetworkDto>, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::list_networks(&docker).await
}

#[tauri::command]
async fn container_action(
    app: tauri::AppHandle,
    host_id: String,
    id: String,
    action: String,
) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::container_action(&docker, &id, &action).await
}

#[tauri::command]
async fn container_logs(
    app: tauri::AppHandle,
    host_id: String,
    id: String,
    tail: Option<String>,
) -> Result<String, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::container_logs(&docker, &id, tail.as_deref().unwrap_or("200")).await
}

#[tauri::command]
async fn inspect_container(
    app: tauri::AppHandle,
    host_id: String,
    id: String,
) -> Result<serde_json::Value, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::inspect_container(&docker, &id).await
}

#[tauri::command]
async fn container_stats(app: tauri::AppHandle, host_id: String) -> Result<Vec<StatDto>, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::container_stats(&docker).await
}

#[tauri::command]
async fn recent_events(app: tauri::AppHandle, host_id: String) -> Result<Vec<EventDto>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::recent_events(&docker, now - 6 * 3600, now).await
}

#[tauri::command]
async fn system_df(app: tauri::AppHandle, host_id: String) -> Result<DfDto, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::system_df(&docker).await
}

#[tauri::command]
async fn prune(app: tauri::AppHandle, host_id: String, kind: String) -> Result<PruneResult, String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::prune(&docker, &kind).await
}

#[tauri::command]
async fn pull_image(app: tauri::AppHandle, host_id: String, image: String) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::pull_image(&docker, &image).await
}

#[tauri::command]
async fn remove_container(app: tauri::AppHandle, host_id: String, id: String) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::remove_container(&docker, &id).await
}

#[tauri::command]
async fn remove_image(app: tauri::AppHandle, host_id: String, id: String) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::remove_image(&docker, &id).await
}

#[tauri::command]
async fn remove_volume(app: tauri::AppHandle, host_id: String, name: String) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::remove_volume(&docker, &name).await
}

#[tauri::command]
async fn remove_network(app: tauri::AppHandle, host_id: String, id: String) -> Result<(), String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::remove_network(&docker, &id).await
}

#[tauri::command]
async fn exec_run(
    app: tauri::AppHandle,
    host_id: String,
    id: String,
    cmd: String,
) -> Result<String, String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::exec_run(&docker, &id, &cmd).await
}

#[tauri::command]
async fn deploy_container(
    app: tauri::AppHandle,
    host_id: String,
    spec: DeploySpec,
    autostart: bool,
) -> Result<String, String> {
    ensure_writable(&app.state::<AppState>())?;
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::create_container(&docker, spec, autostart).await
}

#[tauri::command]
async fn container_config(
    app: tauri::AppHandle,
    host_id: String,
    id: String,
) -> Result<DeploySpec, String> {
    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    docker::container_config(&docker, &id).await
}

/// Run a `docker compose` action on a detected stack. Local host only: the
/// compose files live on the machine running DockOne.
#[tauri::command]
async fn compose_action(
    app: tauri::AppHandle,
    host_id: String,
    project: String,
    action: String,
) -> Result<String, String> {
    ensure_writable(&app.state::<AppState>())?;

    let endpoint = {
        let st = app.state::<AppState>();
        let d = st.data.lock().map_err(|_| "stato bloccato")?;
        d.hosts
            .iter()
            .find(|h| h.id == host_id)
            .map(|h| h.endpoint.clone())
            .ok_or_else(|| format!("host «{host_id}» non trovato"))?
    };
    let local = endpoint.is_empty() || endpoint == "local" || endpoint.starts_with("unix://");
    if !local {
        return Err("Compose è disponibile solo sull'host locale in questa versione".into());
    }

    let docker = resolve(&app.state::<AppState>(), &host_id).await?;
    let (files, workdir) = docker::compose_meta(&docker, &project).await?;

    let mut args: Vec<String> = vec!["compose".into(), "-p".into(), project.clone()];
    for f in &files {
        args.push("-f".into());
        args.push(f.clone());
    }
    match action.as_str() {
        "up" => {
            args.push("up".into());
            args.push("-d".into());
        }
        "down" => args.push("down".into()),
        "stop" => args.push("stop".into()),
        "start" => args.push("start".into()),
        "restart" => args.push("restart".into()),
        other => return Err(format!("azione compose sconosciuta: {other}")),
    }

    let mut cmd = tokio::process::Command::new("docker");
    cmd.args(&args);
    if let Some(wd) = &workdir {
        cmd.current_dir(wd);
    }
    let out = cmd
        .output()
        .await
        .map_err(|e| format!("«docker compose» non disponibile: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if out.status.success() {
        Ok(format!("{stdout}{stderr}").trim().to_string())
    } else {
        Err(format!("{stderr}{stdout}").trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// alerts watcher
// ---------------------------------------------------------------------------

/// Background loop: poll every host on the configured interval and fire a
/// desktop notification when a container transitions into a bad state
/// (crash/exit/dead/unhealthy). The first observation only seeds the baseline.
async fn alert_loop(app: AppHandle) {
    loop {
        let (enabled, secs) = {
            let st = app.state::<AppState>();
            let d = st.data.lock();
            d.map(|d| (d.settings.alerts_enabled, d.settings.alert_poll_secs))
                .unwrap_or((false, 30))
        };
        let wait = if secs < 5 { 5 } else { secs };
        tokio::time::sleep(std::time::Duration::from_secs(wait)).await;

        if !enabled {
            continue;
        }

        let hosts = {
            let st = app.state::<AppState>();
            let hosts = st.data.lock().map(|d| d.hosts.clone());
            match hosts {
                Ok(h) => h,
                Err(_) => continue,
            }
        };

        for host in hosts {
            let docker = match resolve(&app.state::<AppState>(), &host.id).await {
                Ok(d) => d,
                Err(_) => continue,
            };
            let Ok(list) = docker::list_containers(&docker).await else {
                continue;
            };

            for c in list {
                let key = format!("{}/{}", host.id, c.name);
                let current = if c.health == "unhealthy" {
                    "unhealthy".to_string()
                } else {
                    c.state.clone()
                };

                // Read the previous state and record the new one atomically.
                let previous = {
                    let st = app.state::<AppState>();
                    let prev = st.last_states.lock().ok().and_then(|m| m.get(&key).cloned());
                    if let Ok(mut m) = st.last_states.lock() {
                        m.insert(key.clone(), current.clone());
                    }
                    prev
                };

                let Some(previous) = previous else {
                    continue; // first sight: just seed the baseline
                };
                if previous == current {
                    continue;
                }

                let bad = matches!(current.as_str(), "exited" | "dead" | "unhealthy");
                if bad {
                    let title = format!("⚠ {} · {}", c.name, host.name);
                    let body = match current.as_str() {
                        "unhealthy" => "Health check fallito".to_string(),
                        _ => format!("Il container è passato a «{current}»"),
                    };
                    let _ = app.notification().builder().title(title).body(body).show();
                    let _ = tauri::Emitter::emit(
                        &app,
                        "dockone-alert",
                        serde_json::json!({
                            "host": host.name,
                            "container": c.name,
                            "state": current,
                        }),
                    );
                }
            }
        }
    }
}

fn slug(s: &str) -> String {
    let base: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-').to_string();
    if base.is_empty() {
        "host".into()
    } else {
        base
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            list_hosts,
            test_host,
            add_host,
            remove_host,
            toggle_host_favorite,
            get_settings,
            save_settings,
            export_config,
            import_config,
            toggle_favorite_container,
            dashboard,
            get_containers,
            get_images,
            get_volumes,
            get_networks,
            container_action,
            container_logs,
            inspect_container,
            container_stats,
            recent_events,
            system_df,
            prune,
            pull_image,
            remove_container,
            remove_image,
            remove_volume,
            remove_network,
            exec_run,
            deploy_container,
            container_config,
            compose_action,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(alert_loop(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

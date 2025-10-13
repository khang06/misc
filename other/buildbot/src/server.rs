use std::{
    io::Cursor,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use anyhow::Context;
use axum::{
    Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::StatusCode,
    routing::get,
};
use axum_client_ip::CfConnectingIp;
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
    typed_header::TypedHeaderRejection,
};
use chrono::Utc;
use serde::Serialize;
use sevenz_rust2::{ArchiveReader, Password};
use tokio::{fs::File, io::AsyncWriteExt};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{Config, verifier::Verifier};

#[derive(Serialize)]
struct UpdateFile {
    pub filename: String,
    pub file_hash: String,
    pub filesize: usize,
    pub url_full: String,
}

struct ArchiveFile {
    pub filename: String,
    pub data: Vec<u8>,
}

struct AppState<V: Verifier> {
    pub config: Arc<Config>,
    pub verifier: V,
}
type SharedState<V> = Arc<AppState<V>>;

pub async fn run<V: Verifier>(config: Arc<Config>, ctx: V) {
    let state = Arc::new(AppState {
        config,
        verifier: ctx,
    });
    let app = Router::new()
        .route("/", get(gtfo))
        .route("/{*wildcard}", get(gtfo))
        .route("/ciupload/{commit}", get(gtfo).put(handle_upload))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            250 * 1024 * 1024, // 250mb
        ))
        .with_state(Arc::clone(&state));

    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr)
        .await
        .expect("listening");
    info!("Server listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn gtfo() -> (StatusCode, String) {
    (StatusCode::FORBIDDEN, "get out".to_owned())
}

async fn validate_upload(
    config: &Config,
    ip: IpAddr,
    auth: Result<TypedHeader<Authorization<Basic>>, TypedHeaderRejection>,
    commit: &str,
    data: &Bytes,
) -> bool {
    // Verify CI password
    let auth = if let Ok(auth) = auth {
        auth.0
    } else {
        warn!("PUT request from {ip} had no auth");
        return false;
    };
    if auth.username() != "ci" || auth.password() != config.password {
        warn!("PUT request from {ip} had bad auth");
        return false;
    }

    // Verify if the path is a valid Git commit
    if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        warn!("PUT request from {ip} had invalid commit hash: {commit}");
        return false;
    }

    // Verify file size
    if data.is_empty() {
        warn!("PUT request from {ip} had no file");
        return false;
    }

    true
}

async fn handle_upload<V: Verifier>(
    State(state): State<SharedState<V>>,
    ConnectInfo(connect_info): ConnectInfo<SocketAddr>,
    cf_ip: Result<CfConnectingIp, axum_client_ip::Rejection>,
    auth: Result<TypedHeader<Authorization<Basic>>, TypedHeaderRejection>,
    Path(commit): Path<String>,
    data: Bytes,
) -> (StatusCode, String) {
    // Pull IP from Cloudflare header
    // This is behind nginx with only CF IPs whitelisted, so this should be fine
    let ip = if let Ok(cf_ip) = cf_ip {
        cf_ip.0
    } else {
        connect_info.ip()
    };

    if !validate_upload(&state.config, ip, auth, &commit, &data).await {
        return gtfo().await;
    }

    let req_id = Uuid::new_v4();

    // Run the rest of the logic separately to not stall CI
    tokio::spawn(async move {
        if let Err(err) = process_build(&state, req_id, ip, &commit, data).await {
            report_error(&state, &format!("{err:#}")).await;
        }
    });

    (StatusCode::CREATED, req_id.to_string())
}

async fn process_build<V: Verifier>(
    state: &SharedState<V>,
    req_id: Uuid,
    ip: IpAddr,
    commit: &str,
    data: Bytes,
) -> anyhow::Result<()> {
    // Make sure the build is approved for upload
    let approval = state
        .verifier
        .ask_for_approval(req_id, ip, commit, data.len())
        .await;
    if approval != Some(true) {
        return Ok(());
    }

    info!("Extracting archive");
    let data_temp = data.clone();
    let files = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<ArchiveFile>> {
        let mut archive = ArchiveReader::new(Cursor::new(data_temp.as_ref()), Password::empty())
            .context("opening archive")?;
        let mut files: Vec<ArchiveFile> = Vec::new();

        archive
            .for_each_entries(|entry, reader| {
                let file = ArchiveFile {
                    filename: entry.name().to_owned(),
                    data: {
                        let mut buf = Vec::with_capacity(entry.size() as usize);
                        reader.read_to_end(&mut buf)?;
                        buf
                    },
                };
                files.push(file);

                Ok(true)
            })
            .context("reading archive entries")?;

        Ok(files)
    })
    .await
    .context("awaiting archive extraction")?
    .context("extracting archive")?;

    info!("Recreating build output directory");
    if let Err(err) = tokio::fs::remove_dir_all(&state.config.upload_path).await
        && err.kind() != tokio::io::ErrorKind::NotFound
    {
        Err(err).context("removing old output directory")?;
    }
    tokio::fs::create_dir(&state.config.upload_path)
        .await
        .context("creating output directory")?;

    info!("Writing full archive");
    let archive_name = format!(
        "{}-{}-{}.7z",
        &state.config.archive_prefix,
        Utc::now().format("%Y%m%d"),
        &commit[..6]
    );
    tokio::fs::write(state.config.upload_path.join(&archive_name), &data)
        .await
        .context("writing archive")?;

    info!("Writing extracted files");
    let mut file_metadata: Vec<UpdateFile> = Vec::new();

    for file in files {
        info!("Processing {:?}", file.filename);
        let digest = md5::compute(&file.data);
        let hash_str = format!("{digest:x?}");

        tokio::fs::write(state.config.upload_path.join(&hash_str), &file.data)
            .await
            .context("writing file")?;

        let url_full = format!("{}/{hash_str}", state.config.base_url);
        file_metadata.push(UpdateFile {
            filename: file.filename,
            file_hash: hash_str,
            filesize: file.data.len(),
            url_full,
        });
    }

    info!("Writing file list");
    let mut file_list = File::create(state.config.upload_path.join("files.json"))
        .await
        .context("creating file list")?;
    let serialized = serde_json::to_string(&file_metadata).context("serializing file list")?;
    file_list
        .write_all(serialized.as_bytes())
        .await
        .context("writing file list")?;

    info!("Done!");
    state.verifier.report_success(&archive_name).await;
    Ok(())
}

async fn report_error<V: Verifier>(state: &SharedState<V>, msg: &str) {
    error!("Error while committing build: {msg}");
    state.verifier.report_error(msg).await;
}

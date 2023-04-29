mod asset;
mod config;
mod data;
mod error;
mod fragments;
mod pages;
mod session;

use crate::asset::serve_asset;
pub use crate::config::Config;
use crate::config::Listen;
use crate::data::demo::{Demo, Filter, ListDemo};
use crate::data::maps::map_list;
use crate::data::steam_id::SteamId;
use crate::data::user::User;
use crate::fragments::demo_list::DemoList;
use crate::pages::about::AboutPage;
use crate::pages::api::ApiPage;
use crate::pages::demo::{ClassIconsStyle, DemoPage};
use crate::pages::index::{DemoListScript, Index};
use crate::pages::profile::Profile;
use crate::pages::upload::{UploadPage, UploadScript};
use crate::pages::uploads::Uploads;
use crate::pages::viewer::{ParseWorkerScript, ParserWasm, ViewerPage, ViewerScript, ViewerStyle};
use crate::pages::{render, GlobalStyle};
use crate::session::{SessionData, COOKIE_NAME};
use async_session::{MemoryStore, Session, SessionStore};
use axum::extract::{MatchedPath, Path, Query, RawQuery};
use axum::headers::Cookie;
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::IntoResponse;
use axum::{extract::State, routing::get, Router, Server, TypedHeader};
use demostf_build::Asset;
pub use error::Error;
use hyperlocal::UnixServerExt;
use maud::{Markup, Render};
use sqlx::PgPool;
use std::env::{args, var};
use std::fs::{remove_file, set_permissions, Permissions};
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use steam_openid::SteamOpenId;
use tower_http::trace::TraceLayer;
use tracing::{error, info, info_span};
use tracing_subscriber::{
    fmt::layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

struct App {
    connection: PgPool,
    openid: SteamOpenId,
    api: String,
    pub session_store: MemoryStore,
}

#[derive(Asset)]
#[asset(source = "images/logo.png", url = "/images/logo.png")]
struct LogoPng;
#[derive(Asset)]
#[asset(source = "images/logo.svg", url = "/images/logo.svg")]
struct LogoSvg;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(layer().with_filter(EnvFilter::new(
            var("RUST_LOG").unwrap_or_else(|_| "warn,frontend=info".into()),
        )))
        .try_init()
        .expect("Failed to init tracing");

    let config = args().skip(1).next().expect("no config file provided");
    let config = Config::load(&config)?;
    let connection = config.database.connect().await?;

    let session_store = MemoryStore::new();

    let state = Arc::new(App {
        connection,
        openid: SteamOpenId::new(&config.site.url, "/login/callback")
            .expect("invalid steam login url"),
        api: config.site.api,
        session_store: session_store.clone(),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/uploads/:uploader", get(uploads))
        .route("/profiles/:uploader", get(profiles))
        .route(GlobalStyle::route(), get(serve_asset::<GlobalStyle>))
        .route(
            ClassIconsStyle::route(),
            get(serve_asset::<ClassIconsStyle>),
        )
        .route(UploadScript::route(), get(serve_asset::<UploadScript>))
        .route(DemoListScript::route(), get(serve_asset::<DemoListScript>))
        .route(ViewerScript::route(), get(serve_asset::<ViewerScript>))
        .route(ViewerStyle::route(), get(serve_asset::<ViewerStyle>))
        .route(
            ParseWorkerScript::route(),
            get(serve_asset::<ParseWorkerScript>),
        )
        .route(ParserWasm::route(), get(serve_asset::<ParserWasm>))
        .route(LogoPng::route(), get(serve_asset::<LogoPng>))
        .route(LogoSvg::route(), get(serve_asset::<LogoSvg>))
        .route("/fragments/demo-list", get(demo_list))
        .route("/about", get(about))
        .route("/api", get(api))
        .route("/login/callback", get(login_callback))
        .route("/login", get(login))
        .route("/logout", get(logout))
        .route("/upload", get(upload))
        .route("/viewer", get(viewer))
        .route("/viewer/:id", get(viewer))
        .route("/:id", get(demo))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    matched_path,
                    some_other_field = tracing::field::Empty,
                )
            }),
        )
        .fallback(handler_404)
        .with_state(state);
    let service = app.into_make_service();

    match config.listen {
        Listen::Tcp { address, port } => {
            let addr = SocketAddr::from((address, port));
            info!("listening on {}", addr);
            Server::bind(&addr).serve(service).await?;
        }
        Listen::Socket { path } => {
            info!("listening on {}", path.display());
            if path.exists() {
                remove_file(&path)?;
            }
            let socket = Server::bind_unix(&path)?;
            set_permissions(&path, Permissions::from_mode(0o666))?;

            socket.serve(service).await?;
        }
    }

    Ok(())
}

#[axum::debug_handler]
async fn index(
    State(app): State<Arc<App>>,
    session: SessionData,
    filter: Option<Query<Filter>>,
) -> Result<Markup> {
    let filter = filter.map(|filter| filter.0).unwrap_or_default();
    let demos = ListDemo::list(&app.connection, filter).await?;
    let maps: Vec<_> = map_list(&app.connection).await?.collect();
    Ok(render(
        Index {
            demos: &demos,
            maps: &maps,
            api: &app.api,
        },
        session,
    ))
}

async fn about(State(_app): State<Arc<App>>, session: SessionData) -> Result<Markup> {
    Ok(render(
        AboutPage {
            key: session.token(),
        },
        session,
    ))
}

async fn api(State(app): State<Arc<App>>, session: SessionData) -> Result<Markup> {
    Ok(render(
        ApiPage {
            steam_id: session.steam_id().unwrap_or(SteamId::Id(76561198024494988)),
            api_base: &app.api,
        },
        session,
    ))
}

async fn demo(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    session: SessionData,
) -> Result<Markup> {
    let id = id.parse().map_err(|_| Error::NotFound)?;
    let demo = Demo::by_id(&app.connection, id)
        .await?
        .ok_or(Error::NotFound)?;
    Ok(render(DemoPage { demo }, session))
}

async fn login_callback(
    State(app): State<Arc<App>>,
    RawQuery(query): RawQuery,
) -> Result<impl IntoResponse> {
    let query = query.as_deref().unwrap_or_default();
    let steam_id = app.openid.verify(query).await.map_err(|e| {
        error!("{e:?}");
        Error::SteamAuth
    })?;
    let steam_id = SteamId::new(steam_id);
    let user = User::get(&app.connection, steam_id).await?;
    let mut session = Session::new();
    session
        .insert("user", user)
        .expect("failed to serialize user");
    let cookie = app
        .session_store
        .store_session(session)
        .await?
        .unwrap_or_default();
    Ok((
        StatusCode::FOUND,
        [
            (
                SET_COOKIE,
                HeaderValue::from_str(&format!(
                    "{}={}; HttpOnly; SameSite=Lax; Path=/",
                    COOKIE_NAME, cookie
                ))
                .expect("invalid cookie"),
            ),
            (LOCATION, HeaderValue::from_static("/")),
        ],
    ))
}

async fn login(State(app): State<Arc<App>>) -> impl IntoResponse {
    (
        StatusCode::FOUND,
        [(
            LOCATION,
            HeaderValue::from_str(app.openid.get_redirect_url()).unwrap(),
        )],
    )
}

async fn logout(
    State(app): State<Arc<App>>,
    cookie: Option<TypedHeader<Cookie>>,
) -> impl IntoResponse {
    if let Some(session_cookie) = cookie.as_deref().and_then(|cookie| cookie.get(COOKIE_NAME)) {
        if let Ok(Some(cookie)) = app.session_store.load_session(session_cookie.into()).await {
            let _ = app.session_store.destroy_session(cookie);
        }
    }
    (
        StatusCode::FOUND,
        [
            (
                SET_COOKIE,
                HeaderValue::from_str(&format!(
                    "{}=; HttpOnly; SameSite=Lax; expires=Thu, 01 Jan 1970 00:00:00 GMT",
                    COOKIE_NAME
                ))
                .expect("invalid cookie"),
            ),
            (LOCATION, HeaderValue::from_str("/").unwrap()),
        ],
    )
}

async fn upload(State(app): State<Arc<App>>, session: SessionData) -> impl IntoResponse {
    if let Some(token) = session.token() {
        render(
            UploadPage {
                key: token.as_str(),
                api: app.api.as_str(),
            },
            session,
        )
        .into_response()
    } else {
        (
            StatusCode::FOUND,
            [(LOCATION, HeaderValue::from_str("/").unwrap())],
        )
            .into_response()
    }
}

#[axum::debug_handler]
async fn demo_list(State(app): State<Arc<App>>, filter: Option<Query<Filter>>) -> Result<Markup> {
    let filter = filter.map(|filter| filter.0).unwrap_or_default();
    let demos = ListDemo::list(&app.connection, filter).await?;
    Ok(DemoList { demos: &demos }.render())
}

#[axum::debug_handler]
async fn uploads(
    State(app): State<Arc<App>>,
    session: SessionData,
    filter: Option<Query<Filter>>,
    Path(uploader): Path<SteamId>,
) -> Result<Markup> {
    let mut filter = filter.map(|filter| filter.0).unwrap_or_default();
    filter.uploader = Some(uploader.clone());

    let demos = ListDemo::list(&app.connection, filter).await?;
    let maps: Vec<_> = map_list(&app.connection).await?.collect();
    let user = User::get(&app.connection, uploader)
        .await
        .map_err(|_| Error::NotFound)?;
    Ok(render(
        Uploads {
            user,
            demos: &demos,
            maps: &maps,
            api: &app.api,
        },
        session,
    ))
}

#[axum::debug_handler]
async fn profiles(
    State(app): State<Arc<App>>,
    session: SessionData,
    filter: Option<Query<Filter>>,
    Path(profile): Path<SteamId>,
) -> Result<Markup> {
    let mut filter = filter.map(|filter| filter.0).unwrap_or_default();
    filter.players.push(profile.clone());

    let demos = ListDemo::list(&app.connection, filter).await?;
    let maps: Vec<_> = map_list(&app.connection).await?.collect();
    let user = User::get(&app.connection, profile)
        .await
        .map_err(|_| Error::NotFound)?;
    Ok(render(
        Profile {
            user,
            demos: &demos,
            maps: &maps,
            api: &app.api,
        },
        session,
    ))
}

async fn viewer(
    State(app): State<Arc<App>>,
    id: Option<Path<String>>,
    session: SessionData,
) -> Result<Markup> {
    let demo = if let Some(Path(id)) = id {
        let id = id.parse().map_err(|_| Error::NotFound)?;
        Some(
            Demo::by_id(&app.connection, id)
                .await?
                .ok_or(Error::NotFound)?,
        )
    } else {
        None
    };
    Ok(render(ViewerPage { demo }, session))
}

async fn handler_404() -> impl IntoResponse {
    Error::NotFound
}

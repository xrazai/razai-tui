use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use sqlx::PgPool;

use crate::db;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

type HmacSha256 = Hmac<Sha256>;

const ACCESS_TOKEN_KEY: &str = "shopee_access_token";
const REFRESH_TOKEN_KEY: &str = "shopee_refresh_token";
const ACCESS_TOKEN_EXPIRES_AT_KEY: &str = "shopee_access_token_expires_at";
const REFRESH_TOKEN_EXPIRES_AT_KEY: &str = "shopee_refresh_token_expires_at";
const REFRESH_WINDOW_SECONDS: i64 = 10 * 60;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
static CALLBACK_LISTENER_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct ShopeeConfig {
    pub partner_id: i64,
    pub partner_key: String,
    pub shop_id: i64,
    pub api_host: String,
    pub redirect_url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ShopeeTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: i64,
    pub refresh_token_expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    error: Option<String>,
    message: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expire_in: Option<i64>,
}

#[derive(Debug, Serialize)]
struct RefreshTokenRequest<'a> {
    partner_id: i64,
    refresh_token: &'a str,
    shop_id: i64,
}

#[derive(Debug, Serialize)]
struct CodeTokenRequest<'a> {
    code: &'a str,
    partner_id: i64,
}

pub async fn startup_status(pool: Option<&PgPool>) -> String {
    match ensure_connected(pool).await {
        Ok(_) => String::from("Shopee conectada"),
        Err(error) => format!("Shopee desconectada: {error}"),
    }
}

pub fn ensure_ngrok_tunnel() -> String {
    let callback_addr = callback_addr();
    let port = callback_addr
        .rsplit_once(':')
        .map(|(_, port)| port)
        .unwrap_or("8910");

    if let Some(public_url) = current_ngrok_public_url() {
        persist_public_urls(&public_url);
        return format!("Ngrok ativo: {public_url}");
    }

    let start_result = Command::new("ngrok")
        .arg("http")
        .arg(port)
        .creation_flags(0x08000000)
        .spawn();
    if let Err(error) = start_result {
        return format!("Ngrok nao iniciado automaticamente: {error}");
    }

    for _ in 0..15 {
        thread::sleep(Duration::from_millis(500));
        if let Some(public_url) = current_ngrok_public_url() {
            persist_public_urls(&public_url);
            return format!("Ngrok iniciado: {public_url}");
        }
    }

    String::from("Ngrok iniciado, mas URL publica ainda nao foi detectada")
}

pub async fn online_stock_summary(pool: Option<&PgPool>) -> String {
    let config = match ShopeeConfig::from_env() {
        Ok(config) => config,
        Err(error) => return format!("Shopee desconectada: {error}"),
    };
    let tokens = match ensure_connected_with_config(pool, &config).await {
        Ok(tokens) => tokens,
        Err(error) => return format!("Shopee desconectada: {error}"),
    };

    match get_item_list(&config, &tokens).await {
        Ok(count) => format!("Shopee conectada. Estoque Online retornou {count} anuncios."),
        Err(error) if error.is_token_error() => match refresh_tokens(pool, &config, &tokens).await {
            Ok(tokens) => match get_item_list(&config, &tokens).await {
                Ok(count) => format!("Shopee conectada. Estoque Online retornou {count} anuncios."),
                Err(error) => format!("Shopee: falha ao consultar estoque online: {error}"),
            },
            Err(error) => format!("Shopee precisa reautorizar: {error}"),
        },
        Err(error) => format!("Shopee: falha ao consultar estoque online: {error}"),
    }
}

pub async fn create_listing_status(pool: Option<&PgPool>) -> String {
    match ensure_connected(pool).await {
        Ok(_) => String::from(
            "Shopee conectada. Criar anuncio preparado; validar categorias, atributos, imagens e estoque BR antes de publicar.",
        ),
        Err(error) => format!("Shopee desconectada: {error}"),
    }
}

pub async fn exchange_code(pool: Option<&PgPool>, code: &str) -> Result<(), ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    let path = "/api/v2/auth/token/get";
    let timestamp = now_timestamp();
    let sign = public_sign(config.partner_id, path, timestamp, &config.partner_key);
    let url = signed_public_url(&config, path, timestamp, &sign);
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .post(url)
        .json(&CodeTokenRequest {
            code,
            partner_id: config.partner_id,
        })
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;

    let token_response = parse_token_response(response).await?;
    persist_tokens(pool, &token_response).await
}

pub fn start_callback_listener(pool: Option<PgPool>) -> Result<String, ShopeeError> {
    dotenvy::dotenv_override().ok();
    let config = ShopeeConfig::from_env()?;
    let addr = callback_addr();
    let auth_url = authorization_url(&config);
    if CALLBACK_LISTENER_STARTED.load(Ordering::SeqCst) {
        return Ok(format!(
            "Callback Shopee ja ativo em http://{addr}. Push URL: /shopee/push. Redirect OAuth: /shopee/callback. Abra: {auth_url}"
        ));
    }
    let listener = TcpListener::bind(&addr)
        .map_err(|error| ShopeeError::Http(format!("falha ao abrir callback local {addr}: {error}")))?;
    CALLBACK_LISTENER_STARTED.store(true, Ordering::SeqCst);

    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let body = match handle_callback_stream(pool.as_ref(), &mut stream) {
                CallbackResult::TokenSaved => {
                    "Shopee conectada. Tokens salvos. Pode voltar ao Razai TUI.".to_string()
                }
                CallbackResult::WebhookAccepted => {
                    "Shopee webhook recebido.".to_string()
                }
                CallbackResult::Error(error) => {
                    format!("Falha ao conectar Shopee: {error}")
                }
            };
            let _ = write_http_response(&mut stream, &body);
        }
    });

    Ok(format!(
        "Callback Shopee ativo em http://{addr}. Push URL: /shopee/push. Redirect OAuth: /shopee/callback. Abra: {auth_url}"
    ))
}

async fn ensure_connected(pool: Option<&PgPool>) -> Result<ShopeeTokens, ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    ensure_connected_with_config(pool, &config).await
}

async fn ensure_connected_with_config(
    pool: Option<&PgPool>,
    config: &ShopeeConfig,
) -> Result<ShopeeTokens, ShopeeError> {
    let mut tokens = load_tokens(pool).await;
    if tokens.access_token.is_empty() || tokens.refresh_token.is_empty() {
        return Err(ShopeeError::MissingTokens(config.authorization_hint()));
    }

    let now = now_timestamp();
    if tokens.refresh_token_expires_at > 0 && tokens.refresh_token_expires_at <= now {
        return Err(ShopeeError::RefreshExpired);
    }
    if should_refresh(tokens.access_token_expires_at, now) {
        tokens = refresh_tokens(pool, config, &tokens).await?;
    }

    Ok(tokens)
}

async fn refresh_tokens(
    pool: Option<&PgPool>,
    config: &ShopeeConfig,
    current: &ShopeeTokens,
) -> Result<ShopeeTokens, ShopeeError> {
    if current.refresh_token.trim().is_empty() {
        return Err(ShopeeError::MissingRefreshToken);
    }

    let path = "/api/v2/auth/access_token/get";
    let timestamp = now_timestamp();
    let sign = public_sign(config.partner_id, path, timestamp, &config.partner_key);
    let url = signed_public_url(config, path, timestamp, &sign);
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .post(url)
        .json(&RefreshTokenRequest {
            partner_id: config.partner_id,
            refresh_token: &current.refresh_token,
            shop_id: config.shop_id,
        })
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;

    let token_response = parse_token_response(response).await?;
    persist_tokens(pool, &token_response).await?;
    Ok(token_response)
}

async fn get_item_list(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
) -> Result<usize, ShopeeError> {
    let path = "/api/v2/product/get_item_list";
    let timestamp = now_timestamp();
    let sign = shop_sign(
        config.partner_id,
        path,
        timestamp,
        &tokens.access_token,
        config.shop_id,
        &config.partner_key,
    );
    let url = format!(
        "{}{}?partner_id={}&timestamp={}&access_token={}&shop_id={}&sign={}&offset=0&page_size=20&item_status=NORMAL",
        config.api_host, path, config.partner_id, timestamp, tokens.access_token, config.shop_id, sign
    );

    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .get(url)
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;

    parse_item_count(response).await
}

async fn parse_item_count(response: reqwest::Response) -> Result<usize, ShopeeError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;
    if !status.is_success() {
        return Err(ShopeeError::Api(format!("HTTP {status}: {body}")));
    }
    let value = serde_json::from_str::<Value>(&body)
        .map_err(|error| ShopeeError::Api(format!("Resposta invalida: {error}")))?;
    ensure_no_api_error(&value)?;
    Ok(value
        .pointer("/response/item")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0))
}

async fn parse_token_response(response: reqwest::Response) -> Result<ShopeeTokens, ShopeeError> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;
    if status != StatusCode::OK {
        return Err(ShopeeError::Api(format!("HTTP {status}: {text}")));
    }
    let token_response = serde_json::from_str::<TokenResponse>(&text)
        .map_err(|error| ShopeeError::Api(format!("Resposta de token invalida: {error}")))?;
    if let Some(error) = token_response.error.as_deref().filter(|error| !error.is_empty()) {
        return Err(ShopeeError::Api(format!(
            "{}: {}",
            error,
            token_response.message.unwrap_or_default()
        )));
    }
    let now = now_timestamp();
    Ok(ShopeeTokens {
        access_token: token_response
            .access_token
            .ok_or(ShopeeError::Api(String::from("access_token ausente")))?,
        refresh_token: token_response
            .refresh_token
            .ok_or(ShopeeError::Api(String::from("refresh_token ausente")))?,
        access_token_expires_at: now + token_response.expire_in.unwrap_or(14_400),
        refresh_token_expires_at: now + REFRESH_TOKEN_TTL_SECONDS,
    })
}

fn ensure_no_api_error(value: &Value) -> Result<(), ShopeeError> {
    let error = value.get("error").and_then(Value::as_str).unwrap_or("");
    if error.is_empty() {
        return Ok(());
    }
    let message = value.get("message").and_then(Value::as_str).unwrap_or("");
    Err(ShopeeError::Api(format!("{error}: {message}")))
}

async fn load_tokens(pool: Option<&PgPool>) -> ShopeeTokens {
    let mut tokens = ShopeeTokens::from_env();
    if let Some(pool) = pool {
        if let Ok(Some(value)) = db::get_config(pool, ACCESS_TOKEN_KEY).await {
            tokens.access_token = value;
        }
        if let Ok(Some(value)) = db::get_config(pool, REFRESH_TOKEN_KEY).await {
            tokens.refresh_token = value;
        }
        if let Ok(Some(value)) = db::get_config(pool, ACCESS_TOKEN_EXPIRES_AT_KEY).await {
            tokens.access_token_expires_at = value.parse().unwrap_or(0);
        }
        if let Ok(Some(value)) = db::get_config(pool, REFRESH_TOKEN_EXPIRES_AT_KEY).await {
            tokens.refresh_token_expires_at = value.parse().unwrap_or(0);
        }
    }
    tokens
}

async fn persist_tokens(pool: Option<&PgPool>, tokens: &ShopeeTokens) -> Result<(), ShopeeError> {
    if let Some(pool) = pool {
        db::set_config(pool, ACCESS_TOKEN_KEY, &tokens.access_token).await?;
        db::set_config(pool, REFRESH_TOKEN_KEY, &tokens.refresh_token).await?;
        db::set_config(
            pool,
            ACCESS_TOKEN_EXPIRES_AT_KEY,
            &tokens.access_token_expires_at.to_string(),
        )
        .await?;
        db::set_config(
            pool,
            REFRESH_TOKEN_EXPIRES_AT_KEY,
            &tokens.refresh_token_expires_at.to_string(),
        )
        .await?;
    }
    update_dotenv(tokens).map_err(ShopeeError::Env)
}

fn update_dotenv(tokens: &ShopeeTokens) -> Result<(), String> {
    let access_expires_at = tokens.access_token_expires_at.to_string();
    let refresh_expires_at = tokens.refresh_token_expires_at.to_string();
    update_env_key_values(&[
        ("SHOPEE_ACCESS_TOKEN", tokens.access_token.as_str()),
        ("SHOPEE_REFRESH_TOKEN", tokens.refresh_token.as_str()),
        ("SHOPEE_ACCESS_TOKEN_EXPIRES_AT", &access_expires_at),
        ("SHOPEE_REFRESH_TOKEN_EXPIRES_AT", &refresh_expires_at),
    ])
}

fn update_env_key_values(replacements: &[(&str, &str)]) -> Result<(), String> {
    let path = Path::new(".env");
    let original = if path.exists() {
        fs::read_to_string(path).map_err(|error| format!("Falha ao ler .env: {error}"))?
    } else {
        String::new()
    };
    let updated = update_env_contents(&original, replacements);
    fs::write(path, updated).map_err(|error| format!("Falha ao atualizar .env: {error}"))
}

pub fn public_sign(partner_id: i64, path: &str, timestamp: i64, partner_key: &str) -> String {
    sign(
        &format!("{partner_id}{path}{timestamp}"),
        partner_key.as_bytes(),
    )
}

pub fn shop_sign(
    partner_id: i64,
    path: &str,
    timestamp: i64,
    access_token: &str,
    shop_id: i64,
    partner_key: &str,
) -> String {
    sign(
        &format!("{partner_id}{path}{timestamp}{access_token}{shop_id}"),
        partner_key.as_bytes(),
    )
}

fn sign(base: &str, key: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any size");
    mac.update(base.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn signed_public_url(
    config: &ShopeeConfig,
    path: &str,
    timestamp: i64,
    sign: &str,
) -> String {
    format!(
        "{}{}?partner_id={}&timestamp={}&sign={}",
        config.api_host, path, config.partner_id, timestamp, sign
    )
}

fn callback_addr() -> String {
    std::env::var("SHOPEE_CALLBACK_ADDR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from("127.0.0.1:8910"))
}

fn current_ngrok_public_url() -> Option<String> {
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?
        .get("http://127.0.0.1:4040/api/tunnels")
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let value = response.json::<Value>().ok()?;
    value
        .get("tunnels")?
        .as_array()?
        .iter()
        .filter(|tunnel| tunnel.get("proto").and_then(Value::as_str) == Some("https"))
        .find_map(|tunnel| tunnel.get("public_url").and_then(Value::as_str))
        .map(ToString::to_string)
}

fn persist_public_urls(public_url: &str) {
    let redirect_url = format!("{}/shopee/callback", public_url.trim_end_matches('/'));
    let push_url = format!("{}/shopee/push", public_url.trim_end_matches('/'));
    let _ = update_env_key_values(&[
        ("SHOPEE_REDIRECT_URL", redirect_url.as_str()),
        ("SHOPEE_PUSH_WEBHOOK_URL", push_url.as_str()),
    ]);
    unsafe {
        std::env::set_var("SHOPEE_REDIRECT_URL", redirect_url);
        std::env::set_var("SHOPEE_PUSH_WEBHOOK_URL", push_url);
    }
}

fn authorization_url(config: &ShopeeConfig) -> String {
    let path = "/api/v2/shop/auth_partner";
    let timestamp = now_timestamp();
    let sign = public_sign(config.partner_id, path, timestamp, &config.partner_key);
    let redirect = config.redirect_url.as_deref().unwrap_or_default();
    format!(
        "{}{}?partner_id={}&timestamp={}&sign={}&redirect={}",
        config.api_host,
        path,
        config.partner_id,
        timestamp,
        sign,
        percent_encode(redirect)
    )
}

enum CallbackResult {
    TokenSaved,
    WebhookAccepted,
    Error(ShopeeError),
}

fn handle_callback_stream(pool: Option<&PgPool>, stream: &mut TcpStream) -> CallbackResult {
    let mut buffer = [0_u8; 4096];
    let size = match stream.read(&mut buffer) {
        Ok(size) => size,
        Err(error) => {
            return CallbackResult::Error(ShopeeError::Http(format!(
                "falha ao ler callback: {error}"
            )));
        }
    };
    let request = String::from_utf8_lossy(&buffer[..size]);
    let Some(code) = extract_code_from_http_request(&request) else {
        return CallbackResult::WebhookAccepted;
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            return CallbackResult::Error(ShopeeError::Http(format!(
                "falha ao iniciar runtime: {error}"
            )));
        }
    };
    match runtime.block_on(exchange_code(pool, &code)) {
        Ok(()) => CallbackResult::TokenSaved,
        Err(error) => CallbackResult::Error(error),
    }
}

fn write_http_response(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())
}

fn extract_code_from_http_request(request: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    for part in query.split('&') {
        if let Some(code) = part.strip_prefix("code=") {
            return non_empty_owned(&percent_decode(code));
        }
    }
    None
}

fn non_empty_owned(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn percent_decode(value: &str) -> String {
    let mut output = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                output.push(hex);
                index += 3;
                continue;
            }
        }
        output.push(if bytes[index] == b'+' { b' ' } else { bytes[index] });
        index += 1;
    }
    String::from_utf8_lossy(&output).to_string()
}

pub fn should_refresh(access_token_expires_at: i64, now: i64) -> bool {
    access_token_expires_at <= 0 || access_token_expires_at - now <= REFRESH_WINDOW_SECONDS
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn update_env_contents(input: &str, replacements: &[(&str, &str)]) -> String {
    let replacements = replacements
        .iter()
        .copied()
        .collect::<HashMap<&str, &str>>();
    let mut seen = HashMap::<&str, bool>::new();
    let mut lines = input
        .lines()
        .map(|line| {
            let Some((key, _)) = line.split_once('=') else {
                return line.to_string();
            };
            let key = key.trim();
            if let Some(value) = replacements.get(key) {
                seen.insert(key, true);
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();

    for (key, value) in replacements {
        if !seen.contains_key(key) {
            lines.push(format!("{key}={value}"));
        }
    }

    let mut output = lines.join("\n");
    output.push('\n');
    output
}

impl ShopeeConfig {
    pub fn from_env() -> Result<Self, ShopeeError> {
        Ok(Self {
            partner_id: env_i64("SHOPEE_PARTNER_ID")?,
            partner_key: env_string("SHOPEE_PARTNER_KEY")?,
            shop_id: env_i64("SHOPEE_SHOP_ID")?,
            api_host: std::env::var("SHOPEE_API_HOST")
                .unwrap_or_else(|_| String::from("https://partner.shopeemobile.com"))
                .trim_end_matches('/')
                .to_string(),
            redirect_url: std::env::var("SHOPEE_REDIRECT_URL")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        })
    }

    fn authorization_hint(&self) -> String {
        match &self.redirect_url {
            Some(redirect_url) => format!(
                "autorize a loja pela Shopee usando redirect_url={redirect_url} e troque o code por tokens"
            ),
            None => String::from("configure SHOPEE_REDIRECT_URL e autorize a loja na Shopee"),
        }
    }
}

impl ShopeeTokens {
    fn from_env() -> Self {
        Self {
            access_token: std::env::var("SHOPEE_ACCESS_TOKEN").unwrap_or_default(),
            refresh_token: std::env::var("SHOPEE_REFRESH_TOKEN").unwrap_or_default(),
            access_token_expires_at: std::env::var("SHOPEE_ACCESS_TOKEN_EXPIRES_AT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            refresh_token_expires_at: std::env::var("SHOPEE_REFRESH_TOKEN_EXPIRES_AT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        }
    }
}

fn env_string(key: &str) -> Result<String, ShopeeError> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ShopeeError::MissingConfig(key.to_string()))
}

fn env_i64(key: &str) -> Result<i64, ShopeeError> {
    env_string(key)?
        .parse()
        .map_err(|_| ShopeeError::InvalidConfig(key.to_string()))
}

#[derive(Debug)]
pub enum ShopeeError {
    MissingConfig(String),
    InvalidConfig(String),
    MissingTokens(String),
    MissingRefreshToken,
    RefreshExpired,
    Http(String),
    Api(String),
    Env(String),
    Db(sqlx::Error),
}

impl ShopeeError {
    fn is_token_error(&self) -> bool {
        match self {
            ShopeeError::Api(message) => {
                message.contains("access_token")
                    || message.contains("refresh_token")
                    || message.contains("shop_access_expired")
                    || message.contains("error_auth")
            }
            _ => false,
        }
    }
}

impl From<sqlx::Error> for ShopeeError {
    fn from(error: sqlx::Error) -> Self {
        ShopeeError::Db(error)
    }
}

impl std::fmt::Display for ShopeeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShopeeError::MissingConfig(key) => write!(formatter, "{key} nao configurado"),
            ShopeeError::InvalidConfig(key) => write!(formatter, "{key} invalido"),
            ShopeeError::MissingTokens(hint) => write!(formatter, "tokens ausentes; {hint}"),
            ShopeeError::MissingRefreshToken => write!(formatter, "refresh_token ausente"),
            ShopeeError::RefreshExpired => write!(formatter, "refresh_token expirado; reautorize a loja"),
            ShopeeError::Http(error) => write!(formatter, "{error}"),
            ShopeeError::Api(error) => write!(formatter, "{error}"),
            ShopeeError::Env(error) => write!(formatter, "{error}"),
            ShopeeError::Db(error) => write!(formatter, "falha no banco local: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_signature_uses_public_base_string() {
        let sign = public_sign(100, "/api/v2/auth/access_token/get", 1_700_000_000, "secret");
        let expected = super::sign(
            "100/api/v2/auth/access_token/get1700000000",
            b"secret",
        );
        assert_eq!(sign, expected);
    }

    #[test]
    fn shop_signature_uses_access_token_and_shop_id() {
        let sign = shop_sign(
            100,
            "/api/v2/product/get_item_list",
            1_700_000_000,
            "access",
            200,
            "secret",
        );
        let expected = super::sign(
            "100/api/v2/product/get_item_list1700000000access200",
            b"secret",
        );
        assert_eq!(sign, expected);
    }

    #[test]
    fn refreshes_when_token_is_missing_or_close_to_expiry() {
        assert!(should_refresh(0, 100));
        assert!(should_refresh(650, 100));
        assert!(!should_refresh(701, 100));
    }

    #[test]
    fn updates_env_contents_without_printing_or_dropping_other_keys() {
        let updated = update_env_contents(
            "DATABASE_URL=db\nSHOPEE_ACCESS_TOKEN=old\n",
            &[
                ("SHOPEE_ACCESS_TOKEN", "new_access"),
                ("SHOPEE_REFRESH_TOKEN", "new_refresh"),
            ],
        );
        assert!(updated.contains("DATABASE_URL=db"));
        assert!(updated.contains("SHOPEE_ACCESS_TOKEN=new_access"));
        assert!(updated.contains("SHOPEE_REFRESH_TOKEN=new_refresh"));
        assert!(!updated.contains("old"));
    }

    #[test]
    fn api_error_detection_handles_auth_errors() {
        let value = json!({"error": "error_auth", "message": "Invalid access_token."});
        let error = ensure_no_api_error(&value).unwrap_err();
        assert!(error.is_token_error());
    }

    #[test]
    fn callback_request_extracts_code() {
        let request = "GET /shopee/callback?code=abc%20123&shop_id=1 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(
            extract_code_from_http_request(request),
            Some(String::from("abc 123"))
        );
    }

    #[test]
    fn redirect_url_is_percent_encoded() {
        assert_eq!(
            percent_encode("https://example.ngrok.app/shopee/callback"),
            "https%3A%2F%2Fexample.ngrok.app%2Fshopee%2Fcallback"
        );
    }
}

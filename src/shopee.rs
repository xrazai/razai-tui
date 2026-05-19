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
use serde_json::{Value, json};
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

#[derive(Clone, Debug)]
pub struct ShopeeStockOccurrence {
    pub sku: String,
    pub item_id: i64,
    pub model_id: i64,
    pub name: String,
    pub seller_stock: i64,
    pub available_stock: i64,
    pub reserved_stock: i64,
    pub location_id: Option<String>,
    pub multi_location: bool,
}

#[derive(Clone, Debug)]
pub struct ShopeeStockGroup {
    pub sku: String,
    pub occurrences: Vec<ShopeeStockOccurrence>,
    pub total_current_stock: i64,
    pub target_stock: i64,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ShopeeStockSyncResult {
    pub updated: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
}

impl ShopeeStockGroup {
    pub fn can_sync(&self) -> bool {
        self.warning.is_none()
    }

    pub fn target_label(&self) -> &'static str {
        if self.target_stock == 100 {
            "Ativar 100"
        } else {
            "Zerar 0"
        }
    }

    pub fn toggle_target(&mut self) {
        self.target_stock = if self.target_stock == 0 { 100 } else { 0 };
    }
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

pub async fn create_listing_status(pool: Option<&PgPool>) -> String {
    match ensure_connected(pool).await {
        Ok(_) => String::from(
            "Shopee conectada. Criar anuncio: selecione produto local, categoria, atributos obrigatorios, imagens, logistica, estoque, GTIN e fiscal BR; publicar NORMAL apos revisao.",
        ),
        Err(error) => format!("Shopee desconectada: {error}"),
    }
}

pub fn listing_guide_status() -> String {
    String::from(
        "Guia Shopee BR: Criar anuncio exige produto local, categoria, atributos obrigatorios, marca, logistica, preco, peso, dimensoes, estoque, GTIN, imagens e tax_info BR. Consulte docs/ShopeeDocs/SHOPEE_CRIAR_ANUNCIO_BR.md e SHOPEE_ESTOQUE_SKU.md.",
    )
}

pub async fn fetch_online_stock_groups(
    pool: Option<&PgPool>,
) -> Result<Vec<ShopeeStockGroup>, ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    let tokens = ensure_connected_with_config(pool, &config).await?;
    match fetch_online_stock_groups_with_tokens(&config, &tokens).await {
        Ok(groups) => Ok(groups),
        Err(error) if error.is_token_error() => {
            let tokens = refresh_tokens(pool, &config, &tokens).await?;
            fetch_online_stock_groups_with_tokens(&config, &tokens).await
        }
        Err(error) => Err(error),
    }
}

pub async fn sync_stock_groups(
    pool: Option<&PgPool>,
    groups: &[ShopeeStockGroup],
) -> Result<ShopeeStockSyncResult, ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    let tokens = ensure_connected_with_config(pool, &config).await?;
    match sync_stock_groups_with_tokens(&config, &tokens, groups).await {
        Ok(result) => Ok(result),
        Err(error) if error.is_token_error() => {
            let tokens = refresh_tokens(pool, &config, &tokens).await?;
            sync_stock_groups_with_tokens(&config, &tokens, groups).await
        }
        Err(error) => Err(error),
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
    let auth_url = authorization_entry_url(&config);
    if CALLBACK_LISTENER_STARTED.load(Ordering::SeqCst) {
        return Ok(format!(
            "Callback Shopee ja ativo em http://{addr}. Push URL: /shopee/push. Redirect OAuth: /shopee/callback. Abra para conectar: {auth_url}"
        ));
    }
    let listener = TcpListener::bind(&addr).map_err(|error| {
        ShopeeError::Http(format!("falha ao abrir callback local {addr}: {error}"))
    })?;
    CALLBACK_LISTENER_STARTED.store(true, Ordering::SeqCst);

    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            match handle_callback_stream(pool.as_ref(), &mut stream) {
                CallbackResult::TokenSaved => {
                    let _ = write_http_response(
                        &mut stream,
                        "Shopee conectada. Tokens salvos. Pode voltar ao Razai TUI.",
                    );
                }
                CallbackResult::WebhookAccepted => {
                    let _ = write_http_response(&mut stream, "Shopee webhook recebido.");
                }
                CallbackResult::Redirect(location) => {
                    let _ = write_http_redirect(&mut stream, &location);
                }
                CallbackResult::Error(error) => {
                    let _ = write_http_response(
                        &mut stream,
                        &format!("Falha ao conectar Shopee: {error}"),
                    );
                }
            }
        }
    });

    Ok(format!(
        "Callback Shopee ativo em http://{addr}. Push URL: /shopee/push. Redirect OAuth: /shopee/callback. Abra para conectar: {auth_url}"
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

async fn fetch_online_stock_groups_with_tokens(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
) -> Result<Vec<ShopeeStockGroup>, ShopeeError> {
    let item_ids = get_all_item_ids(config, tokens).await?;
    let mut item_infos = Vec::new();
    for chunk in item_ids.chunks(50) {
        item_infos.extend(get_item_base_infos(config, tokens, chunk).await?);
    }

    let mut occurrences = Vec::new();
    for item in item_infos {
        let item_id = item.get("item_id").and_then(Value::as_i64).unwrap_or(0);
        let item_name = item
            .get("item_name")
            .and_then(Value::as_str)
            .unwrap_or("sem nome")
            .to_string();
        let has_model = item
            .get("has_model")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if has_model {
            let models = get_model_list(config, tokens, item_id).await?;
            occurrences.extend(model_occurrences(item_id, &item_name, &models));
        } else if let Some(occurrence) = item_occurrence(&item) {
            occurrences.push(occurrence);
        }
    }

    Ok(group_stock_occurrences(occurrences))
}

async fn sync_stock_groups_with_tokens(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    groups: &[ShopeeStockGroup],
) -> Result<ShopeeStockSyncResult, ShopeeError> {
    let mut result = ShopeeStockSyncResult::default();
    for group in groups {
        if !group.can_sync() {
            result.skipped += group.occurrences.len();
            continue;
        }
        for occurrence in &group.occurrences {
            match update_stock(config, tokens, occurrence, group.target_stock).await {
                Ok(()) => result.updated += 1,
                Err(error) => result.failed.push(format!(
                    "{} item {} model {}: {error}",
                    group.sku, occurrence.item_id, occurrence.model_id
                )),
            }
        }
    }
    Ok(result)
}

async fn get_all_item_ids(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
) -> Result<Vec<i64>, ShopeeError> {
    let path = "/api/v2/product/get_item_list";
    let mut offset = String::from("0");
    let mut item_ids = Vec::new();
    loop {
        let url = signed_shop_url(
            config,
            tokens,
            path,
            &[
                ("offset", offset.as_str()),
                ("page_size", "50"),
                ("item_status", "NORMAL"),
            ],
        );
        let value = get_json(url).await?;
        ensure_no_api_error(&value)?;
        let response = value.get("response").unwrap_or(&Value::Null);
        if let Some(items) = response.get("item").and_then(Value::as_array) {
            item_ids.extend(
                items
                    .iter()
                    .filter_map(|item| item.get("item_id").and_then(Value::as_i64)),
            );
        }
        if !response
            .get("has_next_page")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            break;
        }
        offset = response
            .get("next_offset")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| response.get("next_offset").map(Value::to_string))
            .unwrap_or_else(|| item_ids.len().to_string());
    }
    Ok(item_ids)
}

async fn get_item_base_infos(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    item_ids: &[i64],
) -> Result<Vec<Value>, ShopeeError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let path = "/api/v2/product/get_item_base_info";
    let ids = item_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let url = signed_shop_url(
        config,
        tokens,
        path,
        &[
            ("item_id_list", ids.as_str()),
            ("need_tax_info", "true"),
            ("need_complaint_policy", "true"),
        ],
    );
    let value = get_json(url).await?;
    ensure_no_api_error(&value)?;
    Ok(value
        .pointer("/response/item_list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

async fn get_model_list(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    item_id: i64,
) -> Result<Value, ShopeeError> {
    let path = "/api/v2/product/get_model_list";
    let item_id = item_id.to_string();
    let url = signed_shop_url(config, tokens, path, &[("item_id", item_id.as_str())]);
    let value = get_json(url).await?;
    ensure_no_api_error(&value)?;
    Ok(value.get("response").cloned().unwrap_or(Value::Null))
}

async fn update_stock(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    occurrence: &ShopeeStockOccurrence,
    stock: i64,
) -> Result<(), ShopeeError> {
    let path = "/api/v2/product/update_stock";
    let url = signed_shop_url(config, tokens, path, &[]);
    let seller_stock = match &occurrence.location_id {
        Some(location_id) => json!([{"location_id": location_id, "stock": stock}]),
        None => json!([{"stock": stock}]),
    };
    let body = json!({
        "item_id": occurrence.item_id,
        "stock_list": [{
            "model_id": occurrence.model_id,
            "seller_stock": seller_stock
        }]
    });
    let value = post_json(url, &body).await?;
    ensure_no_api_error(&value)
}

async fn get_json(url: String) -> Result<Value, ShopeeError> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .get(url)
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;
    parse_json_response(response).await
}

async fn post_json(url: String, body: &Value) -> Result<Value, ShopeeError> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;
    parse_json_response(response).await
}

async fn parse_json_response(response: reqwest::Response) -> Result<Value, ShopeeError> {
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
    Ok(value)
}

fn item_occurrence(item: &Value) -> Option<ShopeeStockOccurrence> {
    let sku = normalize_sku(item.get("item_sku").and_then(Value::as_str)?)?;
    let item_id = item.get("item_id").and_then(Value::as_i64)?;
    let name = item
        .get("item_name")
        .and_then(Value::as_str)
        .unwrap_or("sem nome")
        .to_string();
    let stock = item_stock(item);
    Some(ShopeeStockOccurrence {
        sku,
        item_id,
        model_id: 0,
        name,
        seller_stock: stock.0,
        available_stock: stock.1,
        reserved_stock: stock.2,
        location_id: stock.3,
        multi_location: stock.4,
    })
}

fn model_occurrences(
    item_id: i64,
    item_name: &str,
    response: &Value,
) -> Vec<ShopeeStockOccurrence> {
    response
        .get("model")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let sku = normalize_sku(model.get("model_sku").and_then(Value::as_str)?)?;
            let model_id = model.get("model_id").and_then(Value::as_i64)?;
            let stock = model_stock(model);
            Some(ShopeeStockOccurrence {
                sku,
                item_id,
                model_id,
                name: item_name.to_string(),
                seller_stock: stock.0,
                available_stock: stock.1,
                reserved_stock: stock.2,
                location_id: stock.3,
                multi_location: stock.4,
            })
        })
        .collect()
}

fn item_stock(item: &Value) -> (i64, i64, i64, Option<String>, bool) {
    if let Some(stock_info) = item.get("stock_info_v2") {
        stock_from_stock_info(stock_info)
    } else {
        (0, 0, 0, None, false)
    }
}

fn model_stock(model: &Value) -> (i64, i64, i64, Option<String>, bool) {
    model
        .get("stock_info_v2")
        .map(stock_from_stock_info)
        .unwrap_or((0, 0, 0, None, false))
}

fn stock_from_stock_info(stock_info: &Value) -> (i64, i64, i64, Option<String>, bool) {
    let seller_stocks = stock_info
        .get("seller_stock")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let seller_stock = seller_stocks
        .iter()
        .filter_map(|stock| stock.get("stock").and_then(Value::as_i64))
        .sum::<i64>();
    let location_ids = seller_stocks
        .iter()
        .filter_map(|stock| stock.get("location_id").and_then(Value::as_str))
        .filter(|location_id| !location_id.trim().is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let available = stock_info
        .pointer("/summary_info/total_available_stock")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let reserved = stock_info
        .pointer("/summary_info/total_reserved_stock")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    (
        seller_stock,
        available,
        reserved,
        (location_ids.len() == 1).then(|| location_ids[0].clone()),
        location_ids.len() > 1,
    )
}

fn normalize_sku(value: &str) -> Option<String> {
    let sku = value.trim().to_ascii_uppercase();
    (!sku.is_empty()).then_some(sku)
}

pub fn group_stock_occurrences(occurrences: Vec<ShopeeStockOccurrence>) -> Vec<ShopeeStockGroup> {
    let mut map = HashMap::<String, Vec<ShopeeStockOccurrence>>::new();
    for occurrence in occurrences {
        map.entry(occurrence.sku.clone())
            .or_default()
            .push(occurrence);
    }
    let mut groups = map
        .into_iter()
        .map(|(sku, occurrences)| {
            let total_current_stock = occurrences
                .iter()
                .map(|occurrence| occurrence.seller_stock)
                .sum::<i64>();
            let warning = occurrences
                .iter()
                .any(|occurrence| occurrence.multi_location)
                .then(|| String::from("multi-location bloqueado para sync automatico"));
            ShopeeStockGroup {
                sku,
                occurrences,
                total_current_stock,
                target_stock: 0,
                warning,
            }
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| left.sku.cmp(&right.sku));
    groups
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
    if let Some(error) = token_response
        .error
        .as_deref()
        .filter(|error| !error.is_empty())
    {
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

fn signed_public_url(config: &ShopeeConfig, path: &str, timestamp: i64, sign: &str) -> String {
    format!(
        "{}{}?partner_id={}&timestamp={}&sign={}",
        config.api_host, path, config.partner_id, timestamp, sign
    )
}

fn signed_shop_url(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    path: &str,
    extra_params: &[(&str, &str)],
) -> String {
    let timestamp = now_timestamp();
    let sign = shop_sign(
        config.partner_id,
        path,
        timestamp,
        &tokens.access_token,
        config.shop_id,
        &config.partner_key,
    );
    let mut url = format!(
        "{}{}?partner_id={}&timestamp={}&access_token={}&shop_id={}&sign={}",
        config.api_host,
        path,
        config.partner_id,
        timestamp,
        percent_encode(&tokens.access_token),
        config.shop_id,
        sign
    );
    for (key, value) in extra_params {
        url.push('&');
        url.push_str(key);
        url.push('=');
        url.push_str(&percent_encode(value));
    }
    url
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

fn authorization_entry_url(config: &ShopeeConfig) -> String {
    config
        .redirect_url
        .as_deref()
        .and_then(|redirect_url| {
            redirect_url
                .strip_suffix("/shopee/callback")
                .map(|base_url| format!("{base_url}/shopee/auth"))
        })
        .unwrap_or_else(|| authorization_url(config))
}

enum CallbackResult {
    TokenSaved,
    WebhookAccepted,
    Redirect(String),
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
    if is_push_request(&request) {
        return CallbackResult::WebhookAccepted;
    }
    if is_auth_request(&request) {
        return match ShopeeConfig::from_env() {
            Ok(config) => CallbackResult::Redirect(authorization_url(&config)),
            Err(error) => CallbackResult::Error(error),
        };
    }
    let Some(code) = extract_code_from_http_request(&request) else {
        return CallbackResult::Error(ShopeeError::Api(String::from(
            "callback OAuth recebido sem code; abra /shopee/auth para iniciar a autorizacao",
        )));
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

fn is_push_request(request: &str) -> bool {
    request_path(request)
        .map(|path| path.trim_start_matches('/').starts_with("shopee/push"))
        .unwrap_or(false)
}

fn is_auth_request(request: &str) -> bool {
    request_path(request)
        .map(|path| path.trim_start_matches('/').starts_with("shopee/auth"))
        .unwrap_or(false)
}

fn write_http_response(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())
}

fn write_http_redirect(stream: &mut TcpStream, location: &str) -> std::io::Result<()> {
    let body = "Redirecionando para autorizacao Shopee.";
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())
}

fn extract_code_from_http_request(request: &str) -> Option<String> {
    let path = request_path(request)?;
    let query = path.split_once('?')?.1;
    for part in query.split('&') {
        if let Some(code) = part.strip_prefix("code=") {
            return non_empty_owned(&percent_decode(code));
        }
    }
    None
}

fn request_path(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().nth(1)
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
        output.push(if bytes[index] == b'+' {
            b' '
        } else {
            bytes[index]
        });
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
                "abra {} para autorizar a loja. Redirect cadastrado na Shopee: {redirect_url}",
                authorization_entry_url(self)
            ),
            None => String::from(
                "configure SHOPEE_REDIRECT_URL; depois abra /shopee/auth para autorizar a loja",
            ),
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
            ShopeeError::RefreshExpired => {
                write!(formatter, "refresh_token expirado; reautorize a loja")
            }
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
        let sign = public_sign(
            100,
            "/api/v2/auth/access_token/get",
            1_700_000_000,
            "secret",
        );
        let expected = super::sign("100/api/v2/auth/access_token/get1700000000", b"secret");
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
        let request =
            "GET /shopee/callback?code=abc%20123&shop_id=1 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(
            extract_code_from_http_request(request),
            Some(String::from("abc 123"))
        );
    }

    #[test]
    fn push_request_is_not_treated_as_oauth_callback() {
        let request = "POST /shopee/push HTTP/1.1\r\nHost: localhost\r\n\r\n{}";
        assert!(is_push_request(request));
        assert_eq!(extract_code_from_http_request(request), None);
    }

    #[test]
    fn auth_request_is_detected_as_authorization_entrypoint() {
        let request = "GET /shopee/auth HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert!(is_auth_request(request));
        assert!(!is_push_request(request));
    }

    #[test]
    fn authorization_entry_uses_short_public_callback_route() {
        let config = ShopeeConfig {
            partner_id: 100,
            partner_key: String::from("secret"),
            shop_id: 200,
            api_host: String::from("https://partner.shopeemobile.com"),
            redirect_url: Some(String::from("https://example.ngrok.app/shopee/callback")),
        };

        assert_eq!(
            authorization_entry_url(&config),
            "https://example.ngrok.app/shopee/auth"
        );
    }

    #[test]
    fn authorization_hint_points_to_auth_entrypoint_not_callback() {
        let config = ShopeeConfig {
            partner_id: 100,
            partner_key: String::from("secret"),
            shop_id: 200,
            api_host: String::from("https://partner.shopeemobile.com"),
            redirect_url: Some(String::from("https://example.ngrok.app/shopee/callback")),
        };
        let hint = config.authorization_hint();

        assert!(hint.contains("https://example.ngrok.app/shopee/auth"));
        assert!(hint.contains("Redirect cadastrado"));
    }

    #[test]
    fn groups_stock_occurrences_by_normalized_sku() {
        let groups = group_stock_occurrences(
            vec![
                ShopeeStockOccurrence {
                    sku: String::from("abc"),
                    item_id: 1,
                    model_id: 0,
                    name: String::from("A"),
                    seller_stock: 2,
                    available_stock: 2,
                    reserved_stock: 0,
                    location_id: None,
                    multi_location: false,
                },
                ShopeeStockOccurrence {
                    sku: String::from("ABC"),
                    item_id: 2,
                    model_id: 10,
                    name: String::from("B"),
                    seller_stock: 3,
                    available_stock: 3,
                    reserved_stock: 0,
                    location_id: None,
                    multi_location: false,
                },
            ]
            .into_iter()
            .map(|mut occurrence| {
                occurrence.sku = normalize_sku(&occurrence.sku).unwrap();
                occurrence
            })
            .collect(),
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].sku, "ABC");
        assert_eq!(groups[0].total_current_stock, 5);
        assert_eq!(groups[0].target_stock, 0);
    }

    #[test]
    fn stock_group_toggle_switches_between_zero_and_one_hundred() {
        let mut group = ShopeeStockGroup {
            sku: String::from("ABC"),
            occurrences: Vec::new(),
            total_current_stock: 5,
            target_stock: 0,
            warning: None,
        };

        group.toggle_target();
        assert_eq!(group.target_stock, 100);
        assert_eq!(group.target_label(), "Ativar 100");
        group.toggle_target();
        assert_eq!(group.target_stock, 0);
        assert_eq!(group.target_label(), "Zerar 0");
    }

    #[test]
    fn multi_location_group_is_not_syncable() {
        let groups = group_stock_occurrences(vec![ShopeeStockOccurrence {
            sku: String::from("ABC"),
            item_id: 1,
            model_id: 0,
            name: String::from("A"),
            seller_stock: 2,
            available_stock: 2,
            reserved_stock: 0,
            location_id: None,
            multi_location: true,
        }]);

        assert!(!groups[0].can_sync());
        assert!(groups[0].warning.is_some());
    }

    #[test]
    fn redirect_url_is_percent_encoded() {
        assert_eq!(
            percent_encode("https://example.ngrok.app/shopee/callback"),
            "https%3A%2F%2Fexample.ngrok.app%2Fshopee%2Fcallback"
        );
    }
}

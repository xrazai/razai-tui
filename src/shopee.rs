use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
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
use crate::db::{TecidoRecord, VinculoRecord};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

type HmacSha256 = Hmac<Sha256>;

const ACCESS_TOKEN_KEY: &str = "shopee_access_token";
const REFRESH_TOKEN_KEY: &str = "shopee_refresh_token";
const ACCESS_TOKEN_EXPIRES_AT_KEY: &str = "shopee_access_token_expires_at";
const REFRESH_TOKEN_EXPIRES_AT_KEY: &str = "shopee_refresh_token_expires_at";
const REFRESH_WINDOW_SECONDS: i64 = 10 * 60;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;
const SHOPEE_FABRIC_CATEGORY_ID: i64 = 100416;
const SHOPEE_FABRIC_CATEGORY_LABEL: &str = "Roupas Femininas > Tecidos > Outros";
const SHOPEE_FABRIC_NCM: &str = "55161300";
const SHOPEE_MAX_MODEL_PRICE_RATIO: f64 = 5.0;
const SHOPEE_STOCK_FETCH_CONCURRENCY: usize = 8;
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
    pub parent_sku: String,
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
pub struct ShopeeStockParentGroup {
    pub sku: String,
    pub name: String,
    pub groups: Vec<ShopeeStockGroup>,
    pub total_current_stock: i64,
    pub expanded: bool,
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

#[derive(Clone, Debug)]
pub struct ShopeeListingResult {
    pub item_id: i64,
    pub sku: String,
    pub color_count: usize,
    pub size_count: usize,
    pub model_count: usize,
    pub image_id: String,
}

#[derive(Clone, Debug)]
pub struct ShopeeListingModel {
    pub tier_index: [usize; 2],
    pub model_sku: String,
    pub weight_kg: f64,
    pub price: f64,
}

#[derive(Clone, Debug)]
pub struct ShopeeListingUpdatePlan {
    pub item_id: i64,
    pub item_name: String,
    pub parent_sku: String,
    pub existing_color_count: usize,
    pub size_count: usize,
    pub missing_colors: Vec<String>,
    pub model_count: usize,
    pub blocked_reason: Option<String>,
    needs_tier_update: bool,
    tier_variation: Vec<Value>,
    models_to_add: Vec<Value>,
}

#[derive(Clone, Debug, Default)]
pub struct ShopeeListingUpdateResult {
    pub updated_items: usize,
    pub added_models: usize,
    pub skipped_items: usize,
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
    format!(
        "Guia Shopee BR: Criar anuncio usa categoria {SHOPEE_FABRIC_CATEGORY_LABEL}, produto local, marca, logistica, preco, peso, dimensoes, estoque, GTIN, imagens e tax_info BR. Consulte docs/ShopeeDocs/SHOPEE_CRIAR_ANUNCIO_BR.md e SHOPEE_ESTOQUE_SKU.md.",
    )
}

pub async fn create_fabric_listing(
    pool: Option<&PgPool>,
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    meter_price: f64,
) -> Result<ShopeeListingResult, ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    let tokens = ensure_connected_with_config(pool, &config).await?;
    match create_fabric_listing_with_tokens(&config, &tokens, tecido, vinculos, meter_price).await {
        Ok(result) => Ok(result),
        Err(error) if error.is_token_error() => {
            let tokens = refresh_tokens(pool, &config, &tokens).await?;
            create_fabric_listing_with_tokens(&config, &tokens, tecido, vinculos, meter_price).await
        }
        Err(error) => Err(error),
    }
}

pub async fn preview_listing_updates(
    pool: Option<&PgPool>,
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
) -> Result<Vec<ShopeeListingUpdatePlan>, ShopeeError> {
    validate_listing_inputs(tecido, vinculos, 1.0)?;
    let config = ShopeeConfig::from_env()?;
    let tokens = ensure_connected(pool).await?;
    let colors = listing_colors(vinculos)?;
    preview_listing_updates_with_tokens(&config, &tokens, tecido, vinculos, &colors).await
}

pub async fn apply_listing_update_plans(
    pool: Option<&PgPool>,
    plans: &[ShopeeListingUpdatePlan],
) -> Result<ShopeeListingUpdateResult, ShopeeError> {
    let config = ShopeeConfig::from_env()?;
    let mut tokens = ensure_connected(pool).await?;
    let mut result = ShopeeListingUpdateResult::default();

    for plan in plans {
        if let Some(reason) = &plan.blocked_reason {
            result.skipped_items += 1;
            result
                .failed
                .push(format!("item {} bloqueado: {reason}", plan.item_id));
            continue;
        }
        if plan.models_to_add.is_empty() {
            result.skipped_items += 1;
            continue;
        }

        match apply_single_listing_update(&config, &tokens, plan).await {
            Ok(added) => {
                result.updated_items += 1;
                result.added_models += added;
            }
            Err(error) if error.is_token_error() => {
                tokens = refresh_tokens(pool, &config, &tokens).await?;
                match apply_single_listing_update(&config, &tokens, plan).await {
                    Ok(added) => {
                        result.updated_items += 1;
                        result.added_models += added;
                    }
                    Err(error) => result
                        .failed
                        .push(format!("item {}: {error}", plan.item_id)),
                }
            }
            Err(error) => result
                .failed
                .push(format!("item {}: {error}", plan.item_id)),
        }
    }

    Ok(result)
}

pub async fn fetch_online_stock_groups(
    pool: Option<&PgPool>,
) -> Result<Vec<ShopeeStockParentGroup>, ShopeeError> {
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
) -> Result<Vec<ShopeeStockParentGroup>, ShopeeError> {
    let item_ids = get_all_item_ids(config, tokens).await?;
    let item_infos = get_all_item_base_infos(config, tokens, &item_ids).await?;

    let mut occurrences = Vec::new();
    let mut model_tasks = tokio::task::JoinSet::new();
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
            if model_tasks.len() >= SHOPEE_STOCK_FETCH_CONCURRENCY {
                let models = join_shopee_task(model_tasks.join_next().await)?;
                occurrences.extend(models);
            }
            let task_config = config.clone();
            let task_tokens = tokens.clone();
            model_tasks.spawn(async move {
                let models = get_model_list(&task_config, &task_tokens, item_id).await?;
                Ok::<_, ShopeeError>(model_occurrences(&item, item_id, &item_name, &models))
            });
        } else if let Some(occurrence) = item_occurrence(&item) {
            occurrences.push(occurrence);
        }
    }
    while let Some(result) = model_tasks.join_next().await {
        occurrences.extend(join_shopee_task(Some(result))?);
    }

    Ok(group_stock_occurrences_by_parent(occurrences))
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

async fn create_fabric_listing_with_tokens(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    meter_price: f64,
) -> Result<ShopeeListingResult, ShopeeError> {
    validate_listing_inputs(tecido, vinculos, meter_price)?;
    let image_path = default_listing_image_path()?;
    let image_id = upload_image(config, &image_path).await?;
    let logistics = listing_logistics(config, tokens).await?;
    let colors = listing_colors(vinculos)?;
    let sizes = listing_sizes(colors.len());
    let models = listing_models(tecido, vinculos, &sizes, meter_price)?;
    let description = fabric_description(tecido);

    let item_body = json!({
        "item_name": listing_item_name(tecido),
        "description": description,
        "category_id": SHOPEE_FABRIC_CATEGORY_ID,
        "brand": {"brand_id": 0, "original_brand_name": "Razai Tecidos"},
        "condition": "NEW",
        "item_status": "NORMAL",
        "item_sku": tecido.sku,
        "original_price": models
            .iter()
            .map(|model| model.price)
            .fold(meter_price, f64::min),
        "weight": base_weight_kg(tecido)?,
        "dimension": {"package_height": 5, "package_width": 20, "package_length": 20},
        "image": {"image_id_list": [image_id]},
        "logistic_info": logistics,
        "seller_stock": [{"stock": 1}],
        "normal_stock": 1,
        "attribute_list": [],
        "item_dangerous": 0,
        "tax_info": {
            "ncm": SHOPEE_FABRIC_NCM,
            "cest": "00",
            "same_state_cfop": "5102",
            "diff_state_cfop": "6102",
            "csosn": "102",
            "origin": "0",
            "measure_unit": "M"
        }
    });
    let add_value = post_json(
        signed_shop_url(config, tokens, "/api/v2/product/add_item", &[]),
        &item_body,
    )
    .await?;
    ensure_no_api_error(&add_value)?;
    let item_id = add_value
        .pointer("/response/item_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ShopeeError::Api(String::from("item_id ausente ao criar anuncio")))?;

    thread::sleep(Duration::from_secs(6));

    let variation_body = json!({
        "item_id": item_id,
        "tier_variation": [
            {
                "name": "Cor",
                "option_list": colors
                    .iter()
                    .map(|color| json!({"option": color, "image": {"image_id": image_id}}))
                    .collect::<Vec<_>>()
            },
            {
                "name": "Tamanho",
                "option_list": sizes
                    .iter()
                    .map(|size| json!({"option": size.label}))
                    .collect::<Vec<_>>()
            }
        ],
        "model": models
            .iter()
            .map(|model| json!({
                "tier_index": model.tier_index,
                "original_price": model.price,
                "model_sku": model.model_sku,
                "seller_stock": [{"stock": 1}],
                "gtin_code": "00",
                "weight": model.weight_kg,
                "dimension": {"package_height": 10, "package_width": 30, "package_length": 30}
            }))
            .collect::<Vec<_>>()
    });
    let tier_value = post_json(
        signed_shop_url(config, tokens, "/api/v2/product/init_tier_variation", &[]),
        &variation_body,
    )
    .await?;
    ensure_no_api_error(&tier_value)?;

    Ok(ShopeeListingResult {
        item_id,
        sku: tecido.sku.clone(),
        color_count: colors.len(),
        size_count: sizes.len(),
        model_count: models.len(),
        image_id,
    })
}

async fn preview_listing_updates_with_tokens(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    local_colors: &[String],
) -> Result<Vec<ShopeeListingUpdatePlan>, ShopeeError> {
    let wanted_sku = normalize_sku(&tecido.sku).ok_or_else(|| {
        ShopeeError::Api(String::from("SKU do tecido invalido para buscar anuncios"))
    })?;
    let item_ids = get_all_item_ids(config, tokens).await?;
    let item_infos = get_all_item_base_infos(config, tokens, &item_ids).await?;
    let mut plans = Vec::new();

    for item in item_infos {
        let Some(remote_sku) = item
            .get("item_sku")
            .and_then(Value::as_str)
            .and_then(normalize_sku)
        else {
            continue;
        };
        if remote_sku != wanted_sku {
            continue;
        }

        let item_id = item
            .get("item_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| ShopeeError::Api(String::from("item_id ausente em anuncio Shopee")))?;
        let item_name = item
            .get("item_name")
            .and_then(Value::as_str)
            .unwrap_or("Anuncio sem nome")
            .to_string();
        let models = get_model_list(config, tokens, item_id).await?;
        plans.push(build_listing_update_plan(
            tecido,
            vinculos,
            local_colors,
            item_id,
            &item_name,
            &remote_sku,
            &models,
        )?);
    }

    Ok(plans)
}

async fn apply_single_listing_update(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    plan: &ShopeeListingUpdatePlan,
) -> Result<usize, ShopeeError> {
    if plan.needs_tier_update {
        let update_body = json!({
            "item_id": plan.item_id,
            "tier_variation": plan.tier_variation
        });
        let update_value = post_json(
            signed_shop_url(config, tokens, "/api/v2/product/update_tier_variation", &[]),
            &update_body,
        )
        .await?;
        ensure_no_api_error(&update_value)?;
    }

    let mut added = 0usize;
    for chunk in plan.models_to_add.chunks(50) {
        let body = json!({
            "item_id": plan.item_id,
            "model_list": chunk
        });
        let value = post_json(
            signed_shop_url(config, tokens, "/api/v2/product/add_model", &[]),
            &body,
        )
        .await?;
        ensure_no_api_error(&value)?;
        added += chunk.len();
    }
    Ok(added)
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

async fn get_all_item_base_infos(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
    item_ids: &[i64],
) -> Result<Vec<Value>, ShopeeError> {
    let mut item_infos = Vec::new();
    let mut tasks = tokio::task::JoinSet::new();
    for chunk in item_ids.chunks(50) {
        if tasks.len() >= SHOPEE_STOCK_FETCH_CONCURRENCY {
            item_infos.extend(join_shopee_task(tasks.join_next().await)?);
        }
        let task_config = config.clone();
        let task_tokens = tokens.clone();
        let task_item_ids = chunk.to_vec();
        tasks.spawn(async move {
            get_item_base_infos(&task_config, &task_tokens, &task_item_ids).await
        });
    }
    while let Some(result) = tasks.join_next().await {
        item_infos.extend(join_shopee_task(Some(result))?);
    }
    Ok(item_infos)
}

fn join_shopee_task<T>(
    result: Option<Result<Result<T, ShopeeError>, tokio::task::JoinError>>,
) -> Result<T, ShopeeError> {
    result
        .ok_or_else(|| ShopeeError::Http("tarefa Shopee finalizada sem resultado".to_string()))?
        .map_err(|error| ShopeeError::Http(error.to_string()))?
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

async fn listing_logistics(
    config: &ShopeeConfig,
    tokens: &ShopeeTokens,
) -> Result<Vec<Value>, ShopeeError> {
    let value = get_json(signed_shop_url(
        config,
        tokens,
        "/api/v2/logistics/get_channel_list",
        &[],
    ))
    .await?;
    ensure_no_api_error(&value)?;
    let wanted = [
        "Retirada",
        "Shopee Xpress",
        "Entrega Rápida",
        "Entrega Rapida",
    ];
    let channels = value
        .pointer("/response/logistics_channel_list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut logistics = Vec::new();
    for channel in channels {
        let name = channel
            .get("logistics_channel_name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !wanted.iter().any(|wanted| name.contains(wanted)) {
            continue;
        }
        if let Some(id) = channel.get("logistics_channel_id").and_then(Value::as_i64) {
            logistics.push(json!({
                "logistic_id": id,
                "enabled": true,
                "is_free": false
            }));
        }
    }
    if logistics.is_empty() {
        return Err(ShopeeError::Api(String::from(
            "nenhum canal de envio solicitado esta disponivel na loja",
        )));
    }
    Ok(logistics)
}

async fn upload_image(config: &ShopeeConfig, path: &Path) -> Result<String, ShopeeError> {
    let bytes = fs::read(path).map_err(|error| {
        ShopeeError::Env(format!(
            "falha ao ler SHOPEE_DEFAULT_IMAGE_PATH {}: {error}",
            path.display()
        ))
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.jpg")
        .to_string();
    let part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
    let form = reqwest::multipart::Form::new().part("image", part);
    let path = "/api/v2/media_space/upload_image";
    let timestamp = now_timestamp();
    let sign = public_sign(config.partner_id, path, timestamp, &config.partner_key);
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|error| ShopeeError::Http(error.to_string()))?
        .post(signed_public_url(config, path, timestamp, &sign))
        .multipart(form)
        .send()
        .await
        .map_err(|error| ShopeeError::Http(error.to_string()))?;
    let value = parse_json_response(response).await?;
    ensure_no_api_error(&value)?;
    value
        .pointer("/response/image_info/image_id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| ShopeeError::Api(String::from("image_id ausente no upload Shopee")))
}

fn default_listing_image_path() -> Result<PathBuf, ShopeeError> {
    if let Ok(path) = std::env::var("SHOPEE_DEFAULT_IMAGE_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(ShopeeError::Env(format!(
            "SHOPEE_DEFAULT_IMAGE_PATH nao aponta para arquivo valido: {}",
            path.display()
        )));
    }
    find_first_picture().ok_or_else(|| {
        ShopeeError::Env(String::from(
            "configure SHOPEE_DEFAULT_IMAGE_PATH com uma imagem JPG/PNG para o anuncio",
        ))
    })
}

fn find_first_picture() -> Option<PathBuf> {
    let pictures = std::env::var("USERPROFILE")
        .ok()
        .map(|profile| PathBuf::from(profile).join("Pictures"))?;
    find_first_picture_in(&pictures, 0)
}

fn find_first_picture_in(path: &Path, depth: usize) -> Option<PathBuf> {
    if depth > 3 {
        return None;
    }
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_supported_image(&path) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_first_picture_in(&path, depth + 1)
        {
            return Some(found);
        }
    }
    None
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png"
            )
        })
        .unwrap_or(false)
}

#[derive(Clone, Debug)]
struct ListingSize {
    label: String,
    meters: f64,
}

fn validate_listing_inputs(
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    meter_price: f64,
) -> Result<(), ShopeeError> {
    if tecido.gramatura_linear_g_m.unwrap_or_default() <= 0 {
        return Err(ShopeeError::Api(String::from(
            "tecido sem gramatura linear; cadastre a gramatura antes de criar anuncio",
        )));
    }
    if vinculos.is_empty() {
        return Err(ShopeeError::Api(String::from(
            "tecido sem vinculos de cor/estampa; crie os vinculos antes de anunciar",
        )));
    }
    if vinculos.len() > 100 {
        return Err(ShopeeError::Api(String::from(
            "mais de 100 cores/vinculos; a Shopee permite no maximo 100 combinacoes",
        )));
    }
    if meter_price < 1.0 {
        return Err(ShopeeError::Api(String::from(
            "preco por metro Shopee deve ser maior ou igual a 1,00",
        )));
    }
    Ok(())
}

fn listing_item_name(tecido: &TecidoRecord) -> String {
    let name = format!("{} Razai Tecidos", tecido.nome.trim());
    truncate_chars(&name, 120)
}

fn fabric_description(tecido: &TecidoRecord) -> String {
    let gramatura = tecido
        .gramatura_linear_g_m
        .map(|value| format!("{value} g/m linear"))
        .unwrap_or_else(|| String::from("gramatura sob consulta"));
    let gramatura_m2 = tecido
        .gramatura_g_m2
        .map(|value| format!("{value} g/m2"))
        .unwrap_or_else(|| String::from("g/m2 sob consulta"));
    format!(
        "{nome} da Razai Tecidos.\n\nEspecificacoes tecnicas:\n- Composicao: {composicao}\n- Largura: {largura:.2} m\n- Gramatura: {gramatura}\n- Gramatura por area: {gramatura_m2}\n- Tipo: {tipo}\n- Acabamento: {acabamento}\n- Transparencia: {transparencia}\n- Elasticidade: {elasticidade}\n\nIndicado para confeccao, artesanato, decoracao, moda, patchwork e projetos criativos conforme a estrutura do tecido. Escolha a cor e o tamanho desejado nas variacoes do anuncio.\n\nAs cores podem variar levemente conforme tela, iluminacao e lote. Em compras de maior metragem, recomendamos adquirir a quantidade necessaria no mesmo pedido para melhor continuidade do lote.",
        nome = tecido.nome.trim(),
        composicao = tecido.composicao.trim(),
        largura = tecido.largura_m,
        tipo = tecido.tipo,
        acabamento = tecido.acabamento,
        transparencia = tecido.transparencia,
        elasticidade = tecido.elasticidade,
    )
}

fn listing_colors(vinculos: &[VinculoRecord]) -> Result<Vec<String>, ShopeeError> {
    let mut colors = Vec::new();
    for vinculo in vinculos {
        let base = truncate_chars(vinculo.cor_nome.trim(), 30);
        if !base.is_empty() {
            let mut color = base.clone();
            let mut suffix = 2;
            while colors.iter().any(|existing| existing == &color) {
                let marker = format!(" {suffix}");
                color = format!(
                    "{}{}",
                    truncate_chars(&base, 30usize.saturating_sub(marker.chars().count())),
                    marker
                );
                suffix += 1;
            }
            colors.push(color);
        }
    }
    if colors.is_empty() {
        Err(ShopeeError::Api(String::from(
            "nenhum nome de cor/variacao valido encontrado",
        )))
    } else {
        Ok(colors)
    }
}

fn listing_sizes(color_count: usize) -> Vec<ListingSize> {
    let max_size_count = (100 / color_count.max(1)).clamp(1, 20);
    let mut sizes = vec![ListingSize {
        label: String::from("0,5m"),
        meters: 0.5,
    }];
    let mut meter = 1.0;
    while sizes.len() < max_size_count {
        if meter / 0.5 > SHOPEE_MAX_MODEL_PRICE_RATIO {
            break;
        }
        sizes.push(ListingSize {
            label: format!("{}m", meter as i64),
            meters: meter,
        });
        meter += 1.0;
    }
    sizes
}

fn listing_models(
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    sizes: &[ListingSize],
    meter_price: f64,
) -> Result<Vec<ShopeeListingModel>, ShopeeError> {
    let mut models = Vec::new();
    for (color_index, vinculo) in vinculos.iter().enumerate() {
        for (size_index, size) in sizes.iter().enumerate() {
            models.push(ShopeeListingModel {
                tier_index: [color_index, size_index],
                model_sku: vinculo
                    .sku
                    .as_deref()
                    .filter(|sku| !sku.trim().is_empty())
                    .unwrap_or(&tecido.sku)
                    .to_string(),
                weight_kg: model_weight_kg(tecido, size.meters)?,
                price: model_price(meter_price, size.meters),
            });
        }
    }
    Ok(models)
}

fn build_listing_update_plan(
    tecido: &TecidoRecord,
    vinculos: &[VinculoRecord],
    local_colors: &[String],
    item_id: i64,
    item_name: &str,
    parent_sku: &str,
    model_response: &Value,
) -> Result<ShopeeListingUpdatePlan, ShopeeError> {
    let Some(tiers) = model_response
        .get("tier_variation")
        .and_then(Value::as_array)
    else {
        return Ok(blocked_listing_update_plan(
            item_id,
            item_name,
            parent_sku,
            "anuncio sem variacoes",
        ));
    };
    if tiers.len() != 2 {
        return Ok(blocked_listing_update_plan(
            item_id,
            item_name,
            parent_sku,
            "estrutura diferente de Cor x Tamanho",
        ));
    }

    let color_tier_name = tiers[0]
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let size_tier_name = tiers[1]
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !tier_name_matches(color_tier_name, "cor") || !tier_name_matches(size_tier_name, "tamanho") {
        return Ok(blocked_listing_update_plan(
            item_id,
            item_name,
            parent_sku,
            "variacoes nao sao Cor x Tamanho",
        ));
    }

    let existing_colors = tier_options(&tiers[0]);
    let sizes = tier_options(&tiers[1]);
    if existing_colors.is_empty() || sizes.is_empty() {
        return Ok(blocked_listing_update_plan(
            item_id,
            item_name,
            parent_sku,
            "tier de cor ou tamanho vazio",
        ));
    }

    let remote_models = remote_listing_models(model_response);
    let color_matches =
        listing_color_matches(vinculos, local_colors, &existing_colors, &remote_models);
    let missing_colors = color_matches
        .iter()
        .filter(|match_info| match_info.is_new_color)
        .map(|match_info| match_info.color.clone())
        .collect::<Vec<_>>();
    let final_color_count = existing_colors.len() + missing_colors.len();
    let final_model_count = final_color_count * sizes.len();
    if final_model_count > 100 {
        return Ok(blocked_listing_update_plan(
            item_id,
            item_name,
            parent_sku,
            "total de combinacoes passaria de 100",
        ));
    }

    let size_prices = match remote_size_prices(model_response, sizes.len()) {
        Ok(prices) => prices,
        Err(_) => {
            return Ok(blocked_listing_update_plan(
                item_id,
                item_name,
                parent_sku,
                "nao foi possivel mapear preco por tamanho",
            ));
        }
    };

    let color_image_id = first_tier_image_id(&tiers[0]);
    let mut tier_variation = tiers.clone();
    let color_options = tier_variation[0]
        .get_mut("option_list")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| ShopeeError::Api(String::from("option_list de cor invalido")))?;
    for color in &missing_colors {
        let mut option = json!({"option": color});
        if let Some(image_id) = &color_image_id {
            option["image"] = json!({"image_id": image_id});
        }
        color_options.push(option);
    }

    let mut models_to_add = Vec::new();
    for match_info in &color_matches {
        let vinculo = &vinculos[match_info.vinculo_index];
        let model_sku = vinculo
            .sku
            .as_deref()
            .filter(|sku| !sku.trim().is_empty())
            .unwrap_or(&tecido.sku)
            .to_string();
        for (size_index, size) in sizes.iter().enumerate() {
            if remote_models
                .iter()
                .any(|model| model.tier_index == [match_info.color_index, size_index])
            {
                continue;
            }
            let meters = parse_listing_size_meters(size).ok_or_else(|| {
                ShopeeError::Api(format!("tamanho Shopee invalido para peso: {size}"))
            })?;
            models_to_add.push(json!({
                "tier_index": [match_info.color_index, size_index],
                "original_price": size_prices[size_index],
                "model_sku": model_sku,
                "seller_stock": [{"stock": 1}],
                "gtin_code": "00",
                "weight": model_weight_kg(tecido, meters)?,
                "dimension": {"package_height": 10, "package_width": 30, "package_length": 30}
            }));
        }
    }

    if models_to_add.is_empty() {
        return Ok(ShopeeListingUpdatePlan {
            item_id,
            item_name: item_name.to_string(),
            parent_sku: parent_sku.to_string(),
            existing_color_count: existing_colors.len(),
            size_count: sizes.len(),
            missing_colors,
            model_count: 0,
            blocked_reason: None,
            needs_tier_update: false,
            tier_variation: tiers.clone(),
            models_to_add,
        });
    }

    Ok(ShopeeListingUpdatePlan {
        item_id,
        item_name: item_name.to_string(),
        parent_sku: parent_sku.to_string(),
        existing_color_count: existing_colors.len(),
        size_count: sizes.len(),
        missing_colors,
        model_count: models_to_add.len(),
        blocked_reason: None,
        needs_tier_update: color_matches
            .iter()
            .any(|match_info| match_info.is_new_color),
        tier_variation,
        models_to_add,
    })
}

fn blocked_listing_update_plan(
    item_id: i64,
    item_name: &str,
    parent_sku: &str,
    reason: &str,
) -> ShopeeListingUpdatePlan {
    ShopeeListingUpdatePlan {
        item_id,
        item_name: item_name.to_string(),
        parent_sku: parent_sku.to_string(),
        existing_color_count: 0,
        size_count: 0,
        missing_colors: Vec::new(),
        model_count: 0,
        blocked_reason: Some(reason.to_string()),
        needs_tier_update: false,
        tier_variation: Vec::new(),
        models_to_add: Vec::new(),
    }
}

fn tier_name_matches(value: &str, expected: &str) -> bool {
    value.trim().eq_ignore_ascii_case(expected)
}

fn tier_options(tier: &Value) -> Vec<String> {
    tier.get("option_list")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|option| option.get("option").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn first_tier_image_id(tier: &Value) -> Option<String> {
    tier.get("option_list")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|option| option.pointer("/image/image_id").and_then(Value::as_str))
        .find(|image_id| !image_id.trim().is_empty())
        .map(ToString::to_string)
}

fn remote_size_prices(model_response: &Value, size_count: usize) -> Result<Vec<f64>, ShopeeError> {
    let mut prices = vec![None; size_count];
    let models = model_response
        .get("model")
        .and_then(Value::as_array)
        .ok_or_else(|| ShopeeError::Api(String::from("model list ausente")))?;
    for model in models {
        let Some(size_index) = model
            .get("tier_index")
            .and_then(Value::as_array)
            .and_then(|indexes| indexes.get(1))
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
        else {
            continue;
        };
        if size_index >= size_count || prices[size_index].is_some() {
            continue;
        }
        prices[size_index] = remote_model_price(model);
    }

    prices
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ShopeeError::Api(String::from("preco ausente para algum tamanho")))
}

fn remote_model_price(model: &Value) -> Option<f64> {
    model
        .get("price_info")
        .and_then(Value::as_array)
        .and_then(|prices| prices.first())
        .and_then(|price| {
            price
                .get("original_price")
                .or_else(|| price.get("current_price"))
                .and_then(Value::as_f64)
        })
        .or_else(|| model.get("original_price").and_then(Value::as_f64))
}

#[derive(Clone, Debug)]
struct RemoteListingModel {
    tier_index: [usize; 2],
    model_sku: Option<String>,
}

#[derive(Clone, Debug)]
struct ListingColorMatch {
    vinculo_index: usize,
    color: String,
    color_index: usize,
    is_new_color: bool,
}

fn remote_listing_models(model_response: &Value) -> Vec<RemoteListingModel> {
    model_response
        .get("model")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let indexes = model.get("tier_index")?.as_array()?;
            let color_index = indexes
                .first()
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())?;
            let size_index = indexes
                .get(1)
                .and_then(Value::as_u64)
                .and_then(|index| usize::try_from(index).ok())?;
            Some(RemoteListingModel {
                tier_index: [color_index, size_index],
                model_sku: model
                    .get("model_sku")
                    .and_then(Value::as_str)
                    .and_then(normalize_sku),
            })
        })
        .collect()
}

fn listing_color_matches(
    vinculos: &[VinculoRecord],
    local_colors: &[String],
    existing_colors: &[String],
    remote_models: &[RemoteListingModel],
) -> Vec<ListingColorMatch> {
    let mut matches = Vec::new();
    let mut next_new_color_index = existing_colors.len();

    for (vinculo_index, (vinculo, color)) in vinculos.iter().zip(local_colors.iter()).enumerate() {
        if let Some(color_index) = color_index_by_sku(vinculo, remote_models) {
            matches.push(ListingColorMatch {
                vinculo_index,
                color: color.clone(),
                color_index,
                is_new_color: false,
            });
            continue;
        }

        if let Some(color_index) = existing_colors
            .iter()
            .position(|existing| normalized_label(existing) == normalized_label(color))
        {
            matches.push(ListingColorMatch {
                vinculo_index,
                color: color.clone(),
                color_index,
                is_new_color: false,
            });
            continue;
        }

        matches.push(ListingColorMatch {
            vinculo_index,
            color: color.clone(),
            color_index: next_new_color_index,
            is_new_color: true,
        });
        next_new_color_index += 1;
    }

    matches
}

fn color_index_by_sku(
    vinculo: &VinculoRecord,
    remote_models: &[RemoteListingModel],
) -> Option<usize> {
    let local_sku = vinculo.sku.as_deref().and_then(normalize_sku)?;
    remote_models
        .iter()
        .find(|model| {
            model
                .model_sku
                .as_deref()
                .is_some_and(|remote_sku| listing_skus_match(&local_sku, remote_sku))
        })
        .map(|model| model.tier_index[0])
}

fn listing_skus_match(local_sku: &str, remote_sku: &str) -> bool {
    local_sku == remote_sku
        || stock_child_sku_from_model_sku(local_sku).as_deref() == Some(remote_sku)
        || stock_child_sku_from_model_sku(remote_sku).as_deref() == Some(local_sku)
        || stock_child_sku_from_model_sku(local_sku) == stock_child_sku_from_model_sku(remote_sku)
}

fn normalized_label(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn parse_listing_size_meters(value: &str) -> Option<f64> {
    let trimmed = value.trim().trim_end_matches('m').trim_end_matches('M');
    trimmed.replace(',', ".").parse::<f64>().ok()
}

fn model_price(meter_price: f64, meters: f64) -> f64 {
    ((meter_price * meters) * 100.0).round() / 100.0
}

fn base_weight_kg(tecido: &TecidoRecord) -> Result<f64, ShopeeError> {
    model_weight_kg(tecido, 1.0)
}

fn model_weight_kg(tecido: &TecidoRecord, meters: f64) -> Result<f64, ShopeeError> {
    let grams = tecido.gramatura_linear_g_m.unwrap_or_default() as f64 * meters;
    if grams <= 0.0 {
        return Err(ShopeeError::Api(String::from(
            "gramatura linear invalida para calcular peso",
        )));
    }
    Ok(((grams / 1000.0) * 1000.0).round() / 1000.0)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
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
        parent_sku: sku.clone(),
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
    item: &Value,
    item_id: i64,
    item_name: &str,
    response: &Value,
) -> Vec<ShopeeStockOccurrence> {
    let Some(parent_sku) =
        normalize_sku(item.get("item_sku").and_then(Value::as_str).unwrap_or(""))
    else {
        return Vec::new();
    };
    let tier_one_options = stock_tier_one_options(response);
    response
        .get("model")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let sku = stock_child_sku(model, &tier_one_options)?;
            let model_id = model.get("model_id").and_then(Value::as_i64)?;
            let stock = model_stock(model);
            Some(ShopeeStockOccurrence {
                parent_sku: parent_sku.clone(),
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

fn stock_tier_one_options(response: &Value) -> Vec<String> {
    response
        .get("tier_variation")
        .and_then(Value::as_array)
        .and_then(|tiers| tiers.first())
        .and_then(|tier| tier.get("option_list"))
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| option.get("option").and_then(Value::as_str))
                .filter_map(normalize_sku)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn stock_child_sku(model: &Value, tier_one_options: &[String]) -> Option<String> {
    if let Some(tier_index) = model
        .get("tier_index")
        .and_then(Value::as_array)
        .and_then(|indexes| indexes.first())
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        && let Some(option) = tier_one_options.get(tier_index)
    {
        return Some(option.clone());
    }

    stock_child_sku_from_model_sku(model.get("model_sku").and_then(Value::as_str)?)
}

fn stock_child_sku_from_model_sku(value: &str) -> Option<String> {
    let sku = normalize_sku(value)?;
    let Some((maybe_size, color_sku)) = sku.split_once('-') else {
        return Some(sku);
    };
    if is_size_sku_part(maybe_size) {
        normalize_sku(color_sku)
    } else {
        Some(sku)
    }
}

fn is_size_sku_part(value: &str) -> bool {
    let size = value.trim_end_matches('M');
    let has_digit = size.chars().any(|char| char.is_ascii_digit());
    has_digit
        && size
            .chars()
            .all(|char| char.is_ascii_digit() || char == ',' || char == '.')
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

pub fn group_stock_occurrences_by_parent(
    occurrences: Vec<ShopeeStockOccurrence>,
) -> Vec<ShopeeStockParentGroup> {
    let mut map = HashMap::<String, Vec<ShopeeStockOccurrence>>::new();
    for occurrence in occurrences {
        map.entry(occurrence.parent_sku.clone())
            .or_default()
            .push(occurrence);
    }
    let mut parents = map
        .into_iter()
        .map(|(sku, occurrences)| {
            let name = occurrences
                .first()
                .map(|occurrence| occurrence.name.clone())
                .unwrap_or_else(|| String::from("sem nome"));
            let total_current_stock = occurrences
                .iter()
                .map(|occurrence| occurrence.seller_stock)
                .sum::<i64>();
            ShopeeStockParentGroup {
                sku,
                name,
                groups: group_stock_occurrences(occurrences),
                total_current_stock,
                expanded: false,
            }
        })
        .collect::<Vec<_>>();
    parents.sort_by(|left, right| left.sku.cmp(&right.sku));
    parents
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
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16)
        {
            output.push(hex);
            index += 3;
            continue;
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
                    parent_sku: String::from("ANAR"),
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
                    parent_sku: String::from("ANAR"),
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
    fn groups_stock_by_parent_sku_then_variation_sku() {
        let parents = group_stock_occurrences_by_parent(vec![
            ShopeeStockOccurrence {
                parent_sku: String::from("ANAR"),
                sku: String::from("ANAR-AZUL"),
                item_id: 1,
                model_id: 10,
                name: String::from("Anarruga"),
                seller_stock: 2,
                available_stock: 2,
                reserved_stock: 0,
                location_id: None,
                multi_location: false,
            },
            ShopeeStockOccurrence {
                parent_sku: String::from("ANAR"),
                sku: String::from("ANAR-BRANCO"),
                item_id: 1,
                model_id: 11,
                name: String::from("Anarruga"),
                seller_stock: 3,
                available_stock: 3,
                reserved_stock: 0,
                location_id: None,
                multi_location: false,
            },
        ]);

        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].sku, "ANAR");
        assert_eq!(parents[0].groups.len(), 2);
        assert_eq!(parents[0].total_current_stock, 5);
    }

    #[test]
    fn stock_child_sku_uses_tier_one_option_first() {
        let tier_one_options = vec![String::from("BORDO"), String::from("CAFE")];
        let model = json!({
            "tier_index": [1, 0],
            "model_sku": "050-BORDO"
        });

        assert_eq!(
            stock_child_sku(&model, &tier_one_options).as_deref(),
            Some("CAFE")
        );
    }

    #[test]
    fn stock_child_sku_falls_back_to_model_sku_size_prefix() {
        let tier_one_options = Vec::new();
        let model = json!({
            "model_sku": "050-BORDO"
        });

        assert_eq!(
            stock_child_sku(&model, &tier_one_options).as_deref(),
            Some("BORDO")
        );
    }

    #[test]
    fn stock_child_sku_from_model_sku_ignores_size_prefix() {
        assert_eq!(
            stock_child_sku_from_model_sku("050-BORDO").as_deref(),
            Some("BORDO")
        );
        assert_eq!(
            stock_child_sku_from_model_sku("1M-BORDO").as_deref(),
            Some("BORDO")
        );
        assert_eq!(
            stock_child_sku_from_model_sku("0,5M-BORDO").as_deref(),
            Some("BORDO")
        );
        assert_eq!(
            stock_child_sku_from_model_sku("0.5M-BORDO").as_deref(),
            Some("BORDO")
        );
        assert_eq!(
            stock_child_sku_from_model_sku("ANAR-BORDO").as_deref(),
            Some("ANAR-BORDO")
        );
    }

    #[test]
    fn groups_stock_by_color_when_model_sku_has_size_prefix() {
        let item = json!({"item_sku": "ANAR"});
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [{"option": "Bordo"}]},
                {"name": "Tamanho", "option_list": [{"option": "0,5m"}, {"option": "1m"}]}
            ],
            "model": [
                {
                    "tier_index": [0, 0],
                    "model_sku": "050-BORDO",
                    "model_id": 10,
                    "stock_info_v2": {"seller_stock": [{"stock": 2}]}
                },
                {
                    "tier_index": [0, 1],
                    "model_sku": "1M-BORDO",
                    "model_id": 11,
                    "stock_info_v2": {"seller_stock": [{"stock": 3}]}
                }
            ]
        });
        let parents =
            group_stock_occurrences_by_parent(model_occurrences(&item, 1, "Anarruga", &response));

        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].groups.len(), 1);
        assert_eq!(parents[0].groups[0].sku, "BORDO");
        assert_eq!(parents[0].groups[0].occurrences.len(), 2);
        assert_eq!(parents[0].groups[0].total_current_stock, 5);
    }

    #[test]
    fn item_without_variation_is_parent_and_child() {
        let parents = group_stock_occurrences_by_parent(vec![ShopeeStockOccurrence {
            parent_sku: String::from("HELA"),
            sku: String::from("HELA"),
            item_id: 1,
            model_id: 0,
            name: String::from("Helanca"),
            seller_stock: 4,
            available_stock: 4,
            reserved_stock: 0,
            location_id: None,
            multi_location: false,
        }]);

        assert_eq!(parents[0].sku, "HELA");
        assert_eq!(parents[0].groups[0].sku, "HELA");
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
            parent_sku: String::from("ABC"),
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
    fn listing_sizes_keep_color_combinations_under_one_hundred() {
        let sizes = listing_sizes(12);
        assert_eq!(sizes.len(), 3);
        assert_eq!(sizes[0].label, "0,5m");
        assert_eq!(sizes[1].label, "1m");
        assert_eq!(sizes[2].label, "2m");
        assert!(sizes.len() * 12 <= 100);
    }

    #[test]
    fn model_weight_uses_linear_grammage_by_meterage() {
        let tecido = test_tecido(250);

        assert_eq!(model_weight_kg(&tecido, 0.5).unwrap(), 0.125);
        assert_eq!(model_weight_kg(&tecido, 2.0).unwrap(), 0.5);
    }

    #[test]
    fn listing_models_reuse_link_sku_for_all_sizes() {
        let tecido = test_tecido(300);
        let vinculos = vec![VinculoRecord {
            cor_id: 1,
            tecido_nome: tecido.nome.clone(),
            cor_nome: String::from("Azul"),
            cor_hex: None,
            sku: Some(String::from("ANAR-AZUL")),
            tecido_custo_base: None,
            custo_override: None,
            custo_efetivo: None,
            has_imagem_original: false,
            has_imagem_brand: false,
            has_imagem_modelo: false,
            has_imagem_alternativa: false,
        }];
        let sizes = listing_sizes(1);
        let models = listing_models(&tecido, &vinculos, &sizes, 20.0).unwrap();

        assert_eq!(models.len(), 3);
        assert!(models.iter().all(|model| model.model_sku == "ANAR-AZUL"));
        assert_eq!(models[0].tier_index, [0, 0]);
        assert_eq!(models[1].tier_index, [0, 1]);
        assert_eq!(models[0].price, 10.0);
        assert_eq!(models[1].price, 20.0);
        assert_eq!(models[2].price, 40.0);
    }

    #[test]
    fn listing_update_adds_missing_color_with_remote_size_prices() {
        let tecido = test_tecido(300);
        let vinculos = vec![
            test_vinculo(&tecido, 1, "Azul", "ANAR-AZUL"),
            test_vinculo(&tecido, 2, "Bordo", "ANAR-BORDO"),
        ];
        let local_colors = listing_colors(&vinculos).unwrap();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [
                    {"option": "Azul", "image": {"image_id": "img1"}}
                ]},
                {"name": "Tamanho", "option_list": [
                    {"option": "0,5m"},
                    {"option": "1m"}
                ]}
            ],
            "model": [
                {"model_id": 10, "tier_index": [0, 0], "model_sku": "ANAR-AZUL", "price_info": [{"original_price": 12.5}]},
                {"model_id": 11, "tier_index": [0, 1], "model_sku": "ANAR-AZUL", "price_info": [{"original_price": 25.0}]}
            ]
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert!(plan.blocked_reason.is_none());
        assert_eq!(plan.missing_colors, vec![String::from("Bordo")]);
        assert_eq!(plan.model_count, 2);
        assert_eq!(plan.models_to_add[0]["tier_index"], json!([1, 0]));
        assert_eq!(plan.models_to_add[0]["original_price"], json!(12.5));
        assert_eq!(plan.models_to_add[1]["original_price"], json!(25.0));
        assert_eq!(plan.models_to_add[0]["model_sku"], json!("ANAR-BORDO"));
        assert_eq!(
            plan.tier_variation[0]["option_list"][1]["image"]["image_id"],
            json!("img1")
        );
    }

    #[test]
    fn listing_update_compares_sku_before_color_name() {
        let tecido = test_tecido(300);
        let vinculos = vec![
            test_vinculo(&tecido, 1, "Azul Local", "ANAR-AZUL"),
            test_vinculo(&tecido, 2, "Bordo", "ANAR-BORDO"),
        ];
        let local_colors = listing_colors(&vinculos).unwrap();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [
                    {"option": "Azul Shopee", "image": {"image_id": "img1"}}
                ]},
                {"name": "Tamanho", "option_list": [
                    {"option": "1m"}
                ]}
            ],
            "model": [
                {"model_id": 10, "tier_index": [0, 0], "model_sku": "ANAR-AZUL", "price_info": [{"original_price": 25.0}]}
            ]
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert_eq!(plan.missing_colors, vec![String::from("Bordo")]);
        assert_eq!(plan.model_count, 1);
        assert_eq!(plan.models_to_add[0]["model_sku"], json!("ANAR-BORDO"));
    }

    #[test]
    fn listing_update_falls_back_to_color_name_when_sku_differs() {
        let tecido = test_tecido(300);
        let vinculos = vec![
            test_vinculo(&tecido, 1, "Azul", "ANAR-AZUL-NOVO"),
            test_vinculo(&tecido, 2, "Bordo", "ANAR-BORDO"),
        ];
        let local_colors = listing_colors(&vinculos).unwrap();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [
                    {"option": "Azul", "image": {"image_id": "img1"}}
                ]},
                {"name": "Tamanho", "option_list": [
                    {"option": "1m"}
                ]}
            ],
            "model": [
                {"model_id": 10, "tier_index": [0, 0], "model_sku": "ANAR-AZUL-ANTIGO", "price_info": [{"original_price": 25.0}]}
            ]
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert_eq!(plan.missing_colors, vec![String::from("Bordo")]);
        assert_eq!(plan.model_count, 1);
        assert_eq!(plan.models_to_add[0]["model_sku"], json!("ANAR-BORDO"));
    }

    #[test]
    fn listing_update_adds_missing_model_when_color_already_exists() {
        let tecido = test_tecido(300);
        let vinculos = vec![test_vinculo(&tecido, 1, "Bordo", "ANAR-BORDO")];
        let local_colors = listing_colors(&vinculos).unwrap();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [
                    {"option": "Bordo", "image": {"image_id": "img1"}},
                    {"option": "Azul", "image": {"image_id": "img1"}}
                ]},
                {"name": "Tamanho", "option_list": [
                    {"option": "0,5m"},
                    {"option": "1m"}
                ]}
            ],
            "model": [
                {"model_id": 10, "tier_index": [0, 0], "model_sku": "ANAR-BORDO", "price_info": [{"original_price": 12.5}]},
                {"model_id": 11, "tier_index": [1, 0], "model_sku": "ANAR-AZUL", "price_info": [{"original_price": 12.5}]},
                {"model_id": 12, "tier_index": [1, 1], "model_sku": "ANAR-AZUL", "price_info": [{"original_price": 25.0}]}
            ]
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert!(plan.missing_colors.is_empty());
        assert!(!plan.needs_tier_update);
        assert_eq!(plan.model_count, 1);
        assert_eq!(plan.models_to_add[0]["tier_index"], json!([0, 1]));
        assert_eq!(plan.models_to_add[0]["original_price"], json!(25.0));
    }

    #[test]
    fn listing_update_sku_match_ignores_remote_size_prefix() {
        let tecido = test_tecido(300);
        let vinculos = vec![test_vinculo(&tecido, 1, "Bordo Local", "BORDO")];
        let local_colors = listing_colors(&vinculos).unwrap();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": [
                    {"option": "Bordo Shopee", "image": {"image_id": "img1"}}
                ]},
                {"name": "Tamanho", "option_list": [
                    {"option": "1m"}
                ]}
            ],
            "model": [
                {"model_id": 10, "tier_index": [0, 0], "model_sku": "1M-BORDO", "price_info": [{"original_price": 25.0}]}
            ]
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert!(plan.missing_colors.is_empty());
        assert_eq!(plan.model_count, 0);
    }

    #[test]
    fn listing_update_blocks_non_color_size_structure() {
        let tecido = test_tecido(300);
        let vinculos = vec![test_vinculo(&tecido, 1, "Azul", "ANAR-AZUL")];
        let response = json!({
            "tier_variation": [
                {"name": "Material", "option_list": [{"option": "Algodao"}]},
                {"name": "Tamanho", "option_list": [{"option": "1m"}]}
            ],
            "model": []
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &listing_colors(&vinculos).unwrap(),
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert_eq!(
            plan.blocked_reason.as_deref(),
            Some("variacoes nao sao Cor x Tamanho")
        );
    }

    #[test]
    fn listing_update_blocks_when_final_combinations_exceed_one_hundred() {
        let tecido = test_tecido(300);
        let vinculos = (0..11)
            .map(|index| test_vinculo(&tecido, index, &format!("Cor {index}"), "ANAR-COR"))
            .collect::<Vec<_>>();
        let local_colors = listing_colors(&vinculos).unwrap();
        let existing_sizes = (0..10)
            .map(|index| json!({"option": format!("{}m", index + 1)}))
            .collect::<Vec<_>>();
        let existing_models = (0..100)
            .map(|index| {
                json!({
                    "model_id": index,
                    "tier_index": [index / 10, index % 10],
                    "price_info": [{"original_price": 10.0}]
                })
            })
            .collect::<Vec<_>>();
        let response = json!({
            "tier_variation": [
                {"name": "Cor", "option_list": (0..10).map(|index| json!({"option": format!("Cor {index}")})).collect::<Vec<_>>()},
                {"name": "Tamanho", "option_list": existing_sizes}
            ],
            "model": existing_models
        });

        let plan = build_listing_update_plan(
            &tecido,
            &vinculos,
            &local_colors,
            100,
            "Anarruga",
            "ANAR",
            &response,
        )
        .unwrap();

        assert_eq!(
            plan.blocked_reason.as_deref(),
            Some("total de combinacoes passaria de 100")
        );
    }

    #[test]
    fn model_price_is_proportional_to_meterage() {
        assert_eq!(model_price(19.9, 0.5), 9.95);
        assert_eq!(model_price(19.9, 3.0), 59.7);
    }

    fn test_vinculo(
        tecido: &TecidoRecord,
        cor_id: impl Into<i64>,
        cor_nome: &str,
        sku: &str,
    ) -> VinculoRecord {
        VinculoRecord {
            cor_id: cor_id.into(),
            tecido_nome: tecido.nome.clone(),
            cor_nome: cor_nome.to_string(),
            cor_hex: None,
            sku: Some(sku.to_string()),
            tecido_custo_base: None,
            custo_override: None,
            custo_efetivo: None,
            has_imagem_original: false,
            has_imagem_brand: false,
            has_imagem_modelo: false,
            has_imagem_alternativa: false,
        }
    }

    fn test_tecido(gramatura_linear_g_m: i32) -> TecidoRecord {
        TecidoRecord {
            id: 1,
            nome: String::from("Anarruga"),
            sku: String::from("ANAR"),
            composicao: String::from("100% poliester"),
            largura_m: 1.5,
            custo_base: None,
            rendimento_m_kg: None,
            gramatura_linear_g_m: Some(gramatura_linear_g_m),
            gramatura_g_m2: Some(167),
            tipo: String::from("Liso"),
            transparencia: String::from("Media"),
            elasticidade: String::from("Baixa"),
            acabamento: String::from("Fosco"),
        }
    }

    #[test]
    fn redirect_url_is_percent_encoded() {
        assert_eq!(
            percent_encode("https://example.ngrok.app/shopee/callback"),
            "https%3A%2F%2Fexample.ngrok.app%2Fshopee%2Fcallback"
        );
    }
}

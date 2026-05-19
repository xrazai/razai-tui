use std::io;

use tokio::runtime::Runtime;

mod agent;
mod app;
mod db;
mod models;
mod screens;
mod shopee;
mod ui;
use app::App;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ratatui_image::picker::{Picker, ProtocolType};

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    let db_runtime = Runtime::new()?;
    let pool = db_runtime.block_on(db::connect()).ok();
    if let Some(pool) = &pool {
        let _ = db_runtime.block_on(db::ensure_configuracoes_table(pool));
        let _ = db_runtime.block_on(db::ensure_estampas_tables(pool));
        let _ = db_runtime.block_on(db::ensure_vendas_tables(pool));
        let _ = db_runtime.block_on(db::ensure_pedidos_tables(pool));
        let _ = db_runtime.block_on(db::ensure_vinculo_image_columns(pool));
    }
    let tecidos = match &pool {
        Some(pool) => db_runtime
            .block_on(db::list_tecidos(pool))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let cores = match &pool {
        Some(pool) => db_runtime
            .block_on(db::list_cores(pool))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let estampas = match &pool {
        Some(pool) => db_runtime
            .block_on(db::list_estampas(pool))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let selected_printer = match &pool {
        Some(pool) => db_runtime
            .block_on(db::get_config(pool, "receipt_printer"))
            .ok()
            .flatten(),
        None => None,
    };
    let color_delta_e_threshold = match &pool {
        Some(pool) => db_runtime
            .block_on(db::get_config(pool, "color_delta_e_threshold"))
            .ok()
            .flatten()
            .and_then(|value| value.replace(',', ".").parse::<f64>().ok())
            .filter(|value| *value > 0.0)
            .unwrap_or(3.0),
        None => 3.0,
    };
    let vendas_historico = match &pool {
        Some(pool) => db_runtime
            .block_on(db::list_vendas(pool))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let pedidos_historico = match &pool {
        Some(pool) => db_runtime
            .block_on(db::list_pedidos(pool))
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let shopee_ngrok_status = shopee::ensure_ngrok_tunnel();
    let shopee_listener_status = shopee::start_callback_listener(pool.clone()).ok();
    let shopee_connection_status = db_runtime.block_on(shopee::startup_status(pool.as_ref()));
    let shopee_status = match shopee_listener_status {
        Some(listener_status) => {
            format!("{shopee_connection_status}\n{shopee_ngrok_status}\n{listener_status}")
        }
        None => format!("{shopee_connection_status}\n{shopee_ngrok_status}"),
    };

    let mut terminal = setup_terminal()?;
    let (image_picker, image_protocol_status) = detect_image_picker();
    let app_result = App::new(
        pool,
        tecidos,
        cores,
        estampas,
        selected_printer,
        color_delta_e_threshold,
        vendas_historico,
        pedidos_historico,
        shopee_status,
        image_picker,
        image_protocol_status,
        db_runtime,
    )
    .run(&mut terminal);
    restore_terminal(&mut terminal)?;
    app_result
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()
}

fn detect_image_picker() -> (Picker, String) {
    let detected_picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    let override_value = std::env::var("RAZAI_IMAGE_PROTOCOL")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    let Some(override_value) = override_value else {
        let status = image_protocol_status(detected_picker.protocol_type(), false, false);
        return (detected_picker, status);
    };

    if override_value == "auto" {
        let status = image_protocol_status(detected_picker.protocol_type(), false, false);
        return (detected_picker, status);
    }

    let Some(protocol_type) = parse_image_protocol_override(&override_value) else {
        let status = image_protocol_status(detected_picker.protocol_type(), false, true);
        return (detected_picker, status);
    };

    let mut picker = detected_picker;
    picker.set_protocol_type(protocol_type);
    let status = image_protocol_status(protocol_type, true, false);
    (picker, status)
}

fn parse_image_protocol_override(value: &str) -> Option<ProtocolType> {
    match value {
        "halfblocks" | "halfblock" | "blocks" => Some(ProtocolType::Halfblocks),
        "sixel" => Some(ProtocolType::Sixel),
        "kitty" => Some(ProtocolType::Kitty),
        "iterm2" | "iterm" => Some(ProtocolType::Iterm2),
        _ => None,
    }
}

fn image_protocol_status(
    protocol_type: ProtocolType,
    forced_override: bool,
    invalid_override: bool,
) -> String {
    let label = match protocol_type {
        ProtocolType::Halfblocks => "Halfblocks",
        ProtocolType::Sixel => "Sixel",
        ProtocolType::Kitty => "Kitty",
        ProtocolType::Iterm2 => "iTerm2",
    };

    if invalid_override {
        format!("Preview: {label} fallback (override invalido)")
    } else if forced_override {
        format!("Preview: override {label}")
    } else if protocol_type == ProtocolType::Halfblocks {
        String::from("Preview: Halfblocks fallback")
    } else {
        format!("Preview: {label}")
    }
}

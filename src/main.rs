use std::io;

use tokio::runtime::Runtime;

mod agent;
mod app;
mod db;
mod models;
mod screens;
mod ui;
use app::App;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    let db_runtime = Runtime::new()?;
    let pool = db_runtime.block_on(db::connect()).ok();
    if let Some(pool) = &pool {
        let _ = db_runtime.block_on(db::ensure_configuracoes_table(pool));
        let _ = db_runtime.block_on(db::ensure_estampas_tables(pool));
        let _ = db_runtime.block_on(db::ensure_vendas_tables(pool));
        let _ = db_runtime.block_on(db::ensure_pedidos_tables(pool));
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

    let mut terminal = setup_terminal()?;
    let app_result = App::new(
        pool,
        tecidos,
        cores,
        estampas,
        selected_printer,
        vendas_historico,
        pedidos_historico,
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

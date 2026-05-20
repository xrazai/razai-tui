use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::*};

impl App {
    pub(super) fn handle_estoque_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.voltar_estoque(),
            KeyCode::Up => self.previous_estoque_option(),
            KeyCode::Down => self.next_estoque_option(),
            KeyCode::Left
                if self.estoque_screen == EstoqueScreen::Lista
                    && self.estoque_view == EstoqueView::ResumoFornecedor
                    && self.estoque_resumo_field == 0 =>
            {
                self.previous_estoque_resumo_fornecedor()
            }
            KeyCode::Right
                if self.estoque_screen == EstoqueScreen::Lista
                    && self.estoque_view == EstoqueView::ResumoFornecedor
                    && self.estoque_resumo_field == 0 =>
            {
                self.next_estoque_resumo_fornecedor()
            }
            KeyCode::Left => self.section = self.section.previous(),
            KeyCode::Right => self.section = self.section.next(),
            KeyCode::Backspace => self.backspace_estoque_field(),
            KeyCode::Enter => self.enter_estoque(),
            KeyCode::Char(character) if !character.is_control() => {
                self.push_estoque_field(character)
            }
            _ => {}
        }
    }

    pub fn reload_estoque_saldos(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_estoque_saldos(pool)) {
                Ok(saldos) => {
                    self.estoque_saldos = saldos;
                    self.estoque_option = self
                        .estoque_option
                        .min(self.estoque_saldos.len().saturating_sub(1));
                }
                Err(error) => self.db_status = format!("Erro ao carregar estoque: {error}"),
            }
        }
    }

    pub fn reload_estoque_ordens(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_estoque_ordens(pool)) {
                Ok(ordens) => {
                    self.estoque_ordens = ordens;
                    self.estoque_ordem_option = self
                        .estoque_ordem_option
                        .min(self.estoque_ordens.len().saturating_sub(1));
                }
                Err(error) => {
                    self.db_status = format!("Erro ao carregar ordens de estoque: {error}")
                }
            }
        }
    }

    fn reload_estoque_movimentos(&mut self) {
        let Some(saldo) = self.estoque_saldos.get(self.estoque_option) else {
            self.estoque_movimentos.clear();
            return;
        };
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_estoque_movimentos(
                pool,
                saldo.tecido_id,
                saldo.item_id,
                saldo.usa_estampas,
            )) {
                Ok(movimentos) => {
                    self.estoque_movimentos = movimentos;
                    self.estoque_movimento_option = self
                        .estoque_movimento_option
                        .min(self.estoque_movimentos.len().saturating_sub(1));
                }
                Err(error) => {
                    self.db_status = format!("Erro ao carregar historico do estoque: {error}")
                }
            }
        }
    }

    fn next_estoque_option(&mut self) {
        match self.estoque_screen {
            EstoqueScreen::Menu => {
                self.estoque_menu_option = (self.estoque_menu_option + 1) % 4;
            }
            EstoqueScreen::Lista
                if self.estoque_view == EstoqueView::Saldos && !self.estoque_saldos.is_empty() =>
            {
                self.estoque_option = (self.estoque_option + 1) % self.estoque_saldos.len();
            }
            EstoqueScreen::Lista
                if self.estoque_view == EstoqueView::Ordens && !self.estoque_ordens.is_empty() =>
            {
                self.estoque_ordem_option =
                    (self.estoque_ordem_option + 1) % self.estoque_ordens.len();
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::ResumoFornecedor => {
                self.next_estoque_resumo_field();
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::MaisVendidos => {}
            EstoqueScreen::Detalhe => {
                self.estoque_movimento_option = (self.estoque_movimento_option + 1) % 3;
            }
            EstoqueScreen::Movimento => {
                self.estoque_movimento_field = self.estoque_movimento_field.next();
                self.skip_destino_for_entrada();
            }
            EstoqueScreen::OrdemDetalhe => {
                self.estoque_ordem_action_option =
                    (self.estoque_ordem_action_option + 1) % self.estoque_ordem_action_count();
            }
            EstoqueScreen::OrdemFornecedor if !self.fornecedores.is_empty() => {
                self.estoque_ordem_fornecedor_option =
                    (self.estoque_ordem_fornecedor_option + 1) % self.fornecedores.len();
            }
            _ => {}
        }
    }

    fn previous_estoque_option(&mut self) {
        match self.estoque_screen {
            EstoqueScreen::Menu => {
                self.estoque_menu_option = (self.estoque_menu_option + 3) % 4;
            }
            EstoqueScreen::Lista
                if self.estoque_view == EstoqueView::Saldos && !self.estoque_saldos.is_empty() =>
            {
                self.estoque_option = (self.estoque_option + self.estoque_saldos.len() - 1)
                    % self.estoque_saldos.len();
            }
            EstoqueScreen::Lista
                if self.estoque_view == EstoqueView::Ordens && !self.estoque_ordens.is_empty() =>
            {
                self.estoque_ordem_option = (self.estoque_ordem_option + self.estoque_ordens.len()
                    - 1)
                    % self.estoque_ordens.len();
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::ResumoFornecedor => {
                self.previous_estoque_resumo_field();
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::MaisVendidos => {}
            EstoqueScreen::Detalhe => {
                self.estoque_movimento_option = (self.estoque_movimento_option + 2) % 3;
            }
            EstoqueScreen::Movimento => {
                self.estoque_movimento_field = self.estoque_movimento_field.previous();
                self.skip_destino_for_entrada();
            }
            EstoqueScreen::OrdemDetalhe => {
                let action_count = self.estoque_ordem_action_count();
                self.estoque_ordem_action_option =
                    (self.estoque_ordem_action_option + action_count - 1) % action_count;
            }
            EstoqueScreen::OrdemFornecedor if !self.fornecedores.is_empty() => {
                self.estoque_ordem_fornecedor_option =
                    (self.estoque_ordem_fornecedor_option + self.fornecedores.len() - 1)
                        % self.fornecedores.len();
            }
            _ => {}
        }
    }

    fn enter_estoque(&mut self) {
        match self.estoque_screen {
            EstoqueScreen::Menu => match self.estoque_menu_option {
                0 => {
                    self.estoque_view = EstoqueView::Saldos;
                    self.reload_estoque_saldos();
                    self.estoque_screen = EstoqueScreen::Lista;
                }
                1 => {
                    self.estoque_view = EstoqueView::Ordens;
                    self.reload_estoque_ordens();
                    self.estoque_screen = EstoqueScreen::Lista;
                }
                2 => {
                    self.estoque_view = EstoqueView::ResumoFornecedor;
                    self.reload_fornecedores();
                    self.reload_estoque_resumo_fornecedor();
                    self.estoque_screen = EstoqueScreen::Lista;
                }
                _ => {
                    self.estoque_view = EstoqueView::MaisVendidos;
                    self.reload_estoque_mais_vendidos();
                    self.estoque_screen = EstoqueScreen::Lista;
                }
            },
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::Saldos => {
                self.reload_estoque_movimentos();
                self.estoque_movimento_option = 0;
                self.estoque_screen = EstoqueScreen::Detalhe;
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::Ordens => {
                if !self.estoque_ordens.is_empty() {
                    self.estoque_ordem_action_option = 0;
                    self.estoque_screen = EstoqueScreen::OrdemDetalhe;
                }
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::ResumoFornecedor => {
                self.enter_estoque_resumo_fornecedor()
            }
            EstoqueScreen::Lista if self.estoque_view == EstoqueView::MaisVendidos => {}
            EstoqueScreen::Detalhe => match self.estoque_movimento_option {
                0 => self.open_estoque_movimento(EstoqueMovimentoTipo::Entrada),
                1 => self.open_estoque_movimento(EstoqueMovimentoTipo::Transferencia),
                _ => self.voltar_estoque(),
            },
            EstoqueScreen::Movimento => match self.estoque_movimento_field {
                EstoqueMovimentoField::Confirmar => self.salvar_estoque_movimento(),
                EstoqueMovimentoField::Voltar => self.voltar_estoque(),
                _ => self.next_estoque_option(),
            },
            EstoqueScreen::OrdemDetalhe => self.enter_estoque_ordem_detalhe(),
            EstoqueScreen::OrdemFornecedor => self.direcionar_estoque_ordem(),
            _ => {}
        }
    }

    fn open_estoque_movimento(&mut self, tipo: EstoqueMovimentoTipo) {
        self.estoque_movimento_tipo = tipo;
        self.estoque_movimento_field = EstoqueMovimentoField::Quantidade;
        self.estoque_quantidade.clear();
        self.estoque_destino.clear();
        self.estoque_observacao.clear();
        self.estoque_screen = EstoqueScreen::Movimento;
    }

    fn voltar_estoque(&mut self) {
        match self.estoque_screen {
            EstoqueScreen::Menu => self.section = Section::Dashboard,
            EstoqueScreen::Lista => self.estoque_screen = EstoqueScreen::Menu,
            EstoqueScreen::Detalhe => self.estoque_screen = EstoqueScreen::Lista,
            EstoqueScreen::Movimento => self.estoque_screen = EstoqueScreen::Detalhe,
            EstoqueScreen::OrdemDetalhe => self.estoque_screen = EstoqueScreen::Lista,
            EstoqueScreen::OrdemFornecedor => self.estoque_screen = EstoqueScreen::OrdemDetalhe,
        }
    }

    fn push_estoque_field(&mut self, character: char) {
        if self.estoque_screen != EstoqueScreen::Movimento {
            if self.estoque_screen == EstoqueScreen::Lista
                && self.estoque_view == EstoqueView::ResumoFornecedor
            {
                match self.estoque_resumo_field {
                    1 if is_date_input_char(character) && self.estoque_resumo_inicio.len() < 10 => {
                        self.estoque_resumo_inicio.push(character);
                    }
                    2 if is_date_input_char(character) && self.estoque_resumo_fim.len() < 10 => {
                        self.estoque_resumo_fim.push(character);
                    }
                    _ => {}
                }
            }
            return;
        }
        match self.estoque_movimento_field {
            EstoqueMovimentoField::Quantidade => self.estoque_quantidade.push(character),
            EstoqueMovimentoField::Destino
                if self.estoque_movimento_tipo == EstoqueMovimentoTipo::Transferencia =>
            {
                self.estoque_destino.push(character);
            }
            EstoqueMovimentoField::Observacao => self.estoque_observacao.push(character),
            _ => {}
        }
    }

    fn backspace_estoque_field(&mut self) {
        if self.estoque_screen != EstoqueScreen::Movimento {
            if self.estoque_screen == EstoqueScreen::Lista
                && self.estoque_view == EstoqueView::ResumoFornecedor
            {
                match self.estoque_resumo_field {
                    1 => {
                        self.estoque_resumo_inicio.pop();
                    }
                    2 => {
                        self.estoque_resumo_fim.pop();
                    }
                    _ => {}
                }
            }
            return;
        }
        match self.estoque_movimento_field {
            EstoqueMovimentoField::Quantidade => {
                self.estoque_quantidade.pop();
            }
            EstoqueMovimentoField::Destino => {
                self.estoque_destino.pop();
            }
            EstoqueMovimentoField::Observacao => {
                self.estoque_observacao.pop();
            }
            _ => {}
        }
    }

    fn salvar_estoque_movimento(&mut self) {
        let Some(saldo) = self.estoque_saldos.get(self.estoque_option) else {
            return;
        };
        let quantidade = parse_number(&self.estoque_quantidade).unwrap_or(0.0);
        if quantidade <= 0.0 {
            self.db_status = String::from("Informe quantidade maior que zero.");
            return;
        }
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para estoque.");
            return;
        };
        let tipo = match self.estoque_movimento_tipo {
            EstoqueMovimentoTipo::Entrada => "entrada",
            EstoqueMovimentoTipo::Transferencia => "saida_transferencia",
        };
        match self.db_runtime.block_on(db::insert_estoque_manual(
            pool,
            saldo.tecido_id,
            saldo.item_id,
            saldo.usa_estampas,
            tipo,
            quantidade,
            (self.estoque_movimento_tipo == EstoqueMovimentoTipo::Transferencia)
                .then_some(self.estoque_destino.as_str()),
            Some(self.estoque_observacao.as_str()),
        )) {
            Ok(()) => {
                let selected = (saldo.tecido_id, saldo.item_id, saldo.usa_estampas);
                self.reload_estoque_saldos();
                self.reload_estoque_ordens();
                if let Some(index) = self.estoque_saldos.iter().position(|saldo| {
                    (saldo.tecido_id, saldo.item_id, saldo.usa_estampas) == selected
                }) {
                    self.estoque_option = index;
                }
                self.reload_estoque_movimentos();
                self.estoque_screen = EstoqueScreen::Detalhe;
                self.db_status = String::from("Movimentacao de estoque registrada.");
            }
            Err(error) => self.db_status = format!("Erro ao salvar estoque: {error}"),
        }
    }

    fn skip_destino_for_entrada(&mut self) {
        if self.estoque_movimento_tipo == EstoqueMovimentoTipo::Entrada
            && self.estoque_movimento_field == EstoqueMovimentoField::Destino
        {
            self.estoque_movimento_field = EstoqueMovimentoField::Observacao;
        }
    }

    fn next_estoque_resumo_field(&mut self) {
        self.estoque_resumo_field = (self.estoque_resumo_field + 1) % 3;
    }

    fn previous_estoque_resumo_field(&mut self) {
        self.estoque_resumo_field = (self.estoque_resumo_field + 2) % 3;
    }

    fn next_estoque_resumo_fornecedor(&mut self) {
        if !self.fornecedores.is_empty() {
            self.estoque_resumo_fornecedor_option =
                (self.estoque_resumo_fornecedor_option + 1) % self.fornecedores.len();
            self.reload_estoque_resumo_fornecedor();
        }
    }

    fn previous_estoque_resumo_fornecedor(&mut self) {
        if !self.fornecedores.is_empty() {
            self.estoque_resumo_fornecedor_option =
                (self.estoque_resumo_fornecedor_option + self.fornecedores.len() - 1)
                    % self.fornecedores.len();
            self.reload_estoque_resumo_fornecedor();
        }
    }

    fn enter_estoque_resumo_fornecedor(&mut self) {
        if matches!(self.estoque_resumo_field, 1 | 2) {
            self.open_date_range_picker(DateRangeTarget::EstoqueResumoFornecedor);
        } else if self.estoque_resumo_field < 2 {
            self.estoque_resumo_field += 1;
        } else {
            self.estoque_resumo_field = 0;
        }
        self.reload_estoque_resumo_fornecedor();
    }

    pub(super) fn reload_estoque_resumo_fornecedor(&mut self) {
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para estoque.");
            return;
        };
        let Some(fornecedor) = self.fornecedores.get(self.estoque_resumo_fornecedor_option) else {
            self.estoque_resumo_fornecedor.clear();
            return;
        };
        let Some(mut inicio) = db::parse_sales_date(&self.estoque_resumo_inicio) else {
            self.estoque_resumo_fornecedor.clear();
            self.db_status = String::from("Data inicial invalida. Use AAAA-MM-DD.");
            return;
        };
        let Some(mut fim) = db::parse_sales_date(&self.estoque_resumo_fim) else {
            self.estoque_resumo_fornecedor.clear();
            self.db_status = String::from("Data final invalida. Use AAAA-MM-DD.");
            return;
        };
        if fim < inicio {
            std::mem::swap(&mut inicio, &mut fim);
            self.estoque_resumo_inicio = inicio.format("%Y-%m-%d").to_string();
            self.estoque_resumo_fim = fim.format("%Y-%m-%d").to_string();
        }
        match self.db_runtime.block_on(db::list_fornecedor_resumo_vendas(
            pool,
            fornecedor.id,
            inicio,
            fim,
        )) {
            Ok(resumo) => self.estoque_resumo_fornecedor = resumo,
            Err(error) => self.db_status = format!("Erro ao carregar resumo fornecedor: {error}"),
        }
    }

    fn reload_estoque_mais_vendidos(&mut self) {
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para estoque.");
            return;
        };
        match self.db_runtime.block_on(db::list_mais_vendidos(pool)) {
            Ok(records) => self.estoque_mais_vendidos = records,
            Err(error) => self.db_status = format!("Erro ao carregar mais vendidos: {error}"),
        }
    }

    fn open_estoque_ordem_fornecedor(&mut self) {
        if !self.estoque_ordem_selected_is_active() {
            self.db_status = String::from("Ordem encerrada nao pode ser direcionada.");
            return;
        }
        self.reload_fornecedores();
        if self.fornecedores.is_empty() {
            self.db_status = String::from("Cadastre um fornecedor em Dados > Fornecedor.");
            return;
        }
        if let Some(ordem) = self.estoque_ordens.get(self.estoque_ordem_option) {
            self.estoque_ordem_fornecedor_option = ordem
                .fornecedor_id
                .and_then(|id| {
                    self.fornecedores
                        .iter()
                        .position(|fornecedor| fornecedor.id == id)
                })
                .unwrap_or(0);
        }
        self.estoque_screen = EstoqueScreen::OrdemFornecedor;
    }

    fn direcionar_estoque_ordem(&mut self) {
        let Some(ordem) = self.estoque_ordens.get(self.estoque_ordem_option) else {
            return;
        };
        let Some(fornecedor) = self.fornecedores.get(self.estoque_ordem_fornecedor_option) else {
            self.db_status = String::from("Selecione um fornecedor.");
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para estoque.");
            return;
        };
        match self
            .db_runtime
            .block_on(db::direcionar_estoque_ordem(pool, ordem.id, fornecedor.id))
        {
            Ok(true) => {
                let ordem_id = ordem.id;
                self.reload_estoque_ordens();
                if let Some(index) = self
                    .estoque_ordens
                    .iter()
                    .position(|ordem| ordem.id == ordem_id)
                {
                    self.estoque_ordem_option = index;
                }
                self.estoque_screen = EstoqueScreen::OrdemDetalhe;
                self.db_status = String::from("Ordem direcionada para fornecedor.");
            }
            Ok(false) => {
                self.reload_estoque_ordens();
                self.estoque_screen = EstoqueScreen::OrdemDetalhe;
                self.db_status = String::from("Ordem encerrada nao pode ser direcionada.");
            }
            Err(error) => self.db_status = format!("Erro ao direcionar ordem: {error}"),
        }
    }

    fn update_estoque_ordem_status(&mut self, status: &str) {
        let Some(ordem) = self.estoque_ordens.get(self.estoque_ordem_option) else {
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para estoque.");
            return;
        };
        match self
            .db_runtime
            .block_on(db::update_estoque_ordem_status(pool, ordem.id, status))
        {
            Ok(true) => {
                let ordem_id = ordem.id;
                self.reload_estoque_ordens();
                if let Some(index) = self
                    .estoque_ordens
                    .iter()
                    .position(|ordem| ordem.id == ordem_id)
                {
                    self.estoque_ordem_option = index;
                    self.estoque_screen = EstoqueScreen::OrdemDetalhe;
                } else {
                    self.estoque_screen = EstoqueScreen::Lista;
                }
                self.db_status = format!("Ordem marcada como {status}.");
            }
            Ok(false) => {
                self.reload_estoque_ordens();
                self.estoque_ordem_action_option = 0;
                self.db_status = String::from("Ordem encerrada nao pode ser alterada.");
            }
            Err(error) => self.db_status = format!("Erro ao atualizar ordem: {error}"),
        }
    }

    fn enter_estoque_ordem_detalhe(&mut self) {
        if self.estoque_ordem_selected_is_active() {
            match self.estoque_ordem_action_option {
                0 => self.open_estoque_ordem_fornecedor(),
                1 => self.update_estoque_ordem_status("concluida"),
                2 => self.update_estoque_ordem_status("cancelada"),
                _ => self.voltar_estoque(),
            }
        } else {
            self.voltar_estoque();
        }
    }

    fn estoque_ordem_action_count(&self) -> usize {
        if self.estoque_ordem_selected_is_active() {
            4
        } else {
            1
        }
    }

    fn estoque_ordem_selected_is_active(&self) -> bool {
        self.estoque_ordens
            .get(self.estoque_ordem_option)
            .is_some_and(|ordem| matches!(ordem.status.as_str(), "pendente" | "direcionada"))
    }
}

fn is_date_input_char(character: char) -> bool {
    character.is_ascii_digit() || character == '-'
}

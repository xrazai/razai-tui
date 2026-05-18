use super::App;
use crate::{db, models::*};

impl App {
    pub(crate) fn open_edit_venda(&mut self) {
        let Some(venda) = self.vendas_historico.get(self.venda_historico_option) else {
            return;
        };
        let venda_id = venda.id;
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para editar venda");
            return;
        };
        match self
            .db_runtime
            .block_on(db::list_venda_itens(pool, venda_id))
        {
            Ok(itens) => {
                self.editing_venda_id = Some(venda_id);
                self.venda_itens = itens;
                self.venda_item_option = 0;
                self.reset_venda_item_editing();
                self.venda_field = VendaField::Preco;
                self.venda_dropdown = None;
                self.venda_preco.clear();
                self.venda_quantidade.clear();
                self.venda_tecido_option = self
                    .venda_tecido_option
                    .min(self.tecidos.len().saturating_sub(1));
                self.reload_venda_vinculos_for_current_tecido();
                self.vendas_screen = VendasScreen::Lancamento;
                self.db_status = format!("Editando venda #{venda_id}");
            }
            Err(error) => self.db_status = format!("Erro ao abrir venda: {error}"),
        }
    }

    pub(crate) fn reload_vendas_historico(&mut self) {
        if let Some(pool) = &self.db_pool {
            let Some(inicio) = db::parse_sales_date(&self.venda_historico_inicio) else {
                self.db_status = String::from("Data inicio invalida. Use AAAA-MM-DD");
                return;
            };
            let Some(fim) = db::parse_sales_date(&self.venda_historico_fim) else {
                self.db_status = String::from("Data fim invalida. Use AAAA-MM-DD");
                return;
            };
            let (inicio, fim) = if inicio <= fim {
                (inicio, fim)
            } else {
                (fim, inicio)
            };
            match self
                .db_runtime
                .block_on(db::list_vendas_periodo(pool, inicio, fim))
            {
                Ok(vendas) => {
                    self.vendas_historico = vendas;
                    self.venda_historico_option = self
                        .venda_historico_option
                        .min(self.vendas_historico.len().saturating_sub(1));
                    self.db_status = format!(
                        "Historico de vendas: {} ate {}",
                        self.venda_historico_inicio, self.venda_historico_fim
                    );
                }
                Err(error) => self.db_status = format!("Erro ao carregar historico: {error}"),
            }
        }
    }

    pub(super) fn push_venda_historico_field(&mut self, character: char) {
        if !(character.is_ascii_digit() || character == '-') {
            return;
        }
        let field = if self.venda_historico_field == 0 {
            &mut self.venda_historico_inicio
        } else if self.venda_historico_field == 1 {
            &mut self.venda_historico_fim
        } else {
            return;
        };
        if field.len() < 10 {
            field.push(character);
        }
    }

    pub(super) fn backspace_venda_historico_field(&mut self) {
        if self.venda_historico_field == 0 {
            self.venda_historico_inicio.pop();
        } else if self.venda_historico_field == 1 {
            self.venda_historico_fim.pop();
        }
    }
}

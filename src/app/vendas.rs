use std::{fs, process::Command};

use crossterm::event::KeyCode;

use super::App;
use crate::{db, models::*};

impl App {
    pub(super) fn handle_vendas_key(&mut self, key: KeyCode) {
        if self.pending_delete_venda {
            match key {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.pending_delete_venda = false;
                }
                KeyCode::Enter | KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.excluir_venda_confirmada();
                }
                _ => {}
            }
            return;
        }

        if self.finalizar_venda_dialog {
            match key {
                KeyCode::Esc => self.finalizar_venda_dialog = false,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down | KeyCode::Tab => {
                    self.finalizar_venda_option = self.finalizar_venda_option.next();
                }
                KeyCode::BackTab => {
                    self.finalizar_venda_option = self.finalizar_venda_option.previous();
                }
                KeyCode::Enter => self.confirmar_finalizacao_venda(),
                _ => {}
            }
            return;
        }

        if self.vendas_screen == VendasScreen::Lancamento && self.venda_dropdown.is_some() {
            match key {
                KeyCode::Esc | KeyCode::Enter => self.venda_dropdown = None,
                KeyCode::Up => self.previous_venda_dropdown_option(),
                KeyCode::Down => self.next_venda_dropdown_option(),
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => self.voltar_vendas(),
            KeyCode::Backspace => self.backspace_venda_field(),
            KeyCode::Up => self.previous_venda_option(),
            KeyCode::Down => self.next_venda_option(),
            KeyCode::Left | KeyCode::BackTab => self.section = self.section.previous(),
            KeyCode::Right | KeyCode::Tab => self.section = self.section.next(),
            KeyCode::Enter => self.enter_vendas(),
            KeyCode::Char(character)
                if !character.is_control() && self.vendas_screen == VendasScreen::Lancamento =>
            {
                self.push_venda_field(character);
            }
            _ => {}
        }
    }

    pub(super) fn next_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option = (self.venda_tecido_option + 1) % self.tecidos.len();
                }
            }
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + 1) % self.venda_vinculos.len();
                }
            }
            VendasScreen::Lancamento => {
                self.venda_field = self.venda_field.next();
                self.normalize_venda_field();
            }
            VendasScreen::Historico => {
                if !self.vendas_historico.is_empty() {
                    self.venda_historico_option =
                        (self.venda_historico_option + 1) % self.vendas_historico.len();
                }
            }
        }
    }

    pub(super) fn previous_venda_option(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => self.venda_menu_option = (self.venda_menu_option + 1) % 2,
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option =
                        (self.venda_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                }
            }
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + self.venda_vinculos.len() - 1)
                            % self.venda_vinculos.len();
                }
            }
            VendasScreen::Lancamento => {
                self.venda_field = self.venda_field.previous();
                self.normalize_venda_field();
            }
            VendasScreen::Historico => {
                if !self.vendas_historico.is_empty() {
                    self.venda_historico_option =
                        (self.venda_historico_option + self.vendas_historico.len() - 1)
                            % self.vendas_historico.len();
                }
            }
        }
    }

    pub(super) fn enter_vendas(&mut self) {
        match self.vendas_screen {
            VendasScreen::Menu => {
                if self.venda_menu_option == 0 {
                    self.vendas_screen = VendasScreen::SelecionarTecido;
                    self.venda_tecido_option = 0;
                    self.venda_itens.clear();
                    self.editing_venda_id = None;
                    self.pending_delete_venda = false;
                } else {
                    self.reload_vendas_historico();
                    self.vendas_screen = VendasScreen::Historico;
                }
            }
            VendasScreen::SelecionarTecido => {
                if !self.tecidos.is_empty() {
                    self.open_venda_vinculos();
                }
            }
            VendasScreen::SelecionarVinculo => {
                if !self.venda_vinculos.is_empty() {
                    self.vendas_screen = VendasScreen::Lancamento;
                    self.venda_field = VendaField::Preco;
                    self.venda_dropdown = None;
                    self.venda_preco.clear();
                    self.venda_quantidade.clear();
                }
            }
            VendasScreen::Lancamento => {
                if matches!(self.venda_field, VendaField::Tecido | VendaField::Vinculo) {
                    self.venda_dropdown = Some(self.venda_field);
                    return;
                }
                if self.venda_field == VendaField::Quantidade {
                    self.confirmar_lancamento();
                } else if self.venda_field == VendaField::Finalizar {
                    self.abrir_dialog_finalizar_venda();
                } else if self.venda_field == VendaField::Cancelar {
                    self.cancelar_venda();
                } else if self.venda_field == VendaField::Excluir {
                    self.pending_delete_venda = self.editing_venda_id.is_some();
                } else {
                    self.venda_field = self.venda_field.next();
                }
            }
            VendasScreen::Historico => self.open_edit_venda(),
        }
    }

    pub(super) fn open_edit_venda(&mut self) {
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

    pub(super) fn open_venda_vinculos(&mut self) {
        let Some(tecido) = self.tecidos.get(self.venda_tecido_option) else {
            self.venda_vinculos.clear();
            return;
        };
        let usa_estampas = tecido.tipo == "Estampado";
        if let Some(pool) = &self.db_pool {
            let result = if usa_estampas {
                self.db_runtime
                    .block_on(db::list_estampa_vinculos_by_tecido(pool, tecido.id))
            } else {
                self.db_runtime
                    .block_on(db::list_vinculos_by_tecido(pool, tecido.id))
            };
            match result {
                Ok(vinculos) => {
                    self.venda_vinculos = vinculos;
                    self.db_status = if usa_estampas {
                        String::from("Venda: listando estampas vinculadas ao tecido")
                    } else {
                        String::from("Venda: listando cores vinculadas ao tecido")
                    };
                }
                Err(error) => self.db_status = format!("Erro ao carregar vinculos venda: {error}"),
            }
        } else {
            self.venda_vinculos.clear();
        }
        self.venda_vinculo_option = 0;
        self.vendas_screen = VendasScreen::SelecionarVinculo;
    }

    pub(super) fn reload_venda_vinculos_for_current_tecido(&mut self) {
        let current_screen = self.vendas_screen;
        self.open_venda_vinculos();
        self.vendas_screen = current_screen;
    }

    pub(super) fn next_venda_dropdown_option(&mut self) {
        match self.venda_dropdown {
            Some(VendaField::Tecido) => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option = (self.venda_tecido_option + 1) % self.tecidos.len();
                    self.reload_venda_vinculos_for_current_tecido();
                }
            }
            Some(VendaField::Vinculo) => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + 1) % self.venda_vinculos.len();
                }
            }
            _ => {}
        }
    }

    pub(super) fn previous_venda_dropdown_option(&mut self) {
        match self.venda_dropdown {
            Some(VendaField::Tecido) => {
                if !self.tecidos.is_empty() {
                    self.venda_tecido_option =
                        (self.venda_tecido_option + self.tecidos.len() - 1) % self.tecidos.len();
                    self.reload_venda_vinculos_for_current_tecido();
                }
            }
            Some(VendaField::Vinculo) => {
                if !self.venda_vinculos.is_empty() {
                    self.venda_vinculo_option =
                        (self.venda_vinculo_option + self.venda_vinculos.len() - 1)
                            % self.venda_vinculos.len();
                }
            }
            _ => {}
        }
    }

    pub(super) fn confirmar_lancamento(&mut self) {
        let Some(vinculo) = self.venda_vinculos.get(self.venda_vinculo_option) else {
            return;
        };
        let preco = parse_number(&self.venda_preco).unwrap_or(0.0);
        let quantidade = parse_number(&self.venda_quantidade).unwrap_or(0.0);
        if preco <= 0.0 || quantidade <= 0.0 {
            return;
        }
        self.venda_itens.push(VendaItem {
            descricao: format!("{} - {}", vinculo.tecido_nome, vinculo.cor_nome),
            quantidade,
            preco_unitario: preco,
        });
        self.venda_dropdown = None;
        self.venda_quantidade.clear();
    }

    pub(super) fn abrir_dialog_finalizar_venda(&mut self) {
        if self.venda_itens.is_empty() {
            self.db_status = String::from("Adicione ao menos um item antes de finalizar");
            return;
        }

        self.finalizar_venda_dialog = true;
        self.finalizar_venda_option = FinalizarVendaOption::Finalizar;
    }

    pub(super) fn confirmar_finalizacao_venda(&mut self) {
        let imprimir = self.finalizar_venda_option == FinalizarVendaOption::FinalizarEImprimir;
        if let Some(pool) = &self.db_pool {
            let result = if let Some(venda_id) = self.editing_venda_id {
                self.db_runtime
                    .block_on(db::update_venda(pool, venda_id, &self.venda_itens))
            } else {
                self.db_runtime
                    .block_on(db::insert_venda(pool, &self.venda_itens))
                    .map(|_| ())
            };
            if let Err(error) = result {
                self.db_status = format!("Erro ao salvar venda: {error}");
                return;
            }
            self.reload_vendas_historico();
        }

        self.db_status = if self.editing_venda_id.is_some() {
            String::from("Venda atualizada no historico")
        } else {
            String::from("Venda registrada no historico")
        };
        if imprimir {
            self.db_status = match &self.selected_printer {
                Some(printer) => match self.imprimir_recibo(printer) {
                    Ok(()) => format!("Venda salva. Recibo enviado para {printer}"),
                    Err(error) => format!("Venda salva. Erro ao imprimir: {error}"),
                },
                None => String::from("Venda salva. Configure uma impressora para imprimir"),
            };
        }
        self.finalizar_venda_dialog = false;
        self.vendas_screen = VendasScreen::Menu;
        self.venda_dropdown = None;
        self.venda_preco.clear();
        self.venda_quantidade.clear();
        self.venda_itens.clear();
        self.editing_venda_id = None;
        self.pending_delete_venda = false;
    }

    pub(super) fn reload_vendas_historico(&mut self) {
        if let Some(pool) = &self.db_pool {
            match self.db_runtime.block_on(db::list_vendas(pool)) {
                Ok(vendas) => self.vendas_historico = vendas,
                Err(error) => self.db_status = format!("Erro ao carregar historico: {error}"),
            }
        }
    }

    pub(super) fn cancelar_venda(&mut self) {
        self.venda_itens.clear();
        self.venda_vinculos.clear();
        self.finalizar_venda_dialog = false;
        self.venda_dropdown = None;
        self.venda_preco.clear();
        self.venda_quantidade.clear();
        self.venda_tecido_option = 0;
        self.venda_vinculo_option = 0;
        self.vendas_screen = VendasScreen::Menu;
        self.editing_venda_id = None;
        self.pending_delete_venda = false;
        self.db_status = String::from("Venda cancelada");
    }

    pub(super) fn excluir_venda_confirmada(&mut self) {
        let Some(venda_id) = self.editing_venda_id else {
            self.pending_delete_venda = false;
            return;
        };
        let Some(pool) = &self.db_pool else {
            self.db_status = String::from("Banco local indisponivel para excluir venda");
            self.pending_delete_venda = false;
            return;
        };

        match self.db_runtime.block_on(db::delete_venda(pool, venda_id)) {
            Ok(()) => {
                self.reload_vendas_historico();
                self.venda_historico_option = self
                    .venda_historico_option
                    .min(self.vendas_historico.len().saturating_sub(1));
                self.venda_itens.clear();
                self.venda_vinculos.clear();
                self.venda_preco.clear();
                self.venda_quantidade.clear();
                self.venda_dropdown = None;
                self.editing_venda_id = None;
                self.pending_delete_venda = false;
                self.finalizar_venda_dialog = false;
                self.vendas_screen = VendasScreen::Historico;
                self.db_status = format!("Venda #{venda_id} excluida");
            }
            Err(error) => {
                self.pending_delete_venda = false;
                self.db_status = format!("Erro ao excluir venda: {error}");
            }
        }
    }

    pub(super) fn push_venda_field(&mut self, character: char) {
        match self.venda_field {
            VendaField::Tecido | VendaField::Vinculo => {}
            VendaField::Preco => self.venda_preco.push(character),
            VendaField::Quantidade => self.venda_quantidade.push(character),
            VendaField::Finalizar | VendaField::Cancelar | VendaField::Excluir => {}
        }
    }

    pub(super) fn backspace_venda_field(&mut self) {
        match self.venda_field {
            VendaField::Tecido | VendaField::Vinculo => {}
            VendaField::Preco => {
                self.venda_preco.pop();
            }
            VendaField::Quantidade => {
                self.venda_quantidade.pop();
            }
            VendaField::Finalizar | VendaField::Cancelar | VendaField::Excluir => {}
        }
    }

    fn normalize_venda_field(&mut self) {
        if self.editing_venda_id.is_none() && self.venda_field == VendaField::Excluir {
            self.venda_field = VendaField::Tecido;
        }
    }

    pub(super) fn voltar_vendas(&mut self) {
        if self.editing_venda_id.is_some() && self.vendas_screen == VendasScreen::Lancamento {
            self.vendas_screen = VendasScreen::Historico;
            self.editing_venda_id = None;
            self.pending_delete_venda = false;
            self.finalizar_venda_dialog = false;
            self.venda_dropdown = None;
            return;
        }

        self.vendas_screen = match self.vendas_screen {
            VendasScreen::Menu => VendasScreen::Menu,
            VendasScreen::SelecionarTecido | VendasScreen::Historico => VendasScreen::Menu,
            VendasScreen::SelecionarVinculo => VendasScreen::SelecionarTecido,
            VendasScreen::Lancamento => VendasScreen::SelecionarVinculo,
        };
        self.finalizar_venda_dialog = false;
        self.venda_dropdown = None;
        self.pending_delete_venda = false;
        if self.vendas_screen != VendasScreen::Lancamento {
            self.editing_venda_id = None;
        }
    }

    fn imprimir_recibo(&self, printer: &str) -> Result<(), String> {
        let recibo = self.montar_recibo_impresso();
        let path = std::env::temp_dir().join("razai_recibo_venda.txt");
        fs::write(&path, recibo.as_bytes())
            .map_err(|error| format!("falha ao criar recibo: {error}"))?;

        let script = r#"
param($Path, $Printer)
$source = @'
using System;
using System.Runtime.InteropServices;
public class RawPrinterHelper {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Ansi)]
  public class DOCINFOA {
    [MarshalAs(UnmanagedType.LPStr)] public string pDocName;
    [MarshalAs(UnmanagedType.LPStr)] public string pOutputFile;
    [MarshalAs(UnmanagedType.LPStr)] public string pDataType;
  }
  [DllImport("winspool.Drv", EntryPoint="OpenPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool OpenPrinter(string szPrinter, out IntPtr hPrinter, IntPtr pd);
  [DllImport("winspool.Drv", EntryPoint="ClosePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool ClosePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartDocPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartDocPrinter(IntPtr hPrinter, Int32 level, [In, MarshalAs(UnmanagedType.LPStruct)] DOCINFOA di);
  [DllImport("winspool.Drv", EntryPoint="EndDocPrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndDocPrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="EndPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="WritePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool WritePrinter(IntPtr hPrinter, byte[] pBytes, Int32 dwCount, out Int32 dwWritten);
  public static bool SendBytes(string printerName, byte[] bytes) {
    IntPtr hPrinter;
    DOCINFOA di = new DOCINFOA();
    di.pDocName = "Razai Recibo";
    di.pDataType = "RAW";
    if(!OpenPrinter(printerName.Normalize(), out hPrinter, IntPtr.Zero)) return false;
    try {
      if(!StartDocPrinter(hPrinter, 1, di)) return false;
      try {
        if(!StartPagePrinter(hPrinter)) return false;
        try { int written; return WritePrinter(hPrinter, bytes, bytes.Length, out written); }
        finally { EndPagePrinter(hPrinter); }
      } finally { EndDocPrinter(hPrinter); }
    } finally { ClosePrinter(hPrinter); }
  }
}
'@
Add-Type -TypeDefinition $source
$text = Get-Content -LiteralPath $Path -Raw
$encoding = [System.Text.Encoding]::GetEncoding(860)
$bytes = [byte[]](0x1B,0x40) + $encoding.GetBytes($text) + [byte[]](0x0A,0x0A,0x0A,0x1D,0x56,0x41,0x10)
if (-not [RawPrinterHelper]::SendBytes($Printer, $bytes)) {
  throw "Falha ao enviar RAW para a impressora $Printer"
}
"#;
        let script_path = std::env::temp_dir().join("razai_print_raw.ps1");
        fs::write(&script_path, script)
            .map_err(|error| format!("falha ao criar script de impressao: {error}"))?;
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script_path.to_string_lossy().as_ref(),
                path.to_string_lossy().as_ref(),
                printer,
            ])
            .output()
            .map_err(|error| format!("falha ao chamar impressora: {error}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(if stderr.is_empty() {
                String::from("comando de impressao retornou erro")
            } else {
                stderr
            })
        }
    }

    fn montar_recibo_impresso(&self) -> String {
        let mut receipt = String::new();
        receipt.push_str("\x1B\x45\x01");
        receipt.push_str(&normalize_receipt_text("RAZAI\r\n"));
        receipt.push_str(&normalize_receipt_text("RECIBO DE VENDA\r\n"));
        receipt.push_str("\x1B\x45\x00");
        receipt.push_str("----------------------------------------\r\n");

        for item in &self.venda_itens {
            receipt.push_str(&normalize_receipt_text(&item.descricao));
            receipt.push_str("\r\n");
            receipt.push_str(&format!(
                "QTD: {} x R${} - Total: R${}\r\n\r\n",
                format_quantity(item.quantidade),
                format_money(item.preco_unitario),
                format_money(item.total())
            ));
        }

        let total = self.venda_itens.iter().map(VendaItem::total).sum::<f64>();
        receipt.push_str("----------------------------------------\r\n");
        receipt.push_str("\x1B\x45\x01\x1D\x21\x11");
        receipt.push_str(&format!("TOTAL: R${}\r\n", format_money(total)));
        receipt.push_str("\x1D\x21\x00\x1B\x45\x00\r\n\r\n\r\n");
        receipt
    }
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_quantity(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format_money(value)
    }
}

fn normalize_receipt_text(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'á' | 'à' | 'ã' | 'â' | 'ä' => 'a',
            'Á' | 'À' | 'Ã' | 'Â' | 'Ä' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'ó' | 'ò' | 'õ' | 'ô' | 'ö' => 'o',
            'Ó' | 'Ò' | 'Õ' | 'Ô' | 'Ö' => 'O',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ñ' => 'n',
            'Ñ' => 'N',
            '–' | '—' => '-',
            '“' | '”' => '"',
            '‘' | '’' => '\'',
            character if character.is_ascii() => character,
            _ => ' ',
        })
        .collect()
}

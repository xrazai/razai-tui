use std::{fs, process::Command};

use crate::{app::App, models::VendaItem};

impl App {
    pub(super) fn imprimir_recibo(&self, printer: &str) -> Result<(), String> {
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

        let total_quantity = self
            .venda_itens
            .iter()
            .map(|item| item.quantidade)
            .sum::<f64>();
        let total = self.venda_itens.iter().map(VendaItem::total).sum::<f64>();
        receipt.push_str("----------------------------------------\r\n");
        receipt.push_str("\x1B\x45\x01\x1D\x21\x11");
        receipt.push_str(&format!(
            "QTD TOTAL: {}\r\n",
            format_quantity(total_quantity)
        ));
        receipt.push_str(&format!("TOTAL: R${}\r\n", format_money(total)));
        receipt.push_str("\x1D\x21\x00\x1B\x45\x00\r\n\r\n\r\n");
        receipt
    }
}

fn format_money(value: f64) -> String {
    format!("{value:.2}").replace('.', ",")
}

fn format_quantity(value: f64) -> String {
    format_money(value)
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

use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use windows::{
    ApplicationModel::DataTransfer::{
        DataPackageOperation, DataRequestedEventArgs, DataTransferManager,
    },
    Foundation::TypedEventHandler,
    Storage::{IStorageItem, StorageFile},
    Win32::{
        Foundation::{HWND, RPC_E_CHANGED_MODE},
        System::WinRT::{RO_INIT_SINGLETHREADED, RoInitialize, RoUninitialize},
        UI::{
            Shell::{IDataTransferManagerInterop, ShellExecuteW},
            WindowsAndMessaging::{
                CW_USEDEFAULT, CreateWindowExW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE,
                PeekMessageW, SW_SHOW, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
                WS_EX_TOOLWINDOW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
    core::{HSTRING, Interface, PCWSTR, factory, w},
};

pub(super) enum PrintPdfOutcome {
    Printed,
    Opened { print_error: String },
}

pub(super) fn share_pdf_with_windows_ui(
    context: &str,
    path: &Path,
    title: &str,
    description: &str,
    text: &str,
) -> Result<(), String> {
    log_pdf_debug(&format!("{context}: compartilhamento solicitado"));
    match show_share_ui(context, path, title, description, text) {
        Ok(()) => Ok(()),
        Err(error) => {
            log_pdf_error(&format!("{context}: {error}"));
            let fallback_path = canonical_pdf_path(path, "PDF")?;
            if let Err(fallback_error) = select_pdf_in_explorer(&fallback_path) {
                log_pdf_error(&format!(
                    "{context}: fallback Explorer falhou: {fallback_error}"
                ));
                return Err(format!(
                    "Compartilhamento nativo indisponivel ({error}). Fallback Explorer falhou: {fallback_error}."
                ));
            }
            Err(format!(
                "Compartilhamento nativo indisponivel; PDF selecionado no Explorer: {}",
                fallback_path.display()
            ))
        }
    }
}

pub(super) fn print_or_open_pdf(path: &Path) -> Result<PrintPdfOutcome, String> {
    let path = canonical_pdf_path(path, "PDF")?;
    log_pdf_debug(&format!("Impressao solicitada para {}", path.display()));
    match shell_execute_pdf("print", &path) {
        Ok(()) => Ok(PrintPdfOutcome::Printed),
        Err(print_error) => {
            log_pdf_error(&format!(
                "print falhou para {}: {print_error}",
                path.display()
            ));
            shell_execute_pdf("open", &path)
                .map(|()| PrintPdfOutcome::Opened {
                    print_error: print_error.clone(),
                })
                .map_err(|open_error| {
                    log_pdf_error(&format!(
                        "open falhou para {}: {open_error}",
                        path.display()
                    ));
                    format!("print falhou: {print_error}; abrir PDF tambem falhou: {open_error}")
                })
        }
    }
}

fn show_share_ui(
    context: &str,
    path: &Path,
    title: &str,
    description: &str,
    text: &str,
) -> Result<(), String> {
    let path = canonical_pdf_path(path, "PDF para compartilhamento")?;
    let winrt_path = winrt_path_string(&path);
    log_pdf_debug(&format!(
        "{context}: iniciando Windows Share UI para {winrt_path}"
    ));

    let _winrt = unsafe { initialize_winrt()? };
    let hwnd = unsafe { create_share_owner_window()? };
    let _window = WindowGuard(hwnd);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }

    load_storage_file(&winrt_path)?;
    let interop = factory::<DataTransferManager, IDataTransferManagerInterop>()
        .map_err(|error| format!("falha ao carregar DataTransferManager interop: {error:?}"))?;
    let manager: DataTransferManager = unsafe {
        interop
            .GetForWindow(hwnd)
            .map_err(|error| format!("falha ao preparar Share UI: {error:?}"))?
    };

    let title = HSTRING::from(title);
    let description = HSTRING::from(description);
    let text = HSTRING::from(text);
    let data_requested = Arc::new(AtomicBool::new(false));
    let handler_data_requested = Arc::clone(&data_requested);
    let handler_path = winrt_path.clone();
    let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
        move |_sender, args| {
            handler_data_requested.store(true, Ordering::SeqCst);
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            let request = args.Request()?;
            let data = request.Data()?;
            let properties = data.Properties()?;
            properties.SetTitle(&title)?;
            properties.SetDescription(&description)?;
            data.SetRequestedOperation(DataPackageOperation::Copy)?;
            data.SetText(&text)?;
            let handler_file = match load_storage_file(&handler_path) {
                Ok(file) => file,
                Err(error) => {
                    request.FailWithDisplayText(&HSTRING::from(error))?;
                    return Ok(());
                }
            };
            let item: IStorageItem = handler_file.cast()?;
            let items = windows_collections::IIterable::<IStorageItem>::from(vec![Some(item)]);
            data.SetStorageItems(&items, true)?;
            Ok(())
        },
    );
    let token = manager
        .DataRequested(&handler)
        .map_err(|error| format!("falha ao registrar dados do compartilhamento: {error:?}"))?;

    unsafe {
        interop
            .ShowShareUIForWindow(hwnd)
            .map_err(|error| format!("ShowShareUIForWindow falhou: {error:?}"))?;
    }
    pump_messages_until(Duration::from_secs(3), || {
        data_requested.load(Ordering::SeqCst)
    });
    let _ = manager.RemoveDataRequested(token);

    if !data_requested.load(Ordering::SeqCst) {
        return Err(String::from(
            "Windows Share UI nao solicitou os dados do PDF; painel nativo nao abriu",
        ));
    }

    log_pdf_debug(&format!(
        "{context}: Windows Share UI acionado para {winrt_path}"
    ));
    Ok(())
}

fn load_storage_file(path: &str) -> Result<StorageFile, String> {
    let path = HSTRING::from(path);
    StorageFile::GetFileFromPathAsync(&path)
        .map_err(|error| format!("falha ao localizar PDF para compartilhamento: {error:?}"))?
        .join()
        .map_err(|error| format!("falha ao carregar PDF para compartilhamento: {error:?}"))
}

fn canonical_pdf_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("Nao foi possivel localizar {label}: {error}"))
}

fn select_pdf_in_explorer(path: &Path) -> Result<(), String> {
    Command::new("explorer")
        .arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map_err(|error| format!("Nao foi possivel abrir o Explorer com o PDF: {error}"))?;
    Ok(())
}

fn shell_execute_pdf(verb: &str, path: &Path) -> Result<(), String> {
    let verb = wide_null(verb);
    let file = wide_null(&path.to_string_lossy());
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        return Err(format!(
            "ShellExecuteW falhou com codigo {}",
            result.0 as isize
        ));
    }
    Ok(())
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn winrt_path_string(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{stripped}")
    } else if let Some(stripped) = text.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        text.to_string()
    }
}

unsafe fn initialize_winrt() -> Result<WinRtGuard, String> {
    match unsafe { RoInitialize(RO_INIT_SINGLETHREADED) } {
        Ok(()) => Ok(WinRtGuard { uninitialize: true }),
        Err(error) if error.code() == RPC_E_CHANGED_MODE => {
            log_pdf_debug("WinRT ja inicializado em outro modo; seguindo sem RoUninitialize");
            Ok(WinRtGuard {
                uninitialize: false,
            })
        }
        Err(error) => Err(format!("falha ao inicializar WinRT: {error:?}")),
    }
}

unsafe fn create_share_owner_window() -> Result<HWND, String> {
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("Razai Share"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            320,
            120,
            None,
            None,
            None,
            None,
        )
    }
    .map_err(|error| format!("falha ao criar janela helper de compartilhamento: {error:?}"))
}

fn pump_messages_until<F>(duration: Duration, mut done: F)
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    let mut message = MSG::default();
    while start.elapsed() < duration && !done() {
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() } {
            unsafe {
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

struct WindowGuard(HWND);

impl Drop for WindowGuard {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

struct WinRtGuard {
    uninitialize: bool,
}

impl Drop for WinRtGuard {
    fn drop(&mut self) {
        if self.uninitialize {
            unsafe {
                RoUninitialize();
            }
        }
    }
}

fn log_pdf_error(message: &str) {
    append_pdf_log("razai_pdf_error.log", message);
}

fn log_pdf_debug(message: &str) {
    append_pdf_log("razai_pdf_debug.log", message);
}

fn append_pdf_log(file_name: &str, message: &str) {
    let path: PathBuf = std::env::temp_dir().join(file_name);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winrt_path_strips_extended_prefix() {
        let path = Path::new(r"\\?\C:\Temp\pedido.pdf");

        assert_eq!(winrt_path_string(path), r"C:\Temp\pedido.pdf");
    }

    #[test]
    #[ignore]
    fn manual_share_pdf_with_windows_ui() {
        let path = std::env::var("RAZAI_SHARE_TEST_PDF")
            .expect("defina RAZAI_SHARE_TEST_PDF com o caminho de um PDF existente");
        share_pdf_with_windows_ui(
            "Teste manual",
            Path::new(&path),
            "Pedido Razai",
            "PDF gerado pelo Razai.",
            "Pedido Razai",
        )
        .unwrap();
    }
}

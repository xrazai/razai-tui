# PDF e Integracao Nativa do Windows

Este documento define o padrao para fluxos que geram PDF e acionam UI nativa do Windows no Razai TUI.

## Regra principal

Geracao de PDF e UI nativa sao etapas separadas:

1. O worker em segundo plano carrega dados, monta e grava o PDF.
2. O resultado do worker devolve o caminho do arquivo para o loop principal da TUI.
3. O drain do loop principal aciona a UI nativa: compartilhamento, impressao ou abertura.

Isso evita travar a TUI durante a geracao do arquivo e evita abrir janelas nativas a partir de threads auxiliares sem contexto de janela confiavel.

## Compartilhamento

Pedidos usam a Windows Share UI via WinRT:

- `DataTransferManager` com `IDataTransferManagerInterop::ShowShareUIForWindow`;
- uma janela Win32 helper como dona da Share UI;
- `DataRequested` registrado antes de chamar `ShowShareUIForWindow`;
- sucesso confirmado apenas quando o Windows dispara `DataRequested`.

`ShowShareUIForWindow` pode retornar sucesso mesmo quando o painel nao aparece. Por isso o app nunca deve dizer que o compartilhamento abriu apenas pelo retorno `Ok` dessa chamada.

Nao usar o verbo shell `share` (`ShellExecuteW("share")`) para PDF. Em desktop ele nao e confiavel para esse tipo de arquivo e pode falhar com associacao ausente.

Se a Share UI nao abrir, o fallback esperado e abrir o Explorer com o PDF selecionado. Nesse caso o status deve informar que o compartilhamento nativo ficou indisponivel, nao que foi aberto.

## Impressao e abertura

Checklists usam `ShellExecuteW("print")` para abrir a tela/acao de impressao do visualizador padrao. Se a impressao nao estiver disponivel, o app tenta `ShellExecuteW("open")` para abrir o PDF.

O status deve diferenciar:

- PDF gerado e enviado para impressao;
- impressao indisponivel e PDF aberto;
- PDF gerado, mas impressao e abertura falharam.

## Logs

Acoes Windows de PDF registram detalhes em `%TEMP%`:

- `razai_pdf_debug.log`
- `razai_pdf_error.log`

Esses logs ajudam a diagnosticar casos em que o Windows retorna sucesso, mas nao mostra UI, ou quando o visualizador padrao nao suporta impressao direta.

## Checklist para novos fluxos de PDF

- Salvar PDFs fora do workspace, em `Documents\Razai\<dominio>`, para nao reiniciar `cargo watch`.
- Proteger `printpdf` com `panic::catch_unwind`.
- Rodar geracao em `BackgroundTask<T>`.
- Retornar `pdf_path` no resultado do worker quando a gravacao terminar.
- Acionar Share UI, impressao ou abertura somente no drain da TUI.
- Registrar fallback e falhas em status visivel para o operador.

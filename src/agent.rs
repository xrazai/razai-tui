use crate::models::{DadosOption, DadosScreen, Section, TecidoForm, VendasScreen};
use serde::{Deserialize, Serialize};

pub struct SkillContext {
    pub name: &'static str,
    pub description: &'static str,
}

pub fn active_skill(
    section: Section,
    dados_screen: DadosScreen,
    dados_option: DadosOption,
    vendas_screen: VendasScreen,
) -> SkillContext {
    match (section, dados_screen, dados_option, vendas_screen) {
        (Section::Dados, DadosScreen::CadastrarTecido, _, _) => SkillContext {
            name: "dados.tecidos.cadastro",
            description: "Ajuda no cadastro de tecidos, validacao de campos, SKU e calculos de gramatura.",
        },
        (Section::Dados, DadosScreen::Tecidos, _, _) => SkillContext {
            name: "dados.tecidos.lista",
            description: "Ajuda a consultar tecidos cadastrados e iniciar novos cadastros.",
        },
        (Section::Dados, DadosScreen::Cores, _, _) => SkillContext {
            name: "dados.cores.lista",
            description: "Ajuda a consultar cores cadastradas e iniciar novos cadastros.",
        },
        (Section::Dados, DadosScreen::CadastrarCor, _, _) => SkillContext {
            name: "dados.cores.cadastro",
            description: "Ajuda no cadastro de cores, validacao de hexadecimal e nome.",
        },
        (Section::Dados, DadosScreen::Estampas, _, _) => SkillContext {
            name: "dados.estampas.lista",
            description: "Ajuda a consultar estampas cadastradas e iniciar novos cadastros.",
        },
        (Section::Dados, DadosScreen::CadastrarEstampa, _, _) => SkillContext {
            name: "dados.estampas.cadastro",
            description: "Ajuda no cadastro de estampas e geracao automatica de SKU.",
        },
        (Section::Dados, DadosScreen::VinculosMenu, _, _) => SkillContext {
            name: "dados.vinculos.menu",
            description: "Ajuda a escolher entre criar vinculos e consultar vinculos existentes.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarTecidoCriar, _, _) => SkillContext {
            name: "dados.vinculos.criar.tecido",
            description: "Ajuda a selecionar o tecido que recebera vinculos de cores ou estampas conforme o tipo.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarCores, _, _) => SkillContext {
            name: "dados.vinculos.criar.itens",
            description: "Ajuda a selecionar uma ou varias cores para tecido liso, ou estampas para tecido estampado.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarTecidoVer, _, _) => SkillContext {
            name: "dados.vinculos.ver.tecido",
            description: "Ajuda a selecionar um tecido para consultar cores vinculadas.",
        },
        (Section::Dados, DadosScreen::VinculosLista, _, _) => SkillContext {
            name: "dados.vinculos.lista",
            description: "Ajuda a consultar os vinculos existentes de tecido com cor ou estampa.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Tecido, _) => SkillContext {
            name: "dados.tecidos",
            description: "Ajuda com dados de tecidos.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Cores, _) => SkillContext {
            name: "dados.cores",
            description: "Ajuda com cadastro e consulta de cores.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Estampas, _) => SkillContext {
            name: "dados.estampas",
            description: "Ajuda com cadastro e consulta de estampas.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Vinculos, _) => SkillContext {
            name: "dados.vinculos",
            description: "Ajuda com vinculos entre tecidos e cores ou estampas.",
        },
        (Section::Dashboard, _, _, _) => SkillContext {
            name: "dashboard",
            description: "Ajuda a interpretar indicadores gerais da loja.",
        },
        (Section::Vendas, _, _, VendasScreen::Menu) => SkillContext {
            name: "vendas.menu",
            description: "Ajuda a iniciar uma nova venda ou consultar o historico de vendas.",
        },
        (Section::Vendas, _, _, VendasScreen::SelecionarTecido) => SkillContext {
            name: "vendas.nova.tecido",
            description: "Ajuda a escolher o tecido da venda; o app decide cor ou estampa pelo tipo do tecido.",
        },
        (Section::Vendas, _, _, VendasScreen::SelecionarVinculo) => SkillContext {
            name: "vendas.nova.vinculo",
            description: "Ajuda a escolher a cor vinculada para tecido liso ou a estampa vinculada para tecido estampado.",
        },
        (Section::Vendas, _, _, VendasScreen::Lancamento) => SkillContext {
            name: "vendas.nova.lancamento",
            description: "Ajuda a lancar preco unitario, quantidade vendida e conferir o resumo do pedido.",
        },
        (Section::Vendas, _, _, VendasScreen::Historico) => SkillContext {
            name: "vendas.historico",
            description: "Ajuda a consultar vendas anteriores quando o historico estiver implementado.",
        },
        (Section::Pedidos, _, _, _) => SkillContext {
            name: "pedidos",
            description: "Ajuda com acompanhamento de pedidos.",
        },
        (Section::Estoque, _, _, _) => SkillContext {
            name: "estoque",
            description: "Ajuda com consulta e movimentacao de estoque.",
        },
        (Section::Configuracoes, _, _, _) => SkillContext {
            name: "configuracoes.impressora_recibo",
            description: "Ajuda a configurar a impressora termica 80mm para recibos de venda com envio direto.",
        },
    }
}

pub fn screen_context(form: &TecidoForm) -> String {
    format!(
        "Formulario tecido: nome='{}', composicao='{}', largura='{}', rendimento='{}', gramatura_linear='{}', gramatura_m2='{}'.",
        form.nome,
        form.composicao,
        form.largura,
        form.rendimento,
        form.gramatura_linear,
        form.gramatura_m2
    )
}

pub fn local_reply(skill: &SkillContext, user_message: &str, form: &TecidoForm) -> String {
    let mut reply = format!("Skill ativa: {}. {}", skill.name, skill.description);

    if skill.name == "dados.tecidos.cadastro" {
        reply.push_str(" Campos obrigatorios: Nome, Composicao e Largura.");
        if !form.largura.trim().is_empty() {
            reply.push_str(" Se largura e rendimento/gramatura estiverem preenchidos, o sistema calcula os campos derivados.");
        }
        reply.push(' ');
        reply.push_str(&screen_context(form));
    }

    if !user_message.trim().is_empty() {
        reply.push_str(" OpenRouter ainda nao esta configurado neste app; configure OPENROUTER_API_KEY para respostas reais.");
    }

    reply
}

pub async fn openrouter_reply(
    skill: &SkillContext,
    user_message: &str,
    form: &TecidoForm,
) -> Result<String, String> {
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| String::from("OPENROUTER_API_KEY nao configurada"))?;
    let model = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| String::from("anthropic/claude-sonnet-4.5"));
    let system_prompt = format!(
        "Voce e um agente especialista da TUI Razai. Responda em portugues, curto e pratico.\nSkill ativa: {}\nDescricao: {}\nContexto da tela: {}\nNao execute acoes no banco sem confirmacao explicita.",
        skill.name,
        skill.description,
        screen_context(form)
    );
    let request = ChatCompletionRequest {
        model,
        temperature: 0.2,
        max_tokens: 500,
        messages: vec![
            OpenRouterMessage {
                role: String::from("system"),
                content: system_prompt,
            },
            OpenRouterMessage {
                role: String::from("user"),
                content: user_message.to_string(),
            },
        ],
    };

    let response = reqwest::Client::new()
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "http://localhost/razai-tui")
        .header("X-Title", "Razai TUI")
        .json(&request)
        .send()
        .await
        .map_err(|error| format!("Falha ao chamar OpenRouter: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenRouter retornou {status}: {body}"));
    }

    let completion = response
        .json::<ChatCompletionResponse>()
        .await
        .map_err(|error| format!("Resposta invalida da OpenRouter: {error}"))?;

    completion
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| String::from("OpenRouter retornou resposta vazia"))
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    temperature: f32,
    max_tokens: u16,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

use crate::models::{DadosOption, DadosScreen, Section, TecidoForm};
use serde::{Deserialize, Serialize};

pub struct SkillContext {
    pub name: &'static str,
    pub description: &'static str,
}

pub fn active_skill(
    section: Section,
    dados_screen: DadosScreen,
    dados_option: DadosOption,
) -> SkillContext {
    match (section, dados_screen, dados_option) {
        (Section::Dados, DadosScreen::CadastrarTecido, _) => SkillContext {
            name: "dados.tecidos.cadastro",
            description: "Ajuda no cadastro de tecidos, validacao de campos, SKU e calculos de gramatura.",
        },
        (Section::Dados, DadosScreen::Tecidos, _) => SkillContext {
            name: "dados.tecidos.lista",
            description: "Ajuda a consultar tecidos cadastrados e iniciar novos cadastros.",
        },
        (Section::Dados, DadosScreen::Cores, _) => SkillContext {
            name: "dados.cores.lista",
            description: "Ajuda a consultar cores cadastradas e iniciar novos cadastros.",
        },
        (Section::Dados, DadosScreen::CadastrarCor, _) => SkillContext {
            name: "dados.cores.cadastro",
            description: "Ajuda no cadastro de cores, validacao de hexadecimal e nome.",
        },
        (Section::Dados, DadosScreen::VinculosMenu, _) => SkillContext {
            name: "dados.vinculos.menu",
            description: "Ajuda a escolher entre criar vinculos e consultar vinculos existentes.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarTecidoCriar, _) => SkillContext {
            name: "dados.vinculos.criar.tecido",
            description: "Ajuda a selecionar o tecido que recebera vinculos de cores.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarCores, _) => SkillContext {
            name: "dados.vinculos.criar.cores",
            description: "Ajuda a selecionar uma ou varias cores para vincular ao tecido.",
        },
        (Section::Dados, DadosScreen::VinculosSelecionarTecidoVer, _) => SkillContext {
            name: "dados.vinculos.ver.tecido",
            description: "Ajuda a selecionar um tecido para consultar cores vinculadas.",
        },
        (Section::Dados, DadosScreen::VinculosLista, _) => SkillContext {
            name: "dados.vinculos.lista",
            description: "Ajuda a consultar os vinculos existentes de tecido e cor.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Tecido) => SkillContext {
            name: "dados.tecidos",
            description: "Ajuda com dados de tecidos.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Cores) => SkillContext {
            name: "dados.cores",
            description: "Ajuda com cadastro e consulta de cores.",
        },
        (Section::Dados, DadosScreen::Menu, DadosOption::Vinculos) => SkillContext {
            name: "dados.vinculos",
            description: "Ajuda com vinculos entre tecidos e cores.",
        },
        (Section::Dashboard, _, _) => SkillContext {
            name: "dashboard",
            description: "Ajuda a interpretar indicadores gerais da loja.",
        },
        (Section::Vendas, _, _) => SkillContext {
            name: "vendas",
            description: "Ajuda com lancamentos e analise de vendas.",
        },
        (Section::Pedidos, _, _) => SkillContext {
            name: "pedidos",
            description: "Ajuda com acompanhamento de pedidos.",
        },
        (Section::Estoque, _, _) => SkillContext {
            name: "estoque",
            description: "Ajuda com consulta e movimentacao de estoque.",
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

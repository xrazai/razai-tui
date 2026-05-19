#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum Focus {
    #[default]
    System,
    Chat,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum PedidosScreen {
    #[default]
    Menu,
    SelecionarTecido,
    SelecionarVinculo,
    Lancamento,
    Historico,
}

impl Focus {
    pub fn next(self) -> Self {
        match self {
            Focus::System => Focus::Chat,
            Focus::Chat => Focus::System,
        }
    }

    pub fn previous(self) -> Self {
        self.next()
    }

    pub fn title(self) -> &'static str {
        match self {
            Focus::System => "Sistema",
            Focus::Chat => "Chat",
        }
    }
}

#[derive(Default)]
pub struct ChatState {
    pub input: String,
    pub messages: Vec<ChatMessage>,
    pub waiting: bool,
}

pub struct ChatMessage {
    pub author: &'static str,
    pub text: String,
}

#[derive(Clone)]
pub enum AgentAction {
    CreateTecido {
        nome: String,
        composicao: String,
        largura: String,
        tipo: Option<String>,
        rendimento: Option<String>,
        gramatura_linear: Option<String>,
        gramatura_m2: Option<String>,
    },
    CreateCor {
        nome: String,
        hex: String,
    },
    CreateEstampa {
        nome: String,
    },
    CreateVinculo {
        tecido: String,
        item: String,
    },
    OpenVenda {
        id: i64,
    },
    FilterSalesHistory {
        inicio: String,
        fim: String,
    },
    SelectPrinter {
        printer: String,
    },
    AddVendaItem {
        tecido: String,
        item: String,
        preco: String,
        quantidade: String,
    },
    AddPedidoItem {
        tecido: String,
        item: String,
        preco: String,
        quantidade: String,
    },
}

#[derive(Clone)]
pub enum AgentDraft {
    CreateTecido {
        nome: Option<String>,
        composicao: Option<String>,
        largura: Option<String>,
        tipo: Option<String>,
        rendimento: Option<String>,
        gramatura_linear: Option<String>,
        gramatura_m2: Option<String>,
    },
    CreateCor {
        nome: Option<String>,
        hex: Option<String>,
    },
    CreateEstampa {
        nome: Option<String>,
    },
    CreateVinculo {
        tecido: Option<String>,
        item: Option<String>,
    },
    SelectPrinter {
        printer: Option<String>,
    },
    AddVendaItem {
        tecido: Option<String>,
        item: Option<String>,
        preco: Option<String>,
        quantidade: Option<String>,
    },
    AddPedidoItem {
        tecido: Option<String>,
        item: Option<String>,
        preco: Option<String>,
        quantidade: Option<String>,
    },
}

impl ChatMessage {
    pub fn user(text: String) -> Self {
        Self {
            author: "Voce",
            text,
        }
    }

    pub fn assistant(text: String) -> Self {
        Self { author: "IA", text }
    }
}

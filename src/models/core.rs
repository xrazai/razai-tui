#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum Focus {
    #[default]
    System,
    Chat,
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

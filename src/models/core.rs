#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub enum Focus {
    #[default]
    System,
    Chat,
}

impl Focus {
    pub fn toggle(self) -> Self {
        match self {
            Focus::System => Focus::Chat,
            Focus::Chat => Focus::System,
        }
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

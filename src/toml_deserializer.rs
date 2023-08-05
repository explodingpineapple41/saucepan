use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub bindings: Bindings,
    pub colors: Colors,
}

#[derive(Deserialize, Clone)]
pub struct Bindings {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub insert: String,
    pub normal: String,
    pub command: String,
}

#[derive(Deserialize, Clone)]
pub struct Colors {
    pub editor: EditorColors,
}

#[derive(Deserialize, Clone)]
pub struct EditorColors {
    pub window: WindowColors,
    pub text: TextColors,
}

#[derive(Deserialize, Clone)]
pub struct WindowColors {
    pub background: String,
    pub cursor: String,
    pub highlight: String,
}

#[derive(Deserialize, Clone)]
pub struct TextColors {
    pub unselected: String,
    pub selected: String,
}

pub fn return_config() -> Config {
    toml::from_str(include_str!("../assets/config.toml")).unwrap()
}

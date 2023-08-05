use druid::{Widget, WindowDesc, AppLauncher};
use saucepan::{toml_deserializer::*, EditorData};
use std::{env, fs};

fn build_ui() -> impl Widget<saucepan::EditorData> {
    saucepan::Editor::new(include_bytes!("../assets/inconsolata.ttf"), return_config())
}

fn main() {
    let mut args = env::args();

    let main_window = WindowDesc::new(build_ui())
        .window_size((1280., 720.))
        .title("Saucepan");
    let initial_data = if let Some(x) = args.nth(1) {
        let file = match fs::read_to_string(&x) {
            Ok(f) => f,
            Err(_) => panic!("Failed to launch Saucepan from path: {x}")
        };

        EditorData::from_file(&file)
    } else {
        EditorData::new()
    };

    AppLauncher::with_window(main_window)
        .launch(initial_data)
        .expect("Failed to launch Saucepan");
}


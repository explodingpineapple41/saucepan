pub mod toml_deserializer;

use toml_deserializer::*;
use std::{cmp::{min, max}, collections::HashMap};
use druid::{
    widget::{TextBox, Split, Container},
    piet::{Text, TextLayoutBuilder, TextLayout, CairoTextLayout},
    AppLauncher, Widget, WindowDesc, Data, Lens, EventCtx, Event, Env, LifeCycle, LifeCycleCtx, UpdateCtx, LayoutCtx, BoxConstraints, Size, PaintCtx, RenderContext, KeyEvent, Color, Point, FontFamily, Code, Modifiers, Rect, FontDescriptor, 
};

#[derive(Clone)]
enum EditorCommand {
    Insert(String),
    Backspace,
    Delete,
    Vmove(isize),
    Hmove(isize),
    Mode(EditorMode),
    Visual(VisualMode),
    Command,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum EditorMode {
    Normal,
    Insert,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum VisualMode {
    PerMove,
    AllMove,
    Line,
    Block,
}

type Action = (EditorMode, String);

type CommandMap = HashMap<Action, Box<dyn FnOnce(&mut EditorData)>>;

#[derive(Clone)]
pub struct EditorData {
    file_name: Option<String>,
    buffer: Vec<String>,
    command_buffer: String,
    command_cursor: usize,
    command_mode: bool,
    mode: EditorMode,
    visual: VisualMode,
    editor_size: [usize; 2],
    window_pos: [usize; 2],
    cursor_pos: [usize; 2],
    selection_start: [usize; 2],
}

impl EditorData {
    pub fn new() -> Self {
        Self { 
            file_name: None,
            buffer: vec![String::from("")], 
            command_buffer: "".to_string(),
            command_cursor: 0,
            command_mode: false,
            mode: EditorMode::Normal, 
            visual: VisualMode::PerMove,
            editor_size: [10, 10], 
            window_pos: [0, 0], 
            cursor_pos: [0, 0], 
            selection_start: [0, 0] 
        }
    }

    pub fn from_file(file: &str) -> Self {
        let split_file = file.split("\n").map(|l| l.to_owned()).collect();

        Self {
            file_name: Some(file.to_string()),
            buffer: split_file,
            command_buffer: "".to_string(),
            command_cursor: 0,
            command_mode: false,
            mode: EditorMode::Normal,
            visual: VisualMode::PerMove,
            editor_size: [10, 10],
            window_pos: [0, 0],
            cursor_pos: [0, 0],
            selection_start: [0, 0]
        }
    }

    #[inline]
    fn window_outer_bound(&self) -> [usize; 2] {
        [self.window_pos[0] + self.editor_size[0], self.window_pos[1] + self.editor_size[1]]
    }

    fn format_buffer(&self, config: &Colors, ctx: &mut PaintCtx, font: &[u8]) -> (Vec<(CairoTextLayout, Point)>, [Point; 2], Vec<[Point; 2]>) {
        let displayed_buffer = &self.buffer;
        let text = ctx.text();
        let font = text.load_font(font).unwrap_or(FontFamily::MONOSPACE);
        let mut cursor_bound = [Point::ZERO, Point::ZERO];
        let mut selection_pos = vec![];
        let mut layout = vec![];
        let (first_cursor, last_cursor) = if self.cursor_pos[0] < self.selection_start[0] || 
            (self.cursor_pos[0] == self.selection_start[0] && self.cursor_pos[1] < self.selection_start[1]) {
            (self.cursor_pos, self.selection_start)
        } else {
            (self.selection_start, self.cursor_pos)
        };

        for (i, line) in displayed_buffer.into_iter().enumerate() {
            if i != self.cursor_pos[0] {
                let line_layout = text.new_text_layout(format!("{} ", line))
                    .text_color(Color::from_hex_str(&config.editor.text.unselected).unwrap())
                    .font(font.clone(), 24.)
                    .build()
                    .unwrap();
                let line_metric = line_layout.line_metric(0).unwrap();

                if first_cursor[0] == i {
                    let mut selection_start = line_layout.hit_test_text_position(first_cursor[1]).point;
                    let mut selection_end = line_layout.hit_test_text_position(line.len()).point;
                    selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                    selection_end.y = selection_start.y + line_metric.height;
                    selection_pos.push([selection_start, selection_end]);
                } else if last_cursor[0] == i {
                    let mut selection_start = line_layout.hit_test_text_position(0).point;
                    let mut selection_end = line_layout.hit_test_text_position(last_cursor[1]).point;
                    selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                    selection_end.y = selection_start.y + line_metric.height;
                    selection_pos.push([selection_start, selection_end]);
                } else if i > first_cursor[0] && i < last_cursor[0] {
                    let mut selection_start = line_layout.hit_test_text_position(0).point;
                    let mut selection_end = line_layout.hit_test_text_position(line.len()).point;
                    selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                    selection_end.y = selection_start.y + line_metric.height;
                    selection_pos.push([selection_start, selection_end]);
                }

                layout.push((line_layout, Point::new(0., i as f64 * line_metric.height)));
            } else {
                if line.len() > 0 {
                    let line_layout = text.new_text_layout(format!("{}", &line))
                        .text_color(Color::from_hex_str(&config.editor.text.selected).unwrap())
                        .font(font.clone(), 24.)
                        .build()
                        .unwrap();
                    let line_metric = line_layout.line_metric(0).unwrap();

                    let mut current_pos = line_layout.hit_test_text_position(self.cursor_pos[1]).point;
                    current_pos.y += i as f64 * line_metric.height - line_metric.baseline;

                    if first_cursor[0] == last_cursor[0] {
                        let mut selection_st = line_layout.hit_test_text_position(first_cursor[1]).point;
                        let mut selection_end = line_layout.hit_test_text_position(last_cursor[1]).point;
                        selection_st.y += i as f64 * line_metric.height - line_metric.baseline;
                        selection_end.y = selection_st.y + line_metric.height;
                        selection_pos.push([selection_st, selection_end]);
                    } else if first_cursor[0] == i {
                        let mut selection_start = line_layout.hit_test_text_position(first_cursor[1]).point;
                        let mut selection_end = line_layout.hit_test_text_position(line.len()).point;
                        selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                        selection_end.y = selection_start.y + line_metric.height;
                        selection_pos.push([selection_start, selection_end]);
                    } else if last_cursor[0] == i {
                        let mut selection_start = line_layout.hit_test_text_position(0).point;
                        let mut selection_end = line_layout.hit_test_text_position(last_cursor[1]).point;
                        selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                        selection_end.y = selection_start.y + line_metric.height;
                        selection_pos.push([selection_start, selection_end]);
                    }
                                        
                    cursor_bound = [current_pos, Point::new(if self.cursor_pos[1] == line.len() {
                        2. * current_pos.x - line_layout.hit_test_text_position(self.cursor_pos[1] - 1).point.x
                    } else {
                        line_layout.hit_test_text_position(self.cursor_pos[1] + 1).point.x
                    }, current_pos.y + line_metric.height)];
      
                    layout.push((line_layout, Point::new(0., i as f64 * line_metric.height)));
                    
                }
            }
        }
        (layout, cursor_bound, selection_pos)
    }

    fn handle_keybuffer(&mut self, key_pressed: String, command_map: &CommandMap) {
        todo!();
    }

    fn exec_command(&mut self) {
        println!("Executed command: {}", self.command_buffer);

        self.command_buffer = "".to_string();
        self.command_mode = false;
    }

    fn insert(&mut self, str: &str) {
        if self.command_mode {
            if str == "\n" {
                self.exec_command();
                return;
            }
            self.command_buffer.push_str(str);
            self.command_cursor += str.len();
        } else {
            self.selection_start = self.cursor_pos;
            if let None = str.find('\n') {
                self.buffer[self.cursor_pos[0]].insert_str(self.cursor_pos[1], str); 
                self.cursor_pos[1] += str.len();
            } else {
                let formatted_str = str.split('\n').collect::<Vec<&str>>();
                let truncated = self.buffer[self.cursor_pos[0]].split_off(self.cursor_pos[1]);
                self.insert(formatted_str[0]);
                self.buffer.insert(self.cursor_pos[0] + 1, truncated);

                if formatted_str.len() > 1 {
                    for (i, line) in formatted_str[1..formatted_str.len() - 1].iter().enumerate() {
                        self.buffer.insert(self.cursor_pos[0] + i + 2, line.to_string());
                    }
                }

                self.cursor_pos = [self.cursor_pos[0] + formatted_str.len() - 1, 0];

                if self.buffer.len() == self.cursor_pos[0] {
                    self.buffer.push(String::from(""));
                }
            }
        }
    }

    fn delete_selection(&mut self) {
        let (first_cursor, last_cursor) = if self.cursor_pos[0] < self.selection_start[0] || 
            (self.cursor_pos[0] == self.selection_start[0] && self.cursor_pos[1] < self.selection_start[1]) {
            (self.cursor_pos, self.selection_start)
        } else {
            (self.selection_start, self.cursor_pos)
        };

        if first_cursor[0] != last_cursor[0] {
             self.buffer[first_cursor[0]].truncate(first_cursor[1]);
             self.buffer[last_cursor[0]].replace_range(0..last_cursor[1], "");

             for i in first_cursor[0] + 1..last_cursor[0] - 1 {
                 self.buffer.remove(i);
             }

             let last_line = self.buffer.remove(last_cursor[0]);
             self.buffer[first_cursor[0]].push_str(&last_line);
        } else {
            self.buffer[first_cursor[0]].replace_range(first_cursor[1]..last_cursor[1], "");
        }

        self.cursor_pos = first_cursor;
        self.selection_start = first_cursor;
    }

    fn backspace(&mut self) {
        if self.command_mode {
            if self.command_cursor > 0 {
                self.command_buffer.pop();
                self.command_cursor -= 1;
            }
        } else {
            if self.cursor_pos[1] != 0 {
                self.buffer[self.cursor_pos[0]].remove(self.cursor_pos[1] - 1);
                self.cursor_pos[1] -= 1;

                if self.cursor_pos[1] < self.selection_start[1] && self.cursor_pos[0] == self.selection_start[0] {
                    self.selection_start[1] -= 1;
                }
            } else if self.cursor_pos[0] != 0 {
                let prev_line_len = self.buffer[self.cursor_pos[0] - 1].len();
                let line_to_append = self.buffer.remove(self.cursor_pos[0]);
                self.cursor_pos = [self.cursor_pos[0] - 1, self.buffer[self.cursor_pos[0] - 1].len()];

                if self.cursor_pos[0] == self.selection_start[0] - 1 {
                    self.selection_start = [self.selection_start[0] - 1, self.selection_start[1] + prev_line_len];
                } else if self.cursor_pos[0] < self.selection_start[1] {
                    self.selection_start[0] -= 1;
                }

                self.buffer[self.cursor_pos[0]].push_str(line_to_append.as_str());
            }
        }
    }

    fn vmove_cursor(&mut self, x: isize) {
        if self.command_mode {
            
        } else {
            if x > 0 {
                self.cursor_pos[0] = min(self.cursor_pos[0] + x as usize, self.buffer.len() - 1);
            } else {
                self.cursor_pos[0] = max(self.cursor_pos[0] as isize + x, 0) as usize;
            }

            if self.cursor_pos[1] > self.buffer[self.cursor_pos[0]].len() {
                self.cursor_pos[1] = self.buffer[self.cursor_pos[0]].len();
            }

            if self.visual == VisualMode::PerMove {
                self.selection_start = self.cursor_pos;
            }
        }
    }
    
    fn hmove_cursor(&mut self, x: isize) {
        if self.command_mode {
            self.command_cursor = if x > 0 {
                min(self.command_cursor + x as usize, self.command_buffer.len())
            } else {
                max(self.command_cursor as isize + x, 0) as usize
            }
        } else {
            self.cursor_pos[1] = if x > 0 {
                min(self.cursor_pos[1] + x as usize, self.buffer[self.cursor_pos[0]].len())
            } else {
                max(self.cursor_pos[1] as isize + x, 0) as usize
            };

            if self.visual == VisualMode::PerMove {
                self.selection_start = self.cursor_pos;
            }
        }
        }
}

impl Data for EditorData {
    fn same(&self, other: &Self) -> bool {
        for (i, line) in self.buffer.iter().enumerate() {
            if *line == other.buffer[i] {
                continue;
            } else {
                return false;
            }
        }
        true
    }
}

pub struct Editor {
    font: &'static [u8],
    theme: Colors,
    command_map: CommandMap,
}

impl Editor {
    pub fn new(font: &'static [u8], config: Config) -> Self {
        Self {
            font,
            theme: config.colors,
            command_map: Self::create_command_map(config.bindings),
        }
    }

    fn create_command_map(config: Bindings) -> CommandMap {
        let mut command_map = HashMap::from([
            ((EditorMode::Normal, config.up), Box::new(|data: &mut EditorData| data.vmove_cursor(-1))),
            ((EditorMode::Normal, config.down), Box::new(|data: &mut EditorData| data.vmove_cursor(1))),
            ((EditorMode::Normal, config.left), Box::new(|data: &mut EditorData| data.hmove_cursor(-1))), 
            ((EditorMode::Normal, config.right), Box::new(|data: &mut EditorData| data.hmove_cursor(1))), 
            ((EditorMode::Normal, config.insert), Box::new(|data: &mut EditorData| data.mode = EditorMode::Insert)), 
            ((EditorMode::Normal, config.command), Box::new(|data: &mut EditorData| data = EditorData {mode: EditorMode::Insert, command_mode: true, ..data,})), 
            ((EditorMode::Normal, "{UARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Normal, "{DARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Normal, "{LARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Normal, "{RARR}".to_string()), Box::new(|data: &mut EditorData| data)), 

            ((EditorMode::Insert, config.normal), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{BACK}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{ENTER}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{TAB}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{UARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{DARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{LARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
            ((EditorMode::Insert, "{RARR}".to_string()), Box::new(|data: &mut EditorData| data)), 
        ]);
        
        for i in 32..=126 {
            command_map.insert((EditorMode::Insert,
            if i == 94 || i == 123 || i == 125 || i == 126 {
                format!("\\{}", char::from_u32(i).unwrap())
            } else {
                char::from_u32(i).unwrap().to_string()
            }), Box::new(|data: &mut EditorData| data)), 
        }

        command_map
    }
}

impl Widget<EditorData> for Editor {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut EditorData, env: &Env) {
        //println!("Selection: {:#?}, Cursor: {:#?}", data.selection_start, data.cursor_pos);
        match event {
            Event::WindowConnected => {
                ctx.request_focus();
            },
            Event::KeyDown(key) => data.handle_keybuffer(keyevent_to_key(key), &self.command_map),
            _ => (),
        }
        ctx.request_paint();
    } 
    
    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &EditorData, env: &Env) {
        
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &EditorData, data: &EditorData, env: &Env) {

    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &EditorData, env: &Env) -> Size {
        Size::new(500., 500.)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorData, env: &Env) {
        let bounds = ctx.size().to_rect();
        let rounded = bounds.to_rounded_rect(20.);
        let text = data.format_buffer(&self.theme, ctx, self.font);
        let cursor_rect = Rect::new(text.1[0].x, text.1[0].y, text.1[1].x, text.1[1].y);

        ctx.fill(rounded, &Color::from_hex_str(&self.theme.editor.window.background).unwrap());

        for rect in text.2 {
            let rect = Rect::new(rect[0].x, rect[0].y, rect[1].x, rect[1].y);
            ctx.fill(rect, &Color::from_hex_str(&self.theme.editor.window.highlight).unwrap())
        }

        ctx.fill(cursor_rect, &Color::from_hex_str(&self.theme.editor.window.cursor).unwrap());
        ctx.stroke(rounded, &env.get(druid::theme::PRIMARY_DARK), 5.);

        for (line, point) in text.0 {
            ctx.draw_text(&line, point);
        }
    }
}

fn keyevent_to_key(key: &KeyEvent) -> String {
    let mut key_notation = "".to_string();

    if key.mods.ctrl() {
        key_notation.push('^');
    }

    if key.mods.alt() {
        key_notation.push('~')
    }

    key_notation.push_str(key_to_char(
            &key.code, 
            key.mods.shift(), 
            key.mods.contains(Modifiers::CAPS_LOCK), 
            key.mods.contains(Modifiers::NUM_LOCK), 
            key.mods.contains(Modifiers::FN) ^ key.mods.contains(Modifiers::FN_LOCK)
            ));

    key_notation
    
}

fn key_to_char(key: &Code, shift: bool, caps: bool, numpad: bool, r#fn: bool) -> &str {
    match key {
        Code::KeyA => if shift ^ caps {"A"} else {"a"},
        Code::KeyB => if shift ^ caps {"B"} else {"b"},
        Code::KeyC => if shift ^ caps {"C"} else {"c"},
        Code::KeyD => if shift ^ caps {"D"} else {"d"},
        Code::KeyE => if shift ^ caps {"E"} else {"e"},
        Code::KeyF => if shift ^ caps {"F"} else {"f"},
        Code::KeyG => if shift ^ caps {"G"} else {"g"},
        Code::KeyH => if shift ^ caps {"H"} else {"h"},
        Code::KeyI => if shift ^ caps {"I"} else {"i"},
        Code::KeyJ => if shift ^ caps {"J"} else {"j"},
        Code::KeyK => if shift ^ caps {"K"} else {"k"},
        Code::KeyL => if shift ^ caps {"L"} else {"l"},
        Code::KeyM => if shift ^ caps {"M"} else {"m"},
        Code::KeyN => if shift ^ caps {"N"} else {"n"},
        Code::KeyO => if shift ^ caps {"O"} else {"o"},
        Code::KeyP => if shift ^ caps {"P"} else {"p"},
        Code::KeyQ => if shift ^ caps {"Q"} else {"q"},
        Code::KeyR => if shift ^ caps {"R"} else {"r"},
        Code::KeyS => if shift ^ caps {"S"} else {"s"},
        Code::KeyT => if shift ^ caps {"T"} else {"t"},
        Code::KeyU => if shift ^ caps {"U"} else {"u"},
        Code::KeyV => if shift ^ caps {"V"} else {"v"},
        Code::KeyW => if shift ^ caps {"W"} else {"w"},
        Code::KeyX => if shift ^ caps {"X"} else {"x"},
        Code::KeyY => if shift ^ caps {"Y"} else {"y"},
        Code::KeyZ => if shift ^ caps {"Z"} else {"z"},
        Code::Space => " ",
        Code::Escape => "{ESC}",
        Code::Enter => "{ENTER}",
        Code::Tab => "{TAB}",
        Code::ArrowUp => "{UARR}",
        Code::ArrowDown => "{DARR}",
        Code::ArrowLeft => "{LARR}",
        Code::ArrowRight => "{RARR}",
        Code::PageUp => "{PGUP}",
        Code::PageDown => "{PGDO}",
        Code::Backspace => "{BACK}",
        Code::Delete => "{DEL}",
        Code::Home => "{HOME}",
        Code::End => "{END}",
        Code::Insert => "{INS}",
        Code::Digit0 => if shift {")"} else {"0"},
        Code::Digit1 => if shift {"!"} else {"1"},
        Code::Digit2 => if shift {"@"} else {"2"},
        Code::Digit3 => if shift {"#"} else {"3"},
        Code::Digit4 => if shift {"$"} else {"4"},
        Code::Digit5 => if shift {"%"} else {"5"},
        Code::Digit6 => if shift {"\\^"} else {"6"},
        Code::Digit7 => if shift {"&"} else {"7"},
        Code::Digit8 => if shift {"*"} else {"8"},
        Code::Digit9 => if shift {"("} else {"9"},
        Code::F1 => "{F1}",
        Code::F2 => "{F2}",
        Code::F3 => "{F3}",
        Code::F4 => "{F4}",
        Code::F5 => "{F5}",
        Code::F6 => "{F6}",
        Code::F7 => "{F7}",
        Code::F8 => "{F8}",
        Code::F9 => "{F9}",
        Code::F10 => "{F10}",
        Code::F11 => "{F11}",
        Code::F12 => "{F12}",
        Code::F13 => "{F13}",
        Code::F14 => "{F14}",
        Code::F15 => "{F15}",
        Code::F16 => "{F16}",
        Code::F17 => "{F17}",
        Code::F18 => "{F18}",
        Code::F19 => "{F19}",
        Code::F20 => "{F20}",
        Code::F21 => "{F21}",
        Code::F22 => "{F22}",
        Code::F23 => "{F23}",
        Code::F24 => "{F24}",
        Code::Backquote => if shift {"\\~"} else {"`"},
        Code::Equal => if shift {"+"} else {"="},
        Code::Minus => if shift {"_"} else {"-"},
        Code::BracketLeft => if shift {"\\{"} else {"["},
        Code::BracketRight => if shift {"\\}"} else {"]"},
        Code::Backslash => if shift {"|"} else {"\\"},
        Code::Semicolon => if shift {":"} else {";"},
        Code::Quote => if shift {"\""} else {"'"},
        Code::Comma => if shift {"<"} else {","},
        Code::Period => if shift {">"} else {"."},
        Code::Slash => if shift {"?"} else {"/"},
        Code::Numpad0 => if numpad || r#fn {"0"} else {"{INS}"},
        Code::Numpad1 => if numpad || r#fn {"1"} else {"{END}"},
        Code::Numpad2 => if numpad || r#fn {"2"} else {"{DARR}"},
        Code::Numpad3 => if numpad || r#fn {"3"} else {"{PGDN}"},
        Code::Numpad4 => if numpad || r#fn {"4"} else {"{LARR}"},
        Code::Numpad5 => if numpad || r#fn {"5"} else {""},
        Code::Numpad6 => if numpad || r#fn {"6"} else {"{RARR}"},
        Code::Numpad7 => if numpad || r#fn {"7"} else {"{HOME}"},
        Code::Numpad8 => if numpad || r#fn {"8"} else {"{UARR}"},
        Code::Numpad9 => if numpad || r#fn {"9"} else {"{PGUP}"},
        Code::NumpadAdd => "+",
        Code::NumpadSubtract => "-",
        Code::NumpadMultiply => "*",
        Code::NumpadDivide => "/",
        Code::NumpadDecimal => ".",
        Code::NumpadComma => ",",
        Code::NumpadParenLeft => "(",
        Code::NumpadParenRight => ")",
        _ => ""
    }
}

use std::cmp::{min, max};

use druid::{
    AppLauncher, Widget, WindowDesc, Data, Lens, EventCtx, Event, Env, LifeCycle, LifeCycleCtx, UpdateCtx, LayoutCtx, BoxConstraints, Size, PaintCtx, RenderContext, KeyEvent,
    widget::{TextBox, Split, Container}, Color, piet::{Text, TextLayoutBuilder, D2DTextLayout, TextLayout}, Point, theme, FontFamily, Code, Modifiers, Rect
};

#[derive(Clone)]
enum EditorMode {
    Normal,
    Insert,
    Visual
}

#[derive(Clone)]
struct EditorData {
    buffer: Vec<String>,
    mode: EditorMode,
    editor_size: [usize; 2],
    window_pos: [usize; 2],
    cursor_pos: [usize; 2],
    selection_start: [usize; 2],
}

impl EditorData {
    #[inline]
    fn window_outer_bound(&self) -> [usize; 2]{
        [self.window_pos[0] + self.editor_size[0], self.window_pos[1] + self.editor_size[1]]
    }

    fn format_buffer(&self, ctx: &mut PaintCtx, font: &[u8]) -> (Vec<(D2DTextLayout, Point)>, [Point; 2], Vec<[Point; 2]>) {
        let displayed_buffer = &self.buffer;
        let text = ctx.text();
        let font = text.load_font(font).unwrap_or_default();
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
                    .text_color(Color::BLUE)
                    .font(font.clone(), 24.)
                    .build()
                    .unwrap();
                let line_metric = line_layout.line_metric(0).unwrap();

                if first_cursor[0] == i {
                    let mut selection_start = line_layout.hit_test_text_position(first_cursor[1]).point;
                    let mut selection_end = line_layout.hit_test_text_position(line.len()).point;
                    selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                    selection_end.y += selection_start.y + line_metric.height;
                    selection_pos.push([selection_start, selection_end]);
                } else if last_cursor[0] == i {
                    let mut selection_start = line_layout.hit_test_text_position(0).point;
                    let mut selection_end = line_layout.hit_test_text_position(line.len()).point;
                    selection_start.y += i as f64 * line_metric.height - line_metric.baseline;
                    selection_end.y += selection_start.y + line_metric.height;
                    selection_pos.push([selection_start, selection_end]);
                }

                layout.push((line_layout, Point::new(0., i as f64 * line_metric.height)));
            } else {
                if line.len() > 0 {
                    let line_layout = text.new_text_layout(format!("{}", &line))
                        .text_color(Color::GREEN)
                        .font(font.clone(), 24.)
                        .build()
                        .unwrap();
                    let line_metric = line_layout.line_metric(0).unwrap();

                    let mut current_pos = line_layout.hit_test_text_position(self.cursor_pos[1]).point;
                    current_pos.y += i as f64 * line_metric.height - line_metric.baseline;

                    if first_cursor[0] == last_cursor[0] {
                        let mut selection_st = line_layout.hit_test_text_position(first_cursor[1]).point;
                        let mut selection_end = line_layout.hit_test_text_position(last_cursor[1]).point;
                        println!("HIGHLITED: {:#?}, {:#?}", first_cursor, last_cursor);
                        selection_st.y += i as f64 * line_metric.height - line_metric.baseline;
                        selection_end.y += selection_st.y + line_metric.height;
                        selection_pos.push([selection_st, selection_end]);
                    }
                                        
                    cursor_bound = [current_pos, Point::new(current_pos.x + line_metric.height / 2., current_pos.y + line_metric.height)];
      
                    layout.push((line_layout, Point::new(0., i as f64 * line_metric.height)));
                    
                }
            }
        }
        (layout, cursor_bound, selection_pos)
    }

    fn insert(&mut self, str: &str) {
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

    fn delete_selection(&mut self) {
        if self.selection_start[0] != self.cursor_pos[0] {
             self.buffer[self.selection_start[0]].truncate(self.selection_start[1]);
             for i in (self.selection_start[0] + 1)..(self.cursor_pos[0] - 1) {
                 self.buffer.remove(i);
             }
             self.buffer[self.selection_start[0] + 1].replace_range(0..self.cursor_pos[1], "");
        } else {
            self.buffer[self.selection_start[0]].replace_range(self.selection_start[1]..self.cursor_pos[1], "");
        }

        self.cursor_pos = self.selection_start;

        if self.selection_start[1] != 0 {
            self.selection_start[1] -= 1;
        } else if self.selection_start[0] != 0 {
            self.selection_start = [self.selection_start[0] - 1, self.buffer[self.selection_start[0] - 1].len() - 1];
        }
    }

    fn backspace(&mut self) {
        if self.cursor_pos[1] != 0 {
            self.buffer[self.cursor_pos[0]].remove(self.cursor_pos[1] - 1);
            self.cursor_pos[1] -= 1;
        } else if self.cursor_pos[0] != 0 {
            let line_to_append = self.buffer.remove(self.cursor_pos[0]);
            self.cursor_pos = [self.cursor_pos[0] - 1, self.buffer[self.cursor_pos[0] - 1].len()];
            self.buffer[self.cursor_pos[0]].push_str(line_to_append.as_str());
        }
    }

    fn vmove_cursor(&mut self, x: isize) {
        if x > 0 {
            self.cursor_pos[0] = min(self.cursor_pos[0] + x as usize, self.buffer.len() - 1);
        } else {
            self.cursor_pos[0] = max(self.cursor_pos[0] as isize + x, 0) as usize;
        }

        if self.cursor_pos[1] > self.buffer[self.cursor_pos[0]].len() {
            self.cursor_pos[1] = self.buffer[self.cursor_pos[0]].len();
        }
    }
    
    fn hmove_cursor(&mut self, x: isize) {
        if x > 0 {
            self.cursor_pos[1] = min(self.cursor_pos[1] + x as usize, self.buffer[self.cursor_pos[0]].len());
        } else {
            self.cursor_pos[1] = max(self.cursor_pos[1] as isize + x, 0) as usize;
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

#[derive(Clone)]
struct Editor {
    font: &'static [u8],
}

impl Editor {
    fn new(font: &'static [u8]) -> Self {
        Self {
            font,
        }
    }
}

impl Widget<EditorData> for Editor {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut EditorData, env: &Env) {
        println!("Cursor: {:#?}\nSelection: {:#?}", data.cursor_pos, data.selection_start);
        match event {
            Event::WindowConnected => ctx.request_focus(),
            Event::KeyDown(key) => match &data.mode {
                EditorMode::Normal => match key.code {
                    Code::KeyI => data.mode = EditorMode::Insert,
                    Code::ArrowUp => data.vmove_cursor(-1),
                    Code::ArrowDown => data.vmove_cursor(1),
                    Code::ArrowLeft => data.hmove_cursor(-1),
                    Code::ArrowRight => data.hmove_cursor(1),
                    _ => ()
                }
                EditorMode::Insert => match key.code {
                    Code::Escape => data.mode = EditorMode::Normal,
                    Code::ArrowUp => data.vmove_cursor(-1),
                    Code::ArrowDown => data.vmove_cursor(1),
                    Code::ArrowLeft => data.hmove_cursor(-1),
                    Code::ArrowRight => data.hmove_cursor(1),

                    Code::Backspace => data.backspace(),
                    _ => {
                        data.insert(map_key_to_char(key).unwrap_or_default());
                    }
                }
                _ => (),
            }
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
        let text = data.format_buffer(ctx, self.font);
        let cursor_rect = Rect::new(text.1[0].x, text.1[0].y, text.1[1].x, text.1[1].y);

        ctx.fill(rounded, &Color::rgb8(255, 255, 0));
        ctx.fill(cursor_rect, &Color::rgb8(255, 255, 255));

        for rect in text.2 {
            let rect = Rect::new(rect[0].x, rect[0].y, rect[1].x, rect[1].y);
            ctx.fill(rect, &Color::rgb8(0, 0, 255))
        }

        ctx.stroke(rounded, &env.get(druid::theme::PRIMARY_DARK), 5.);

        for (line, point) in text.0 {
            ctx.draw_text(&line, point);
        }
    }
}

fn map_key_to_char(key: &KeyEvent) -> Option<&str> {
    if key.mods.ctrl() || key.mods.alt() || key.mods.contains(Modifiers::SUPER) {
        return None;
    } else {
        match key.code {
            Code::Space => Some(" "),
            Code::Tab => Some("\t"),
            Code::Enter => Some("\n"),
            Code::KeyA => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("A")} else {Some("a")}, 
            Code::KeyB => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("B")} else {Some("b")}, 
            Code::KeyC => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("C")} else {Some("c")}, 
            Code::KeyD => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("D")} else {Some("d")}, 
            Code::KeyE => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("E")} else {Some("e")}, 
            Code::KeyF => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("F")} else {Some("f")}, 
            Code::KeyG => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("G")} else {Some("g")}, 
            Code::KeyH => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("H")} else {Some("h")}, 
            Code::KeyI => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("I")} else {Some("i")}, 
            Code::KeyJ => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("J")} else {Some("j")}, 
            Code::KeyK => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("K")} else {Some("k")}, 
            Code::KeyL => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("L")} else {Some("l")}, 
            Code::KeyM => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("M")} else {Some("m")}, 
            Code::KeyN => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("N")} else {Some("n")}, 
            Code::KeyO => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("O")} else {Some("o")}, 
            Code::KeyP => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("P")} else {Some("p")}, 
            Code::KeyQ => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("Q")} else {Some("q")}, 
            Code::KeyR => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("R")} else {Some("r")}, 
            Code::KeyS => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("S")} else {Some("s")}, 
            Code::KeyT => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("T")} else {Some("t")}, 
            Code::KeyU => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("U")} else {Some("u")}, 
            Code::KeyV => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("V")} else {Some("v")}, 
            Code::KeyW => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("W")} else {Some("w")}, 
            Code::KeyX => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("X")} else {Some("x")}, 
            Code::KeyY => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("Y")} else {Some("y")}, 
            Code::KeyZ => if key.mods.shift() ^ key.mods.contains(Modifiers::CAPS_LOCK) {Some("Z")} else {Some("z")}, 
            Code::Digit0 => if key.mods.shift() {Some(")")} else {Some("0")},
            Code::Digit1 => if key.mods.shift() {Some("!")} else {Some("1")},
            Code::Digit2 => if key.mods.shift() {Some("@")} else {Some("2")},
            Code::Digit3 => if key.mods.shift() {Some("#")} else {Some("3")},
            Code::Digit4 => if key.mods.shift() {Some("$")} else {Some("4")},
            Code::Digit5 => if key.mods.shift() {Some("%")} else {Some("5")},
            Code::Digit6 => if key.mods.shift() {Some("^")} else {Some("6")},
            Code::Digit7 => if key.mods.shift() {Some("&")} else {Some("7")},
            Code::Digit8 => if key.mods.shift() {Some("*")} else {Some("8")},
            Code::Digit9 => if key.mods.shift() {Some("(")} else {Some("9")},
            Code::Backquote => if key.mods.shift() {Some("~")} else {Some("`")},
            Code::Equal => if key.mods.shift() {Some("+")} else {Some("=")},
            Code::Minus => if key.mods.shift() {Some("_")} else {Some("-")},
            Code::BracketLeft => if key.mods.shift() {Some("{")} else {Some("[")},
            Code::BracketRight => if key.mods.shift() {Some("}")} else {Some("]")},
            Code::Backslash => if key.mods.shift() {Some("|")} else {Some("\\")},
            Code::Semicolon => if key.mods.shift() {Some(":")} else {Some(";")},
            Code::Quote => if key.mods.shift() {Some("\"")} else {Some("'")},
            Code::Comma => if key.mods.shift() {Some("<")} else {Some(",")},
            Code::Period => if key.mods.shift() {Some(">")} else {Some(".")},
            Code::Slash => if key.mods.shift() {Some("?")} else {Some("/")},
            Code::Numpad0 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("0")} else {None},
            Code::Numpad1 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("1")} else {None},
            Code::Numpad2 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("2")} else {None},
            Code::Numpad3 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("3")} else {None},
            Code::Numpad4 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("4")} else {None},
            Code::Numpad5 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("5")} else {None},
            Code::Numpad6 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("6")} else {None},
            Code::Numpad7 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("7")} else {None},
            Code::Numpad8 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("8")} else {None},
            Code::Numpad9 => if key.mods.contains(Modifiers::NUM_LOCK) {Some("9")} else {None},
            Code::NumpadAdd => Some("+"),
            Code::NumpadSubtract => Some("-"),
            Code::NumpadMultiply => Some("*"),
            Code::NumpadDivide => Some("/"),
            Code::NumpadDecimal => Some("."),
            Code::NumpadComma => Some(","),
            Code::NumpadParenLeft => Some("("),
            Code::NumpadParenRight => Some(")"),
            _ => None
        }
    } 
}

fn build_ui() -> impl Widget<EditorData> {
    Editor::new(include_bytes!("../assets/Inconsolata-Regular.ttf"))
}

fn main() {
    let main_window = WindowDesc::new(build_ui())
        .window_size((1280., 720.))
        .title("Saucepan");
    let initial_data = EditorData { buffer: vec![String::from("")], mode: EditorMode::Normal, editor_size: [10, 10], window_pos: [0, 0], cursor_pos: [0, 0], selection_start: [0, 0] };

    AppLauncher::with_window(main_window)
        .launch(initial_data)
        .expect("Failed to launch Saucepan");
}


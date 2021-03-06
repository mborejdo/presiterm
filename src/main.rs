use anyhow::{anyhow, Context};
use clap::{App, Arg};
// use figlet_rs::FIGfont;
use std::sync::Arc;


use termwiz::surface::change::ImageData;
use termwiz::surface::TextureCoordinate;
use fs::File;
use ron::de::from_reader;
use serde::Deserialize;
use std::{fs, path::Path, process::Command, process::Stdio};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use termimad::{Area, MadSkin};
use termwiz::caps::Capabilities;
use termwiz::color::ColorAttribute;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::{Change, CursorVisibility, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{new_terminal, Terminal};
use termwiz::Error;

#[derive(Debug, Deserialize)]
struct Slides {
    files: Vec<FileTypes>,
}

#[derive(Debug, Deserialize)]
enum FileTypes {
    Text(String),
    Markdown(String),
    Image(String),
    Code(String, String),
    Command(Vec<String>),
}

fn text_size(s: &str) -> (usize, usize) {
    let w = 1 + s.lines().fold(0, |acc, l| acc.max(l.len()));

    (w, s.lines().count())
}

impl FileTypes {
    fn write_text(buf: &mut Surface, txt: &String) -> Result<(), Error> {
        let (width, height) = buf.dimensions();
        let top = height.saturating_sub(txt.lines().count()) / 2;

        for (idx, l) in txt.lines().enumerate() {
            let x = width.saturating_sub(l.len()) / 2;
            buf.add_change(Change::CursorPosition {
                x: Position::Absolute(x),
                y: Position::Absolute(top + idx),
            });
            buf.add_change(format!("{}", l));
        }
        buf.flush_changes_older_than(0);
        Ok(())
    }

    fn render(&self, buf: &mut Surface, margin: usize, ps: &SyntaxSet, ts: &ThemeSet) -> Result<(), Error> {
        match self {
            FileTypes::Image(path) => {
                let data = fs::read(Path::new(path))?;
                let image_data = Arc::new(ImageData::with_raw_data(data.into_boxed_slice().to_vec()));

                // suburl
                println!("{}", " ");

                buf.add_change(Change::Image(termwiz::surface::change::Image {
                    width: 35 as usize,
                    height: 35 as usize,
                    top_left: TextureCoordinate::new_f32(0.,0.),
                    bottom_right: TextureCoordinate::new_f32(1.,1.),
                    image: Arc::clone(&image_data),
                }));
                buf.flush_changes_older_than(0);
            }
            FileTypes::Text(txt) => {
                buf.add_change(Change::ClearScreen(ColorAttribute::Default));
                buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));

                Self::write_text(buf, txt)?;
            }
            FileTypes::Command(arr) => {
                let txt = Command::new("nu")
                    .arg("-c")
                    .args(arr)
                    .output()?;

                print!("{}", String::from_utf8_lossy(&txt.stdout))
            }
            FileTypes::Markdown(path) => {
                let (width, height) = buf.dimensions();
                let markdown = fs::read_to_string(Path::new(path))?;
                let (text_w, _) = text_size(markdown.as_str());
                let area_w = text_w.min(width - (margin * 2));
                let area_h = height / 2;
                let x = 0.max((width - area_w) / 2) as u16;
                let y = 0.max((height - area_h) / 2) as u16;

                MadSkin::default()
                    .write_in_area(&markdown, &Area::new(x, y, area_w as u16, area_h as u16))
                    .unwrap();
            }
            FileTypes::Code(path, syntax) => {
                let (width, height) = buf.dimensions();
                let content = fs::read_to_string(Path::new(path))?;
                let text_size = text_size(content.as_str());
                let x = (width - text_size.0) / 2;
                let y = (height - text_size.1) / 2;
                

                let syntax = ps.find_syntax_by_extension(syntax).unwrap();
                let mut highlighter = HighlightLines::new(syntax, &ts.themes["Solarized (light)"]);

                for (idx, line) in LinesWithEndings::from(content.as_str()).enumerate() {
                    let ranges: Vec<(Style, &str)> = highlighter.highlight(line, &ps);
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);

                    buf.add_change(Change::CursorPosition {
                        x: Position::Absolute(x),
                        y: Position::Absolute(y + idx),
                    });
                    buf.add_change(format!("{}", escaped.to_string()));
                    // buf.flush_changes_older_than(0);
                }

                Self::write_text(buf, &format!("{}", ""))?;
                // buf.add_change(Change::CursorPosition {
                //     x: Position::Absolute(0),
                //     y: Position::Absolute(0),
                // });
                buf.flush_changes_older_than(0);
            }
        }

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let matches = App::new("presiterm")
        .version("0.2.0")
        .author("@mib")
        .about("terminal presenter")
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .takes_value(true)
                .required(true)
                .help("input file (*.ron)"),
        )
        .get_matches();

    
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let mut idx = 0_usize;
    let margin = 2_usize;

    let f = File::open(matches.value_of("file").unwrap()).expect("Failed opening file");
    let slides: Slides = from_reader(f).expect("Failed to parse ron");

    let caps = Capabilities::new_from_env()?;
    println!("{:?}", caps);

    let terminal = new_terminal(caps)?;
    let mut buf = BufferedTerminal::new(terminal)?;

    buf.terminal().set_raw_mode()?;
    buf.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
    buf.flush()?;

    loop {
        buf.add_change(Change::ClearScreen(Default::default()));
        buf.flush()?;

        if let Some(file) = slides.files.get(idx) {
            file.render(&mut buf, margin, &ps, &ts)?;
        } else {
            break;
        }

        buf.flush()?;

        match buf.terminal().poll_input(None) {
            Ok(Some(input)) => match input {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => {
                    buf.add_change(Change::ClearScreen(Default::default()));
                    break;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::DownArrow,
                    ..
                }) => {
                    idx = idx.saturating_add(1);
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::UpArrow,
                    ..
                }) => {
                    idx = idx.saturating_sub(1);
                }
                _ => {
                    // print!("{:?}\r\n", input);
                }
            },
            Ok(None) => {}
            Err(e) => {
                buf.add_change(Change::ClearScreen(Default::default()));
               
                print!("{:?}\r\n", e);
                break;
            }
        }
    }

    buf.add_change(Change::CursorVisibility(CursorVisibility::Visible));
    buf.terminal().set_cooked_mode()?;
    buf.flush()?;
    Ok(())
}

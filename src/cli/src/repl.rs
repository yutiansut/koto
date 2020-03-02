use koto::{Error, Parser, Runtime};
use std::{
    fmt,
    io::{stdin, stdout, Write},
};
use termion::{
    clear, color, cursor, cursor::DetectCursorPos, event::Key, input::TermRead, raw::IntoRawMode,
    raw::RawTerminal, style,
};

pub struct Repl<'a> {
    parser: Parser,
    runtime: Runtime<'a>,

    input_history: Vec<String>,
    history_position: Option<usize>,
    input: String,
    cursor: Option<usize>,
}

impl<'a> Repl<'a> {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            runtime: Runtime::new(),
            input_history: Vec::new(),
            history_position: None,
            input: String::new(),
            cursor: None,
        }
    }

    pub fn run(&mut self) {
        let stdin = stdin();
        let mut stdout = stdout().into_raw_mode().unwrap();

        write!(stdout, "Koto\r\n» ").unwrap();
        stdout.flush().unwrap();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Up => {
                    if !self.input_history.is_empty() {
                        let new_position = match self.history_position {
                            Some(position) => {
                                if position > 0 {
                                    position - 1
                                } else {
                                    0
                                }
                            }
                            None => self.input_history.len() - 1,
                        };
                        self.input = self.input_history[new_position].clone();
                        self.history_position = Some(new_position);
                    }
                }
                Key::Down => {
                    self.history_position = match self.history_position {
                        Some(position) => {
                            if position < self.input_history.len() - 1 {
                                Some(position + 1)
                            } else {
                                None
                            }
                        }
                        None => None,
                    };
                    if let Some(position) = self.history_position {
                        self.input = self.input_history[position].clone();
                    } else {
                        self.input.clear();
                    }
                }
                Key::Backspace => {
                    let cursor = self.cursor;
                    match cursor {
                        Some(position) => {
                            let new_position = position - 1;
                            self.input.remove(new_position);
                            if self.input.is_empty() {
                                self.cursor = None;
                            } else {
                                self.cursor = Some(new_position);
                            }
                        }
                        None => {
                            self.input.pop();
                        }
                    }
                }
                Key::Char(c) => match c {
                    '\n' => {
                        write!(stdout, "\r\n").unwrap();
                        match self.parser.parse(&self.input) {
                            Ok(ast) => match self.runtime.run(&ast) {
                                Ok(result) => println!("{}", result),
                                Err(Error::RuntimeError { message, .. }) => {
                                    self.print_error(&mut stdout, &message)
                                }
                            },
                            Err(e) => self.print_error(&mut stdout, &e),
                        }
                        self.input_history.push(self.input.clone());
                        self.history_position = None;
                        self.input.clear();
                    }
                    _ => {
                        let cursor = self.cursor;
                        match cursor {
                            Some(position) => {
                                self.input.insert(position, c);
                                self.cursor = Some(position + 1);
                            }
                            None => self.input.push(c),
                        }
                    }
                },
                Key::Ctrl(c) => match c {
                    'c' => self.input.clear(),
                    'd' => {
                        if self.input.is_empty() {
                            std::process::exit(0)
                        }
                    }
                    _ => {}
                },
                _ => {}
            }

            let (_, cursor_y) = stdout.cursor_pos().unwrap();

            write!(
                stdout,
                "{}{}» {}",
                cursor::Goto(1, cursor_y),
                clear::CurrentLine,
                self.input
            )
            .unwrap();

            stdout.flush().unwrap();
        }

        // loop {
        //     print!("> ");
        //     std::io::stdout().flush().expect("Error flushing output");
        //     std::io::stdin()
        //         .read_line(&mut input)
        //         .expect("Error getting input");
        //     match self.parser.parse(&input) {
        //         Ok(ast) => match self.runtime.run(&ast) {
        //             Ok(result) => println!("{}", result),
        //             Err(Error::RuntimeError { message, .. }) => println!("Error: {}", message),
        //         },
        //         Err(e) => {
        //             println!("Error parsing input: {}", e);
        //         }
        //     }
        //     input.clear();
        // }
    }

    fn print_error<T, E>(&self, stdout: &mut RawTerminal<T>, error: &E)
    where
        T: Write,
        E: fmt::Display,
    {
        write!(
            stdout,
            "{red}error{reset}: {bold}",
            red = color::Fg(color::Red),
            bold = style::Bold,
            reset = style::Reset,
        )
        .unwrap();
        stdout.suspend_raw_mode().unwrap();
        println!("{}", error);
        stdout.activate_raw_mode().unwrap();
        write!(stdout, "{}\r\n", style::Reset).unwrap();
    }
}
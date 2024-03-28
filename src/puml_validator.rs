use colored::Colorize;
use regex::Regex;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
struct PumlErr {
    line_number: usize,
    msg: String,
}

#[derive(Clone)]
struct Puml {
    starting_line: usize,
    lines: Vec<String>,
    errors: Vec<PumlErr>,
}

impl Puml {
    fn new() -> Puml {
        Puml {
            starting_line: 0,
            lines: Vec::new(),
            errors: Vec::new(),
        }
    }
    fn validate(&mut self) {

        // validate missing :
        self.validate_pattern(
            r"[^;]*?;$",
            r":[^;]*?;",
            "missing ':' at the beginning of the line (doesnt check multiline)",
        );

        // validate missing ;
        self.validate_pattern(
            r":[^;]*?",
            r":[^;]*?;$",
            "missing ';' at the end of the line (doesnt check multiline)",
        );

        // validate if else endif
        self.validate_open_close(
            r"^if\s*\([^)]*\)\s*then\s*\([^)]*\)",
            Some(r"^(?:else\s*\([^)]*\)|(?:\([^)]*\))?\s*elseif\s*(?:\([^)]*\))?\s*then\s*(?:\([^)]*\))?)"),
            r"^endif",
            r"if (*) then (*)",
            "endif",
        );

        // validate switch
        self.validate_open_close(
            r"^switch\s*\((.*?)\)",
            Some(r"^case\s*\((.*?)\)"),
            r"^endswitch$",
            r"switch (*)",
            "endswitch",
        );

        // validate repeat while
        self.validate_open_close(
            r"^repeat\b",
            None,
            r"^repeat\s+while\s*\((.*?)\)\s+is\s+(.*)",
            r"repeat",
            "repeat while (*) is (*)",
        );

        // validate while
        self.validate_open_close(
            r"^while\s*\((.*?)\)",
            None,
            r"^endwhile\s*\((.*?)\)",
            r"while (*) [is (*)]",
            "endwhile [(*)]",
        );

        // validate fork
        self.validate_open_close(
            r"^fork",
            Some(r"^fork again$"),
            r"^end fork|^end merge",
            r"fork",
            "end fork|end merge",
        );

        // validate split
        self.validate_open_close(
            r"^split",
            Some(r"^fork again$"),
            r"^end split",
            "split",
            "end split",
        );

        self.errors
            .sort_by(|a, b| a.line_number.cmp(&b.line_number));
    }

    fn print_errors(&self) {
        println!();
        println!("PlantUML starting at line {}:", self.starting_line);
        if self.errors.len() == 0 {
            println!("OK!");
        }
        for PumlErr { line_number, msg } in self.errors.iter() {
            println!(
                "{}: {} {}",
                line_number.to_string().color("grey"),
                self.lines[*line_number].bold(),
                msg.red()
            );
        }
        println!();
    }

    fn validate_pattern(&mut self, simple_pattern: &str, validation_pattern: &str, msg: &str) {
        let simple_pattern = Regex::new(simple_pattern).unwrap();
        let validation_pattern = Regex::new(validation_pattern).unwrap();
        for (line_number, line) in self.lines.iter().enumerate() {
            let line = line.trim();
            if simple_pattern.is_match(line) {
                if !validation_pattern.is_match(line) {
                    self.errors.push(PumlErr {
                        line_number,
                        msg: format!("<- {}", msg),
                    })
                }
            }
        }
    }

    fn validate_open_close(
        &mut self,
        open: &str,
        middle: Option<&str>,
        close: &str,
        open_text: &str,
        close_text: &str,
    ) {
        let open = Regex::new(open).unwrap();
        let middle = match middle {
            Some(str) => Some(Regex::new(str).unwrap()),
            None => None,
        };
        let close = Regex::new(close).unwrap();
        let mut opening_stack: Vec<usize> = Vec::new();

        for (line_number, line) in self.lines.iter().enumerate() {
            let line = line.trim();
            if open.is_match(line) {
                opening_stack.push(line_number);
                continue;
            }

            if let Some(middle) = &middle {
                if middle.is_match(line) {
                    if opening_stack.is_empty() {
                        self.errors.push(PumlErr {
                            line_number,
                            msg: format!("<- no opening {} found", open_text),
                        });
                    }
                    continue;
                }
            }

            if close.is_match(line) {
                match opening_stack.pop() {
                    Some(_) => {}
                    None => {
                        self.errors.push(PumlErr {
                            line_number,
                            msg: format!("<- no opening {} found", open_text),
                        });
                    }
                }
                continue;
            }
        }

        for line_number in opening_stack {
            self.errors.push(PumlErr {
                line_number,
                msg: format!("<- no closing {} found", close_text),
            });
        }
    }
}

struct PumlFile {
    filename: String,
    pumls: Vec<Puml>,
}

impl PumlFile {
    fn new(path: &PathBuf) -> Option<PumlFile> {
        match fs::read_to_string(path) {
            Ok(content) => {
                let mut reading_uml = false;

                let mut puml_file = PumlFile {
                    filename: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    pumls: Vec::new(),
                };

                let mut puml_buffer = Puml {
                    starting_line: 0,
                    lines: Vec::new(),
                    errors: Vec::new(),
                };

                for (line_number, line) in content.lines().enumerate() {
                    let line = line.trim();

                    if line.starts_with("@enduml") {
                        if reading_uml {
                            reading_uml = false;
                            puml_file.pumls.push(puml_buffer.clone());
                            puml_buffer = Puml::new();
                        } else {
                            // error because we are not reading an open uml section
                            eprintln!();
                            eprintln!(
                                "skipping file {:?} because of error while parsing uml sections!",
                                path
                            );
                            eprintln!(
                                "{}: {}  <- closing uml section without starting one before with @startuml",
                                line_number, line
                            );
                            eprintln!();
                            return None;
                        }
                    }

                    // read lines belonging to an uml into the buffer
                    if reading_uml {
                        puml_buffer.lines.push(line.to_string());
                    }

                    // check if the current lines are part of an uml
                    if line.starts_with("@startuml") {
                        if !reading_uml {
                            reading_uml = true;
                            puml_buffer.starting_line = line_number;
                        } else {
                            // error because we are already reading an open uml section
                            eprintln!();
                            eprintln!(
                                "skipping file {:?} because of error while parsing uml sections!",
                                path
                            );
                            eprintln!(
                                "{}: @startuml... <- opening uml section",
                                puml_buffer.starting_line
                            );
                            eprintln!(
                                "{}: {}  <- opening another uml section without closing the previous at line {}",
                                line_number, line, puml_buffer.starting_line
                            );
                            eprintln!();
                            return None;
                        }
                    }
                }

                Some(puml_file)
            }
            Err(e) => {
                eprintln!("ignoring file {:?}, error while reading: {}", path, e);
                None
            }
        }
    }

    fn validate(&mut self) {
        for puml in self.pumls.iter_mut() {
            puml.validate();
        }
    }

    fn print_errors(&self) {
        println!("In file {}:", self.filename);
        for puml in self.pumls.iter() {
            puml.print_errors();
        }
    }
}

pub struct PumlValidator {
    puml_files: Vec<PumlFile>,
}

impl PumlValidator {
    pub fn new(files: Vec<PathBuf>) -> PumlValidator {
        let mut validator = PumlValidator {
            puml_files: Vec::new(),
        };

        for file in files.iter() {
            match PumlFile::new(file) {
                Some(puml_file) => {
                    validator.puml_files.push(puml_file);
                }
                None => {}
            }
        }

        validator
    }
    pub fn validate(&mut self) {
        for puml_file in self.puml_files.iter_mut() {
            puml_file.validate();
        }
    }
    pub fn print_errors(&self) {
        for puml_file in self.puml_files.iter() {
            puml_file.print_errors();
        }
    }
}

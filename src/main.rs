use std::env;
use std::path::PathBuf;

use puml_validator::PumlValidator;

mod puml_validator;

fn main() {
    let paths: Vec<PathBuf> = env::args().skip(1).map(PathBuf::from).collect();
    if paths.len() == 0 {
        println!("usage: pumlchk <file1> <file2> ...");
        return;
    }
    let mut validator = PumlValidator::new(paths);
    validator.validate();
    validator.print_errors();
}

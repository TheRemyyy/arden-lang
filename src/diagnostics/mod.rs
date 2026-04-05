use colored::Colorize;
use crate::borrowck;
use crate::typeck;

pub(crate) fn render_type_errors(errors: Vec<typeck::TypeError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!("{}: {}\n", "error".red().bold(), error.message));
    }
    rendered
}

pub(crate) fn render_borrow_errors(errors: Vec<borrowck::BorrowError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!("{}: {}\n", "error[E0505]".red().bold(), error.message));
    }
    rendered
}

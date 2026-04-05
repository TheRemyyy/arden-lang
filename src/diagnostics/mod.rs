use crate::borrowck;
use crate::typeck;
pub(crate) fn render_type_errors(errors: Vec<typeck::TypeError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!("\x1b[1;31merror\x1b[0m: {}\n", error.message));
    }
    rendered
}

pub(crate) fn render_borrow_errors(errors: Vec<borrowck::BorrowError>) -> String {
    let mut rendered = String::new();
    for error in errors {
        rendered.push_str(&format!(
            "\x1b[1;31merror[E0505]\x1b[0m: {}\n",
            error.message
        ));
    }
    rendered
}

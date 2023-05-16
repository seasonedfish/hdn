use std::fmt;
use std::fs::read;
use std::process::exit;
use owo_colors::{OwoColorize, Style};

use similar::{ChangeTag, TextDiff};

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:>4}", idx + 1),
        }
    }
}

pub fn print_diff(string1: &String, string2: &String) {
    let diff = TextDiff::from_lines(string1, string2);

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, style) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red().bold()),
                    ChangeTag::Insert => ("+", Style::new().green().bold()),
                    ChangeTag::Equal => (" ", Style::new()),
                };
                print!(
                    "{}|{}",
                    Line(change.new_index()),
                    style.style(sign),
                );
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        print!("{}", style.style(value).underline());
                    } else {
                        print!("{}", style.style(value));
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }
}
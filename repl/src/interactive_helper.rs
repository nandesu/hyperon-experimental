
use std::borrow::Cow::{self, Borrowed, Owned};
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use rustyline::completion::FilenameCompleter;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::{Validator, ValidationContext, ValidationResult};
use rustyline::error::ReadlineError;
use rustyline::{Completer, Helper, Hinter};

use hyperon::metta::text::{SExprParser, SyntaxNodeType};

use crate::config_params::*;
use crate::metta_shim::MettaShim;

#[derive(Helper, Completer, Hinter)]
pub struct ReplHelper {
    pub metta: RefCell<MettaShim>,
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    pub colored_prompt: String,
    cursor_bracket: std::cell::Cell<Option<(u8, usize)>>, // If the cursor is over or near a bracket to match
    pub force_submit: Arc<Mutex<bool>>, // We use this to communicate between the key event handler and the Validator
    checked_line: RefCell<String>,
    style: StyleSettings,
}

#[derive(Default)]
struct StyleSettings {
    bracket_styles: Vec<String>,
    comment_style: String,
    variable_style: String,
    symbol_style: String,
    string_style: String,
    error_style: String,
    bracket_match_style: String,
    bracket_match_enabled: bool,
}

impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        //Render the hints in a lighter font
        Owned("\x1b[2m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {

        //See if we need to highlight the bracket matching the cursor position
        //BUG: this could possibly get tripped up by parenthesis inside comments and string literals
        let mut blink_char = None;
        if let Some((bracket, pos)) = self.cursor_bracket.get() {
            blink_char = find_matching_bracket(line, pos, bracket);
        }

        //Iterate over the syntax nodes generated by the parser, coloring them appropriately
        let mut colored_line = String::with_capacity(line.len() * 2);
        let mut bracket_depth = 0;
        self.metta.borrow_mut().inside_env(|_metta| {
            let mut parser = SExprParser::new(line);
            loop {
                match parser.parse_to_syntax_tree() {
                    Some(root_node) => {
                        root_node.visit_depth_first(|node| {
                            // We will only render the leaf nodes in the syntax tree
                            if !node.node_type.is_leaf() {
                                return;
                            }

                            let mut style_sequence: Vec<&str> = vec![];

                            // TODO: In the future, We'll want to be able to use the type system to assign styling,
                            //   which is going to mean looking at Atoms, and not the tokens they were built from

                            //Set up the style for the node
                            match node.node_type {
                                SyntaxNodeType::Comment => {
                                    style_sequence.push(&self.style.comment_style);
                                },
                                SyntaxNodeType::VariableToken => {
                                    style_sequence.push(&self.style.variable_style);
                                },
                                SyntaxNodeType::StringToken => {
                                    style_sequence.push(&self.style.string_style);
                                },
                                SyntaxNodeType::WordToken => {
                                    style_sequence.push(&self.style.symbol_style);
                                },
                                SyntaxNodeType::OpenParen => {
                                    style_sequence.push(&self.style.bracket_styles[bracket_depth%self.style.bracket_styles.len()]);
                                    bracket_depth += 1;
                                },
                                SyntaxNodeType::CloseParen => {
                                    if bracket_depth > 0 {
                                        bracket_depth -= 1;
                                        style_sequence.push(&self.style.bracket_styles[bracket_depth%self.style.bracket_styles.len()]);
                                    } else {
                                        style_sequence.push(&self.style.error_style);
                                    }
                                },
                                SyntaxNodeType::LeftoverText => {
                                    style_sequence.push(&self.style.error_style);
                                }
                                _ => { }
                            }

                            //See if we need to render this node with the "bracket blink"
                            if self.style.bracket_match_enabled {
                                if let Some((_matching_char, blink_idx)) = &blink_char {
                                    if node.src_range.contains(blink_idx) {
                                        style_sequence.push(&self.style.bracket_match_style);
                                    }
                                }
                            }

                            //Push the styles to the buffer
                            let style_count = style_sequence.len();
                            if style_count > 0 {
                                colored_line.push_str("\x1b[");
                                for (style_idx, style) in style_sequence.into_iter().enumerate() {
                                    colored_line.push_str(style);
                                    if style_idx < style_count-1 {
                                        colored_line.push(';');
                                    }
                                }
                                colored_line.push('m');
                            }

                            //Push the node itself to the buffer
                            colored_line.push_str(&line[node.src_range.clone()]);

                            //And push an undo sequence, if the node was stylized
                            if style_count > 0 {
                                colored_line.push_str("\x1b[0m");
                            }
                        });
                    },
                    None => break,
                }
            }
        });

        Owned(colored_line)
    }

    fn highlight_char(&self, line: &str, pos: usize, final_render: bool) -> bool {
        if final_render {
            self.cursor_bracket.set(None);
        } else {
            self.cursor_bracket.set(check_bracket(line, pos));
        }
        true
    }
}

impl Validator for ReplHelper {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {

        //This validator implements the following behavior:
        // * if user hits enter and the line is valid, and the cursor is at the end of the line, it will be submitted.
        // * if user hits ctrl-J (force submit) and the line is valid, it will be submitted regardless of cursor position
        // * if user hits enter and the line is invalid, a newline will be inserted at the cursor position, unless
        //     a linefeed has just been added to the end of the line, in which case a syntax error is reported
        // * if user hits ctrl-J (force submit) and line is invalid, it will be a syntax error, regardless of cursor position
        let force_submit = *self.force_submit.lock().unwrap();
        *self.force_submit.lock().unwrap() = false;
        let mut validation_result = ValidationResult::Incomplete;
        self.metta.borrow_mut().inside_env(|metta| {
            let mut parser = SExprParser::new(ctx.input());
            loop {
                let result = parser.parse(&metta.metta.tokenizer().borrow());

                match result {
                    Ok(Some(_atom)) => (),
                    Ok(None) => {
                        validation_result = ValidationResult::Valid(None);
                        *self.checked_line.borrow_mut() = "".to_string();
                        break
                    },
                    Err(err) => {
                        let input = ctx.input();
                        if input.len() < 1 {
                            break;
                        }
                        if !force_submit &&
                            (*self.checked_line.borrow() != &input[0..input.len()-1] || input.as_bytes()[input.len()-1] != b'\n') {
                            *self.checked_line.borrow_mut() = ctx.input().to_string();
                        } else {
                            validation_result = ValidationResult::Invalid(Some(
                                format!(" - \x1b[0;{}m{}\x1b[0m", self.style.error_style, err)
                            ));
                        }
                        break;
                    }
                }
            }
        });
        Ok(validation_result)
    }

}

impl ReplHelper {
    pub fn new(mut metta: MettaShim) -> Self {

        let style = StyleSettings::new(&mut metta);

        Self {
            metta: RefCell::new(metta),
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter {},
            colored_prompt: "".to_owned(),
            cursor_bracket: std::cell::Cell::new(None),
            force_submit: Arc::new(Mutex::new(false)),
            checked_line: RefCell::new(String::new()),
            style,
        }
    }
}

impl StyleSettings {
    const ERR_STR: &str = "Fatal Error: Invalid REPL config";
    pub fn new(metta_shim: &mut MettaShim) -> Self {
        Self {
            bracket_styles: metta_shim.get_config_expr_vec(CFG_BRACKET_STYLES).expect(Self::ERR_STR),
            comment_style: metta_shim.get_config_string(CFG_COMMENT_STYLE).expect(Self::ERR_STR),
            variable_style: metta_shim.get_config_string(CFG_VARIABLE_STYLE).expect(Self::ERR_STR),
            symbol_style: metta_shim.get_config_string(CFG_SYMBOL_STYLE).expect(Self::ERR_STR),
            string_style: metta_shim.get_config_string(CFG_STRING_STYLE).expect(Self::ERR_STR),
            error_style: metta_shim.get_config_string(CFG_ERROR_STYLE).expect(Self::ERR_STR),
            bracket_match_style: metta_shim.get_config_string(CFG_BRACKET_MATCH_STYLE).expect(Self::ERR_STR),
            bracket_match_enabled: metta_shim.get_config_atom(CFG_BRACKET_MATCH_ENABLED).map(|_bool_atom| true).unwrap_or(true), //TODO, make this work when we can bridge value atoms
        }
    }
}

//*-=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*/
// LICENSE.  The below functions are based on a functions with the same names in the highlight.rs
// file of the rustyline crate source, version 12.0.0.
// Incorporated here under the terms of the MIT license.
//*-=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*=-*/

fn find_matching_bracket(line: &str, pos: usize, bracket: u8) -> Option<(u8, usize)> {
    let matching = matching_bracket(bracket);
    let mut idx;
    let mut unmatched = 1;
    if is_open_bracket(bracket) {
        // forward search
        idx = pos + 1;
        let bytes = &line.as_bytes()[idx..];
        for b in bytes {
            if *b == matching {
                unmatched -= 1;
                if unmatched == 0 {
                    debug_assert_eq!(matching, line.as_bytes()[idx]);
                    return Some((matching, idx));
                }
            } else if *b == bracket {
                unmatched += 1;
            }
            idx += 1;
        }
        debug_assert_eq!(idx, line.len());
    } else {
        // backward search
        idx = pos;
        let bytes = &line.as_bytes()[..idx];
        for b in bytes.iter().rev() {
            if *b == matching {
                unmatched -= 1;
                if unmatched == 0 {
                    debug_assert_eq!(matching, line.as_bytes()[idx - 1]);
                    return Some((matching, idx - 1));
                }
            } else if *b == bracket {
                unmatched += 1;
            }
            idx -= 1;
        }
        debug_assert_eq!(idx, 0);
    }
    None
}

// check under or before the cursor
fn check_bracket(line: &str, pos: usize) -> Option<(u8, usize)> {
    if line.is_empty() {
        return None;
    }
    let mut pos = pos;
    if pos >= line.len() {
        pos = line.len() - 1; // before cursor
        let b = line.as_bytes()[pos]; // previous byte
        if is_close_bracket(b) {
            Some((b, pos))
        } else {
            None
        }
    } else {
        let mut under_cursor = true;
        loop {
            let b = line.as_bytes()[pos];
            if is_close_bracket(b) {
                return if pos == 0 { None } else { Some((b, pos)) };
            } else if is_open_bracket(b) {
                return if pos + 1 == line.len() {
                    None
                } else {
                    Some((b, pos))
                };
            } else if under_cursor && pos > 0 {
                under_cursor = false;
                pos -= 1; // or before cursor
            } else {
                return None;
            }
        }
    }
}

const fn matching_bracket(bracket: u8) -> u8 {
    match bracket {
        b'{' => b'}',
        b'}' => b'{',
        b'[' => b']',
        b']' => b'[',
        b'(' => b')',
        b')' => b'(',
        b => b,
    }
}
const fn is_open_bracket(bracket: u8) -> bool {
    matches!(bracket, b'{' | b'[' | b'(')
}
const fn is_close_bracket(bracket: u8) -> bool {
    matches!(bracket, b'}' | b']' | b')')
}

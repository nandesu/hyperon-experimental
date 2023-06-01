
#![feature(test)]

extern crate test;

use test::{Bencher, black_box};

use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use hyperon::*;
use hyperon::space::grounding::*;

//Specify the test file path to run this benchmark
#[ignore]
#[bench]
fn natural_language_expressions(bencher: &mut Bencher) -> std::io::Result<()> {

    let mut space = GroundingSpace::new();

    //The complete works of Shakespeare can be downloaded as a single file here:
    // https://ocw.mit.edu/ans7870/6/6.006/s08/lecturenotes/files/t8.shakespeare.txt
    // ~200k expressions
    // ~900k words
    let file = File::open("/Users/admin/Desktop/t8.shakespeare.txt")?;

    //Parse the file, with each sentence clause as an expression
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut cur_symbols = vec![sym!("Sentence")];
    let mut expr_count = 0;
    while reader.read_line(&mut line)? > 0 {

        const TERMINATORS: &[char] = &[',', '.', ';', '?', '\"', '-', '[', ']'];
        const SEPARATORS: &[char] = &[' ', '\t', '\n'];
        const IGNORE_CHARS: &[char] = &['\''];

        for clause in line.split_inclusive(TERMINATORS) {
            for sym in clause.split(SEPARATORS) {
                let end = sym.ends_with(TERMINATORS);
                let ignore_chars = [TERMINATORS, IGNORE_CHARS].concat();
                let sym = sym.replace(&ignore_chars[..], "");

                if sym.len() > 0 {
                    cur_symbols.push(Atom::sym(sym));
                }
                if end {
                    let expr = Atom::expr(&cur_symbols[..]);
                    cur_symbols = vec![sym!("Sentence")];
                    space.add(expr);
                    expr_count += 1;
                }
            }
        }
        line.clear();
    }
    println!("expr_count = {expr_count}");

    // // Coriolanus, Act 1, Scene 1. (postfix)
    // let query_expr_1 = &expr!("Sentence" A B C "singularity");
    // let reference_binding_1 = bind_set![{ A: sym!("More"), B: sym!("than"), C: Atom::sym("his") }];

    // Coriolanus, Act 1, Scene 1. (prefix)
    let query_expr_2 = &expr!("Sentence" "More" B C "singularity");
    let reference_binding_2 = bind_set![{ B: sym!("than"), C: Atom::sym("his") }];

    // // Matches 21 Different Expressions
    // let query_expr_3 = &expr!("Sentence" "More" B C D);

    bencher.iter(|| {
        let result_binding = black_box(space.query(query_expr_2));
        assert_eq!(result_binding, reference_binding_2);
    });

    Ok(())
}

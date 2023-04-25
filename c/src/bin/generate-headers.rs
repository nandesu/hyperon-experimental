
use std::env;

extern crate hyperonc;

fn main() -> ::std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let out_file = if args.len() == 3 && args[1] == "-o" {
        std::path::Path::new(&args[2])
    } else {
        std::path::Path::new("hyperon.h")
    };
  
    println!("HyperonC: Generating Headers to: {out_file:?}");

    let parent = out_file.parent().unwrap();
    let _ = std::fs::create_dir_all(parent);

    ::hyperonc::generate_headers(&out_file)
}

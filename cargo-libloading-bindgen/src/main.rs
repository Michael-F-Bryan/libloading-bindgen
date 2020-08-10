use anyhow::Error;
use libloading_bindgen::BindingStrategy;
use quote::ToTokens;
use regex::Regex;
use std::path::PathBuf;
use structopt::StructOpt;
use syn::ForeignItemFn;

fn main() -> Result<(), Error> {
    let args = Args::from_args();

    let mut builder =
        bindgen::builder().header(args.header_file.display().to_string());

    for whitelist_function in &args.whitelist_functions {
        // hack because builder methods take self by value
        let tmp = std::mem::replace(&mut builder, bindgen::builder());
        builder = tmp.whitelist_function(whitelist_function);
    }

    let strategy = args.strategy()?;
    let bindings = libloading_bindgen::generate_bindings(builder, &strategy)?
        .to_token_stream()
        .to_string();

    match args.output {
        Some(path) => std::fs::write(&path, bindings.as_bytes())?,
        None => println!("{}", bindings),
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(
        short = "f",
        long = "whitelist-function",
        multiple = true,
        help = "Functions to include in the bindings (can be a regex)"
    )]
    whitelist_functions: Vec<String>,
    #[structopt(short, long, help = "Where to write the bindings to")]
    output: Option<PathBuf>,
    #[structopt(
        parse(from_os_str),
        help = "The header file to generate bindings for"
    )]
    header_file: PathBuf,
}

impl Args {
    fn strategy(&self) -> Result<impl BindingStrategy, Error> {
        let mut whitelist_functions = Vec::new();

        for pattern in &self.whitelist_functions {
            whitelist_functions.push(Regex::new(pattern)?);
        }

        Ok(Strategy {
            whitelist_functions,
        })
    }
}

#[derive(Debug)]
struct Strategy {
    whitelist_functions: Vec<Regex>,
}

impl BindingStrategy for Strategy {
    fn should_include(&self, item: &ForeignItemFn) -> bool {
        let name = item.sig.ident.to_string();

        self.whitelist_functions
            .iter()
            .any(|pattern| pattern.is_match(&name))
    }
}

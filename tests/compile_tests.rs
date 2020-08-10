use anyhow::{Context, Error};
use cbindgen::{Config, Language};
use libloading_bindgen::BindingStrategy;
use quote::ToTokens;
use std::{
    env,
    fs::File,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use syn::ForeignItemFn;
use tempfile::Builder;

macro_rules! integration_test {
    ($directory:ident) => {
        #[test]
        fn $directory() -> Result<(), Error> {
            let name = stringify!($directory);

            let test_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join(name);

            let output_dir = Builder::new()
                .prefix("libloading-bindgen-")
                .tempdir()
                .context("Unable to create a temp directory")?;
            let ret = compile_and_test(name, &test_dir, output_dir.path());

            if env::var("RETAIN_GENERATED_CODE").is_ok() || ret.is_err() {
                let persisted_path = output_dir.into_path();
                eprintln!(
                    "Saved \"{}\" generated code to \"{}\"",
                    name,
                    persisted_path.display()
                );
            } else {
                output_dir.close().context("Unable to cleanup the test")?;
            }

            ret
        }
    };
}

integration_test!(smoke_test);

fn compile_and_test(
    name: &str,
    test_dir: &Path,
    output_dir: &Path,
) -> Result<(), Error> {
    let library_code = test_dir.join("native.rs");
    let native_dir = output_dir.join("native");
    let native_library =
        compile_native_library(name, &native_dir, &library_code)
            .context("Compiling the native library failed")?;

    let test_code = test_dir.join("test.rs");
    let test_file_manifest = generate_test_binary(
        name,
        output_dir.join(name),
        &native_dir,
        &test_code,
    )?;

    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(test_file_manifest)
        .arg("--")
        .arg(&native_library)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .output()
        .context("Unable to execute `cargo run`")?;
    if !output.status.success() {
        anyhow::bail!(
            "Command failed with exit code: {:?}",
            output.status.code(),
        );
    }

    Ok(())
}

const TEST_CARGO_TOML: &str = r#"
[package]
name = "$PACKAGE_NAME"
version = "0.1.0"
authors = ["Michael-F-Bryan <michaelfbryan@gmail.com>"]
publish = false
edition = "2018"

[dependencies]
libloading = "0.6"
"#;

fn generate_test_binary<P>(
    name: &str,
    output_dir: P,
    native_dir: &Path,
    test_code: &Path,
) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let output_dir = output_dir.as_ref();
    ensure_directory_exists(output_dir)?;

    let cargo_toml = output_dir.join("Cargo.toml");
    let cargo_toml_src = TEST_CARGO_TOML.replace("$PACKAGE_NAME", name);
    std::fs::write(&cargo_toml, cargo_toml_src.as_bytes())
        .context("Unable to save Cargo.toml")?;

    let src_dir = output_dir.join("src");
    ensure_directory_exists(&src_dir)?;

    let main_rs = src_dir.join("main.rs");
    std::fs::copy(test_code, &main_rs).with_context(|| {
        format!(
            "Couldn't copy \"{}\" to \"{}\"",
            test_code.display(),
            main_rs.display()
        )
    })?;

    // generate a C-style header file for our native library
    let cfg = Config {
        language: Language::C,
        ..Default::default()
    };
    let c_bindings = cbindgen::generate_with_config(native_dir, cfg)?;
    let bindings_h = output_dir.join("bindings.h");
    let mut f = File::create(&bindings_h)?;
    c_bindings.write(&mut f);
    f.sync_all()?;

    let strategy = StartsWithName { name };
    let bindgen_builder = bindgen::builder()
        .header(bindings_h.display().to_string())
        .whitelist_function(format!("{}.*", name));
    let rust_bindings =
        libloading_bindgen::generate_bindings(bindgen_builder, &strategy)?
            .to_token_stream()
            .to_string();

    let bindings_rs = src_dir.join("bindings.rs");
    std::fs::write(&bindings_rs, rust_bindings.as_bytes()).with_context(
        || {
            format!(
                "Couldn't save generated bindings to \"{}\"",
                bindings_rs.display()
            )
        },
    )?;

    Ok(cargo_toml)
}

struct StartsWithName<'a> {
    name: &'a str,
}

impl<'a> BindingStrategy for StartsWithName<'a> {
    fn should_include(&self, item: &ForeignItemFn) -> bool {
        let function_name = item.sig.ident.to_string();
        function_name.starts_with(self.name)
    }
}

const NATIVE_CARGO_TOML: &str = r#"
[package]
name = "$PACKAGE_NAME_native"
version = "0.1.0"
authors = ["Michael-F-Bryan <michaelfbryan@gmail.com>"]
publish = false
edition = "2018"

[lib]
crate-type = ["cdylib"]
"#;

fn compile_native_library<P>(
    name: &str,
    output_dir: P,
    library_code: &Path,
) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let output_dir = output_dir.as_ref();

    ensure_directory_exists(&output_dir)?;

    let cargo_toml = output_dir.join("Cargo.toml");
    let cargo_toml_src = NATIVE_CARGO_TOML.replace("$PACKAGE_NAME", name);
    std::fs::write(&cargo_toml, cargo_toml_src.as_bytes())
        .context("Unable to save Cargo.toml")?;

    let src_dir = output_dir.join("src");
    ensure_directory_exists(&src_dir)?;

    let lib_rs = src_dir.join("lib.rs");
    std::fs::copy(library_code, &lib_rs).with_context(|| {
        format!(
            "Couldn't copy \"{}\" to \"{}\"",
            library_code.display(),
            lib_rs.display()
        )
    })?;

    let output = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&cargo_toml)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .output()
        .context("Unable to run cargo")?;

    if !output.status.success() {
        anyhow::bail!(
            "Command failed with exit code: {:?}",
            output.status.code(),
        );
    }

    let compiled_library_path = output_dir
        .join("target")
        .join("debug")
        .join(native_library_name(name));

    if !compiled_library_path.exists() {
        anyhow::bail!(
            "Unable to find the compiled library at \"{}\"",
            compiled_library_path.display()
        );
    }

    Ok(compiled_library_path)
}

#[cfg(unix)]
fn native_library_name(test_name: &str) -> String {
    format!("lib{}_native.so", test_name)
}

fn ensure_directory_exists(dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Couldn't create \"{}\"", dir.display()))
}

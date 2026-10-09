use {
    aargvark::{
        Aargvark,
        traits_impls::AargvarkJson,
        vark,
    },
    flowcontrol::ta_return,
    loga::fatal,
    schemask::{
        Maskoidy,
        Schemask,
    },
};

/// Does all things with schemask schemas.
#[derive(Aargvark)]
pub struct Args {
    command: Command,
}

#[derive(Aargvark)]
#[vark(break_help)]
pub enum Command {
    GenerateJava(CommandGenerateJava),
    /// Generate markdown documentation for a schemask. Outputs the markdown to stdout.
    GenerateMarkdown(CommandGenerateMarkdown),
    /// Generate rust types that match a schemask. Outputs the types to stdout.
    GenerateRust(CommandGenerateRust),
    /// Generate typescript definitions that match a schemask. Outputs the definitions
    /// to stdout.
    GenerateTypescript(CommandGenerateTypescript),
    /// Dump the schemask schema for schemask itself. Outputs the
    SchemaskSchema,
    /// Validate a json file matches a schemask.
    Validate(CommandValidate),
}

#[derive(Aargvark)]
pub struct CommandGenerateJava {
    schema: AargvarkJson<Schemask>,
    #[vark(flag = "--package")]
    package: String,
    #[vark(flag = "--runtime-package")]
    runtime_package: String,
    #[vark(flag = "--prelude-package")]
    prelude_package: String,
    #[vark(flag = "--out")]
    out: std::path::PathBuf,
}

#[derive(Aargvark)]
pub struct CommandGenerateMarkdown {
    schema: AargvarkJson<Schemask>,
}

#[derive(Aargvark)]
pub struct CommandGenerateRust {
    schema: AargvarkJson<Schemask>,
}

#[derive(Aargvark)]
pub struct CommandGenerateTypescript {
    schema: AargvarkJson<Schemask>,
}

/// Validates data against a schema. Exits with an error code if the data is not
/// valid.
#[derive(Aargvark)]
pub struct CommandValidate {
    data: AargvarkJson<serde_json::Value>,
    /// Override the root binding from the schema to use to validate the data (required
    /// if binding has no default root).
    root: Option<String>,
    schema: AargvarkJson<Schemask>,
}

fn main() {
    match (|| {
        ta_return!((), loga::Error);
        match vark::<Args>().command {
            Command::Validate(c) => {
                schemask::validate(&c.schema.value, c.root, &c.data.value).map_err(|e| loga::err(e.to_string()))?;
                println!("Valid");
            },
            Command::GenerateJava(c) => {
                for file in schemask::generate_java(&c.schema.value, &schemask::gen_java::JavaConfig {
                    package: c.package,
                    runtime_package: c.runtime_package,
                    prelude_package: c.prelude_package,
                }) {
                    let path = c.out.join(&file.path);
                    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| loga::err(e.to_string()))?;
                    std::fs::write(&path, file.source).map_err(|e| loga::err(e.to_string()))?;
                }
            },
            Command::GenerateRust(c) => {
                print!("{}", schemask::generate_rust(&c.schema.value));
            },
            Command::GenerateTypescript(c) => {
                print!("{}", schemask::generate_typescript(&c.schema.value));
            },
            Command::GenerateMarkdown(c) => {
                print!("{}", schemask::generate_markdown(&c.schema.value));
            },
            Command::SchemaskSchema => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&Schemask::schemask()).map_err(|e| loga::err(e.to_string()))?
                );
            },
        }
        return Ok(());
    })() {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}

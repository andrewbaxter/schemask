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

fn check(schema: &Schemask) -> Result<(), loga::Error> {
    return schemask::check_schema(schema).map_err(|e| loga::err(format!("Schema is not well formed:\n{}", e)));
}

#[derive(Aargvark)]
#[vark(break_help)]
pub enum Command {
    /// Generate java code for serializing/deserializing a schemask. Outputs the code
    /// to the specified directory. Requires `jackson` 2.15 or newer.
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
    /// Generate typescript definitions that match a schemask. Outputs the definitions
    /// to stdout.
    ValidateSchema(CommandValidateSchema),
}

#[derive(Aargvark)]
pub struct CommandGenerateJava {
    #[vark(flag = "--out")]
    out: std::path::PathBuf,
    #[vark(flag = "--package")]
    package: String,
    schema: AargvarkJson<Schemask>,
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
    #[vark(flag = "--root")]
    /// Override the root binding from the schema to use to validate the data (required
    /// if binding has no default root).
    root: Option<String>,
    schema: AargvarkJson<Schemask>,
}

#[derive(Aargvark)]
pub struct CommandValidateSchema {
    schema: AargvarkJson<Schemask>,
}

fn main() {
    match (|| {
        ta_return!((), loga::Error);
        match vark::<Args>().command {
            Command::ValidateSchema(c) => {
                check(&c.schema.value)?;
                println!("Ok");
            },
            Command::Validate(c) => {
                check(&c.schema.value)?;
                schemask::validate(&c.schema.value, c.root, &c.data.value).map_err(|e| loga::err(e.to_string()))?;
                println!("Valid");
            },
            Command::GenerateJava(c) => {
                let files =
                    schemask::generate_java(
                        &c.schema.value,
                        &schemask::gen_java::JavaConfig { package: c.package },
                    ).map_err(|e| loga::err(format!("Schema is not well formed:\n{}", e)))?;
                for file in files {
                    let path = c.out.join(&file.path);
                    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| loga::err(e.to_string()))?;
                    std::fs::write(&path, file.source).map_err(|e| loga::err(e.to_string()))?;
                }
            },
            Command::GenerateRust(c) => {
                check(&c.schema.value)?;
                print!("{}", schemask::generate_rust(&c.schema.value));
            },
            Command::GenerateTypescript(c) => {
                check(&c.schema.value)?;
                print!("{}", schemask::generate_typescript(&c.schema.value));
            },
            Command::GenerateMarkdown(c) => {
                check(&c.schema.value)?;
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

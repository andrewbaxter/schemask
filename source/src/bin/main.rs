use {
    aargvark::{
        Aargvark,
        traits_impls::AargvarkJson,
        vark,
    },
    flowcontrol::ta_return,
    loga::fatal,
    schemask::Schemask,
};

#[derive(Aargvark)]
pub struct CommandValidate {
    schema: AargvarkJson<Schemask>,
    data: AargvarkJson<serde_json::Value>,
    root: Option<String>,
}

#[derive(Aargvark)]
pub enum Command {
    Validate(CommandValidate),
}

#[derive(Aargvark)]
pub struct Args {
    command: Command,
}

fn main() {
    match (|| {
        ta_return!((), loga::Error);
        let args = vark::<Args>();
        match args.command {
            Command::Validate(c) => {
                schemask::r#match(&c.schema.value, c.root, &c.data.value)
                    .map_err(|e| loga::err(e.to_string()))?;
                println!("Valid");
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

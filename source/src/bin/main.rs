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
}

#[derive(Aargvark)]
pub enum Command {
    Validate(CommandValidate),
}

pub struct Args {
    command: Command,
}

fn main() {
    match (|| {
        ta_return!((), loga::Error);
        let args = vark::<Args>();
        match args.command {
            Command::Validate(c) => todo!(),
        }
        return Ok(());
    })() {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
    println!("Hello, world!")
}

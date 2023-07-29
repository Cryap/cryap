use std::{path::Path, sync::Arc};

use rune::{
    termcolor::{ColorChoice, StandardStream},
    Any, Context, Diagnostics, FromValue, Source, Sources, Vm,
};

#[derive(Debug, Any)]
pub struct Config {
    pub nixos: String,
}

pub async fn process_config() -> rune::Result<()> {
    let mut context = Context::with_default_modules()?;
    let mut module = rune::Module::default();
    module.ty::<Config>()?;
    context.install(module)?;

    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::from_path(Path::new("config/main.rn"))?);

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        log::error!("Error in your config :(");
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(["main"], ())?;
    let output = Config::from_value(output)?;

    println!("{:?}", output);
    Ok(())
}

use crate::backend::creeper::cli::selector::interactive_mode;
use crate::backend::creeper::launcher::MinecraftLauncher;
use anyhow::Result;
use console::style;

pub async fn run_interactive() -> Result<()> {
    println!("{}", style("Dream Launcher [CLI mode]").bold().green());
    let mut launcher = MinecraftLauncher::new(None).await?;
    interactive_mode(&mut launcher).await
}

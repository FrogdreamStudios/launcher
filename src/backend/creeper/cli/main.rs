use crate::backend::creeper::cli::selector::interactive_mode;
use crate::backend::creeper::launcher::MinecraftLauncher;
use anyhow::Result;

pub async fn run_interactive() -> Result<()> {
    println!("Dream Launcher [CLI mode]");
    let mut launcher = MinecraftLauncher::new(None).await?;
    interactive_mode(&mut launcher).await
}

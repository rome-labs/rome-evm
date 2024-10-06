use crate::params::InitParams;
use crate::init::Intializer;
use anyhow::Context;

mod init;
mod params;

fn main() -> anyhow::Result<()> {
    let params = InitParams::from_env().context("Failed to get init params from env")?;
    let initializer = Intializer::new(params).context("Failed to create initializer")?;

    initializer.init().context("Failed to mint native tokens")?;

    Ok(())
}

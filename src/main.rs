pub mod inputs;
pub mod messages;
pub mod player;

pub mod gen {
    // this will include the direct contents of our generated file
    include!(concat!(env!("OUT_DIR"), "/gen.rs"));
}

fn main() -> anyhow::Result<()> {
    Ok(())
}

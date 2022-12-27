use anyhow::Result;

pub trait Executable {
    fn exec(&self) -> Result<()>;
}

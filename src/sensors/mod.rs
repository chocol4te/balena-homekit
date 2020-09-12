use anyhow::Result;

mod aht20;
mod sgp30;

pub trait Sensor {
    fn initialize() -> Result<Self>
    where
        Self: Sized;

    fn measure(&mut self) -> Result<Vec<Measurement>>;
}

#[derive(Debug)]
pub struct Measurement {
    pub name: String,
    pub value: f32,
}

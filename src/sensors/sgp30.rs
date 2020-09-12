use {
    crate::sensors::{Measurement, Sensor},
    anyhow::{anyhow, Result},
    linux_embedded_hal::{Delay, I2cdev},
    sgp30::Sgp30,
};

const I2C_DEV: &str = "/dev/i2c-1";

impl Sensor for Sgp30<I2cdev, Delay> {
    fn initialize() -> Result<Self> {
        let dev = I2cdev::new(I2C_DEV)?;
        let address = 0x58;

        let mut sgp = Sgp30::new(dev, address, Delay);

        sgp.init()
            .or_else(|e| Err(anyhow!("Failed to initialize SGP30: {:?}", e)))?;

        Ok(sgp)
    }

    fn measure(&mut self) -> Result<Vec<Measurement>> {
        let m = self
            .measure()
            .or_else(|e| Err(anyhow!("Failed to read from SGP30: {:?}", e)))?;

        Ok(vec![
            Measurement {
                name: "co2".to_string(),
                value: m.co2eq_ppm as f32,
            },
            Measurement {
                name: "voc".to_string(),
                value: m.tvoc_ppb as f32,
            },
        ])
    }
}

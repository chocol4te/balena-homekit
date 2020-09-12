use {
    crate::sensors::{Measurement, Sensor},
    aht20::Aht20,
    anyhow::{anyhow, Result},
    linux_embedded_hal::{Delay, I2cdev},
};

const I2C_DEV: &str = "/dev/i2c-1";

impl Sensor for Aht20<I2cdev, Delay> {
    fn initialize() -> Result<Self> {
        let dev = I2cdev::new(I2C_DEV)?;

        Aht20::new(dev, Delay).or_else(|e| Err(anyhow!("Failed to initialize AHT20: {:?}", e)))
    }

    fn measure(&mut self) -> Result<Vec<Measurement>> {
        let (h, t) = self
            .read()
            .or_else(|e| Err(anyhow!("Failed to read from AHT20: {:?}", e)))?;
        let humidity = h.rh();
        let temperature = t.celsius();

        Ok(vec![
            Measurement {
                name: "humidity".to_string(),
                value: humidity,
            },
            Measurement {
                name: "temperature".to_string(),
                value: temperature,
            },
        ])
    }
}

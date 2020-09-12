use {
    crate::sensors::{Measurement, Sensor},
    aht20::Aht20,
    anyhow::Result,
    async_channel::Sender,
    chrono::Utc,
    influxdb::{Client as DbClient, WriteQuery},
    lazy_static::lazy_static,
    log::{info, warn},
    rumqttc::{self, EventLoop, MqttOptions, Publish, QoS, Request},
    sgp30::Sgp30,
    std::{env, time::Duration},
    tokio::{sync::Mutex, task, time::interval},
};

mod sensors;

const DB_NAME: &str = "environment";
const MQTT_INTERVAL: Duration = Duration::from_secs(5);
const DB_INTERVAL: Duration = Duration::from_secs(60);

lazy_static! {
    static ref SENSORS: Mutex<Vec<Box<dyn Sensor + Send + Sync>>> = Mutex::new(vec![]);
}

pub async fn client() -> Result<()> {
    pretty_env_logger::init();
    color_backtrace::install();

    let id = env::var("BALENA_DEVICE_UUID")
        .expect("Failed to find Balena device UUID environment variable");
    let mqtt_address =
        env::var("MQTT_ADDR").expect("Failed to find MQTT_ADDR environment variable");
    let mqtt_port = env::var("MQTT_PORT").expect("Failed to find MQTT_PORT environment variable");
    let db_address = env::var("DB_ADDR").expect("Failed to find MQTT_ADDR environment variable");
    let db_port = env::var("DB_PORT").expect("Failed to find MQTT_PORT environment variable");

    info!("MQTT connecting to {}:{}", mqtt_address, mqtt_port);
    let mut eventloop = {
        let mut mqttoptions = MqttOptions::new(&id, mqtt_address, mqtt_port.parse()?);
        mqttoptions.set_keep_alive(5);

        EventLoop::new(mqttoptions, 10)
    };

    info!("INFLUXDB connecting to {}:{}", db_address, db_port);
    let db_client = DbClient::new(format!("http://{}:{}", db_address, db_port), DB_NAME);

    info!("AHT20 initializing");
    match Aht20::initialize() {
        Ok(sensor) => SENSORS.lock().await.push(Box::new(sensor)),
        Err(e) => warn!("{}", e),
    }
    info!("SGP30 initializing");
    match Sgp30::initialize() {
        Ok(sensor) => SENSORS.lock().await.push(Box::new(sensor)),
        Err(e) => warn!("{}", e),
    }

    task::spawn(mqtt_task(id.clone(), eventloop.handle()));
    task::spawn(db_task(id.clone(), db_client));

    info!("Initialization complete");

    loop {
        eventloop.poll().await?;
    }
}

async fn mqtt_task(id: String, sender: Sender<Request>) {
    let mut interval = interval(MQTT_INTERVAL);
    loop {
        interval.tick().await;

        let reqs: Vec<Request> = SENSORS
            .lock()
            .await
            .iter_mut()
            .map(|sensor| sensor.measure().unwrap())
            .map(|xs| xs.into_iter())
            .flatten()
            .map(|m| {
                Request::Publish(Publish::new(
                    format!("{}/{}", id, m.name),
                    QoS::AtLeastOnce,
                    format!("{}", m.value),
                ))
            })
            .collect();

        for req in reqs {
            sender.send(req).await.unwrap();
        }
    }
}

async fn db_task(id: String, db: DbClient) {
    let mut interval = interval(DB_INTERVAL);
    loop {
        interval.tick().await;

        let mut query = WriteQuery::new(Utc::now().into(), DB_NAME).add_tag("id", id.clone());

        let measurements: Vec<Measurement> = SENSORS
            .lock()
            .await
            .iter_mut()
            .map(|x| x.measure().unwrap())
            .map(|xs| xs.into_iter())
            .flatten()
            .collect();

        for m in measurements {
            query = query.add_field(m.name, m.value);
        }

        db.query(&query).await.unwrap();
    }
}
/*
async fn run_sgp30(
    id: String,
    sender: Sender<Request>,
    db: DbClient,
    mut sgp30: Sgp30<I2cdev, Delay>,
) {
    let mut interval = interval(INTERVAL);
    loop {
        interval.tick().await;

        let measurement = sgp30.measure().unwrap();
        let co2 = measurement.co2eq_ppm;
        let voc = measurement.tvoc_ppb;

        let co2_value = if (0..400).contains(&co2) {
            1
        } else if (400..1000).contains(&co2) {
            2
        } else if (1000..2000).contains(&co2) {
            3
        } else if (2000..5000).contains(&co2) {
            4
        } else {
            5
        };

        let voc_value = if (0..25).contains(&voc) {
            1
        } else if (25..50).contains(&voc) {
            2
        } else if (50..325).contains(&voc) {
            3
        } else if (325..500).contains(&voc) {
            4
        } else {
            5
        };

        // Online
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/aq/online", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();

        // Active
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/aq/active", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();

        // Air Quality
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/aq", id),
                QoS::AtLeastOnce,
                format!("{}", cmp::min(voc_value, co2_value)),
            )))
            .await
            .unwrap();

        // VOC
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/aq/voc", id),
                QoS::AtLeastOnce,
                format!("{}", measurement.tvoc_ppb),
            )))
            .await
            .unwrap();

        // CO2
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/aq/co2", id),
                QoS::AtLeastOnce,
                format!("{}", measurement.co2eq_ppm),
            )))
            .await
            .unwrap();
    }
}

async fn run_aht20(
    id: String,
    sender: Sender<Request>,
    db: DbClient,
    mut aht20: Aht20<I2cdev, Delay>,
) {
    let mut interval = interval(INTERVAL);
    loop {
        interval.tick().await;

        let (h, t) = aht20.read().unwrap();
        let humidity = h.rh();
        let temperature = t.celsius();

        // Online
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/humidity/online", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/temperature/online", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();

        // Active
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/humidity/active", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/temperature/active", id),
                QoS::AtLeastOnce,
                b"true".to_vec(),
            )))
            .await
            .unwrap();

        // Humidity
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/humidity", id),
                QoS::AtLeastOnce,
                format!("{}", humidity),
            )))
            .await
            .unwrap();

        // Temperature
        sender
            .send(Request::Publish(Publish::new(
                format!("{}/temperature", id),
                QoS::AtLeastOnce,
                format!("{}", temperature),
            )))
            .await
            .unwrap();

        // DB
        let reading = AHT20Reading {
            time: Utc::now(),
            humidity,
            temperature,
            id: id.clone(),
        };
        db.query(&reading.into_query("environment")).await.unwrap();
    }
}
*/

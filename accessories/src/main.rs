use {
    anyhow::Result,
    async_channel::Sender,
    rumqttc::{self, EventLoop, MqttOptions, Publish, QoS, Request, Subscribe},
    std::{env, time::Duration},
    tokio::{task, time},
};

#[tokio::main(core_threads = 1)]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    color_backtrace::install();

    let id = env::var("BALENA_DEVICE_UUID")
        .expect("Failed to find Balena device UUID environment variable");
    let address = env::var("MQTT_ADDR").expect("Failed to find MQTT_ADDR environment variable");
    let port = env::var("MQTT_PORT").expect("Failed to find MQTT_PORT environment variable");

    let mut mqttoptions = MqttOptions::new(id, address, port.parse()?);
    mqttoptions.set_keep_alive(5);

    let mut eventloop = EventLoop::new(mqttoptions, 10).await;
    let requests_tx = eventloop.handle();
    task::spawn(async move {
        requests(requests_tx).await;
        time::delay_for(Duration::from_secs(3)).await;
    });

    loop {
        let (incoming, outgoing) = eventloop.poll().await?;
        println!("Incoming = {:?}, Outgoing = {:?}", incoming, outgoing);
    }
}

async fn requests(requests_tx: Sender<Request>) {
    let subscription = Subscribe::new("hello/world", QoS::AtMostOnce);
    let _ = requests_tx.send(Request::Subscribe(subscription)).await;

    for i in 0..10 {
        let publish = Publish::new("hello/world", QoS::AtLeastOnce, vec![i; i as usize]);
        requests_tx.send(Request::Publish(publish)).await.unwrap();
        time::delay_for(Duration::from_secs(1)).await;
    }

    time::delay_for(Duration::from_secs(120)).await;
}

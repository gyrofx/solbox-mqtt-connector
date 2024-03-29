mod mqtt;
mod opts;
mod sorel;
mod utils;

use log::{debug, error, info};
use std::{panic, process, time::Duration};

use async_std::task;
use envconfig::Envconfig;
use opts::Opts;

use mqtt::{Mqtt, SolboxMessage};
use sorel::Sorel;

#[async_std::main]
async fn main() {
    env_logger::init();

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        orig_hook(panic_info);
        process::exit(1);
    }));

    let config = Opts::init_from_env().unwrap();

    info!("Starting solbox mqtt exporter");

    let mut sorel = Sorel::new(
        config.sorel_username,
        config.sorel_password,
        config.device_id,
        config.sorel_session_id,
    );

    let (mqtt, mut eventloop) = Mqtt::new(
        &config.mqtt_borker_host,
        config.mqtt_borker_port,
        &config.mqtt_topic,
    );

    match sorel.login_to_sorel().await {
        Ok(_session) => {
            debug!("Logged in successfully");
        }
        Err(e) => panic!("Error: failed to login: {}", e),
    }

    task::spawn(async move {
        loop {
            let sensor1 = sorel
                .fetch_sensor_value(String::from("sensor1"))
                .await
                .unwrap();
            let sensor2 = sorel
                .fetch_sensor_value(String::from("sensor2"))
                .await
                .unwrap();
            let sensor3 = sorel
                .fetch_sensor_value(String::from("sensor3"))
                .await
                .unwrap();
            let relay1 = sorel
                .fetch_relay_value(String::from("relay1"))
                .await
                .unwrap();

            let message = SolboxMessage::new(sensor1, sensor2, sensor3, relay1);
            match mqtt.publish_solbox_message(&message).await {
                Ok(_) => info!("Published to {:?}", message),
                Err(e) => error!("Error: failed to publish message: {}", e),
            }

            task::sleep(Duration::from_secs(
                config.measurement_interval_in_seconds.into(),
            ))
            .await;
        }
    });

    loop {
        break match eventloop.poll().await {
            Ok(_) => {
                continue;
            }
            Err(e) => {
                error!("Error: failed to poll eventloop: {}", e)
            }
        };
    }
}

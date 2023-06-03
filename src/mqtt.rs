use std::time::Duration;

use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;

pub struct Mqtt {
    client: AsyncClient,
    topic: String,
}

impl Mqtt {
    pub fn new(host: &String, port: u16, topic: &String) -> (Mqtt, rumqttc::EventLoop) {
        let mut mqttoptions = MqttOptions::new("rumqtt-async", host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

        (
            Mqtt {
                client,
                topic: topic.to_string(),
            },
            eventloop,
        )
    }

    pub async fn publish_solbox_message(
        &self,
        payload: &SolboxMessage,
    ) -> Result<(), rumqttc::ClientError> {
        self.client
            .publish(&self.topic, QoS::ExactlyOnce, false, payload.to_bytes())
            .await
    }
}

#[derive(Debug)]
pub struct SolboxMessage {
    sensor1: i16,
    sensor2: i16,
    sensor3: i16,
    relay1: i16,
}

impl SolboxMessage {
    pub fn new(sensor1: i16, sensor2: i16, sensor3: i16, relay1: i16) -> SolboxMessage {
        SolboxMessage {
            sensor1,
            sensor2,
            sensor3,
            relay1,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes = json!({
          "temperature-boiler-top": self.sensor3,
          "temperature-boiler-bottom": self.sensor2,
          "temperature-collector": self.sensor1,
          "pump": self.relay1,
          "time": chrono::offset::Utc::now().to_rfc3339()
        })
        .to_string()
        .into_bytes();
        bytes
    }
}

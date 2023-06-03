use std::collections::HashMap;

use log::{debug, error, info};
use reqwest::{header::COOKIE, Client};
use serde::{Deserialize, Serialize};

const SOREL_DOMAIN: &str = "sorel-connect.net";

pub struct Sorel {
    username: String,
    password: String,
    device_id: String,
    session_id: String,
    client: Client,
}

impl Sorel {
    pub fn new(username: String, password: String, device_id: String) -> Sorel {
        Sorel {
            username,
            password,
            device_id,
            session_id: String::from(""),
            client: Client::new(),
        }
    }

    pub async fn login_to_sorel(&mut self) -> Result<String, String> {
        info!("Login to Sorel");
        let client = Client::new();
        let response = client.post(self.login_url()).send().await.unwrap();

        if response.status() != 200 {
            error!("Login failed");
            return Err(String::from("Login failed"));
        }

        for cookie in response.cookies() {
            if cookie.name() == "nabto-session" {
                self.session_id = cookie.value().to_string();
                debug!("Session id: {}", self.session_id);
                return Ok(cookie.value().to_string());
            }
        }

        return Err(String::from("No session found"));
    }

    pub async fn fetch_sensor_value(&self, sensor_id: String) -> Result<i16, &'static str> {
        let response = self.fetch_value(sensor_id).await;
        return celsius_value_getter(response.response.get("val").unwrap());
    }

    pub async fn fetch_relay_value(&self, sensor_id: String) -> Result<i16, &'static str> {
        let response = self.fetch_value(sensor_id).await;
        return relay_value_getter(response.response.get("val").unwrap());
    }

    async fn fetch_value(&self, sensor_id: String) -> SensorResponse {
        let sensor_url = format!("{}/state.json?id={}", self.base_url(), sensor_id);
        let cookie_header = format!("nabto-session={}", self.session_id);
        let response = self
            .client
            .get(sensor_url)
            .header(COOKIE, cookie_header)
            .send()
            .await
            .unwrap();

        let body = response.text().await.unwrap();
        match serde_json::from_str::<SensorResponse>(&body) {
            Ok(parsed_response) => parsed_response,
            Err(e) => {
                panic!("Failed to parse response: {} {}", e, body);
            }
        }
    }

    fn login_url(&self) -> String {
        let login_url = format!(
            "{}/nabto/hosted_plugin/login/execute?email={}&password={}",
            self.base_url(),
            self.username,
            self.password
        );
        login_url
    }

    fn base_url(&self) -> String {
        let sorel_base_url = format!("https://{}.{}", self.device_id, SOREL_DOMAIN);
        sorel_base_url
    }
}

fn relay_value_getter(value: &str) -> Result<i16, &'static str> {
    match value.split("_").last().unwrap() {
        "ON" => return Ok(100),
        "OFF" => return Ok(0),
        _ => return Err("Failed to parse value to a boolean"),
    }
}

fn celsius_value_getter(value: &str) -> Result<i16, &'static str> {
    match value.replace("°C", "").parse::<i16>() {
        Ok(v) => return Ok(v),
        _ => return Err("Failed to parse value to a i16"),
    }
}

// fn redacted_sorel_url(url: &String) -> String {
//   /(test_ref=)[^\&]+/
//     let url = url.replace("password=", "password=REDACTED");
//     url
// }

#[derive(Serialize, Deserialize, Debug)]
struct SensorResponse {
    request: HashMap<String, String>,
    response: HashMap<String, String>,
}

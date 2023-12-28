use std::{collections::HashMap, fs::OpenOptions, io::Write, path};

use log::{error, info};
use reqwest::{header::COOKIE, Client};
use serde::{Deserialize, Serialize};

use crate::utils::shorten_string;

const SOREL_DOMAIN: &str = "sorel-connect.net";
const SOREL_SESSION_FILE: &str = "/tmp/solbox-mqtt-expoter/session_id";

pub struct Sorel {
    username: String,
    password: String,
    device_id: String,
    session_id: String,
    client: Client,
}

impl Sorel {
    pub fn new(
        username: String,
        password: String,
        device_id: String,
        session_id_override: String,
    ) -> Sorel {
        Sorel {
            username,
            password,
            device_id,
            session_id: session_id_override,
            client: Client::new(),
        }
    }

    pub async fn login_to_sorel(&mut self) -> Result<(), String> {
        info!("Login to Sorel");
        if self.session_id != "" {
            info!("Already logged in. Used overridden session ID. ");
            return Ok(());
        }

        match read_session() {
            Ok(session_id) => {
                info!("Found preserved session. {}...", &session_id[..10]);
                self.session_id = session_id;
                return Ok(());
            }
            Err(_error) => {
                info!("No preserved session found. Logging in to Sorel");
            }
        };

        let client = Client::new();
        let response = client.post(self.login_url()).send().await.unwrap();

        if response.status() != 200 {
            error!("Failed to sign in to failed: status {}", response.status());
            invalidate_session();
            return Err(String::from("Login failed"));
        }

        for cookie in response.cookies() {
            if cookie.name() == "nabto-session" {
                self.session_id = cookie.value().to_string();
                write_session(&self.session_id);
                info!("Successfully signed in to Sorel");
                return Ok(());
            }
        }

        return Err(String::from(
            "Failed to sign in to failed: No session found",
        ));
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
        let response_result = self
            .client
            .get(sensor_url)
            .header(COOKIE, cookie_header)
            .send()
            .await;

        let response = match response_result.is_err() {
            true => {
                invalidate_session();
                panic!("Failed to fetch value: {}", response_result.err().unwrap());
            }
            false => {
                let r = response_result.unwrap();
                r
            }
        };

        let body = response.text().await.unwrap();
        match serde_json::from_str::<SensorResponse>(&body) {
            Ok(parsed_response) => parsed_response,
            Err(e) => {
                if body.to_lowercase().trim().starts_with("<!doctype") {
                    invalidate_session();
                }
                panic!(
                    "Failed to parse response: {} {}",
                    e,
                    shorten_string(&body.to_lowercase().trim(), 1000),
                );
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

fn write_session(session_id: &str) {
    std::fs::create_dir_all(path::Path::new("/tmp/solbox-mqtt-expoter")).unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(SOREL_SESSION_FILE)
        .unwrap();
    file.write_all(&session_id.as_bytes()).unwrap();
}

fn read_session() -> Result<String, &'static str> {
    let session = read_session_from_file();
    match session != "" {
        true => Ok(session),
        false => Err("No session found"),
    }
}

fn read_session_from_file() -> String {
    match std::fs::read_to_string(SOREL_SESSION_FILE) {
        Ok(session_id) => session_id,
        Err(_) => "".to_string(),
    }
}

fn invalidate_session() {
    info!("Invalidating session");
    match std::path::Path::new(SOREL_SESSION_FILE).exists() {
        true => std::fs::remove_file(SOREL_SESSION_FILE).unwrap(),
        false => return,
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SensorResponse {
    request: HashMap<String, String>,
    response: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorResponse {
    error: HashMap<String, String>,
}

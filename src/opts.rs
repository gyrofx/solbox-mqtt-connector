use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Opts {
    #[envconfig(from = "SOLBOX_MEASUREMENT_INTERVAL_IN_SECONDS", default = "120")]
    pub measurement_interval_in_seconds: u16,

    #[envconfig(from = "SOLBOX_SOREL_DEVICE_ID")]
    pub device_id: String,

    #[envconfig(from = "SOLBOX_SOREL_USERNAME")]
    pub sorel_username: String,

    #[envconfig(from = "SOLBOX_SOREL_PASSWORD")]
    pub sorel_password: String,

    #[envconfig(from = "SOLBOX_SOREL_OVERRIDE_SESSION_ID", default = "")]
    pub sorel_session_id: String,

    #[envconfig(from = "SOLBOX_MQTT_BROKER_HOST")]
    pub mqtt_borker_host: String,

    #[envconfig(from = "SOLBOX_MQTT_BROKER_PORT")]
    pub mqtt_borker_port: u16,

    #[envconfig(from = "SOLBOX_MQTT_TOPIC")]
    pub mqtt_topic: String,
}

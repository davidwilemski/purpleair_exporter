#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prometheus;

use std::env;
use std::net::SocketAddr;

use axum::prelude::*;
use http::status::StatusCode;
use log::{debug, error, info, warn};
use prometheus::{Encoder, GaugeVec, IntGaugeVec, TextEncoder};
use serde::{self, Deserialize};

lazy_static! {
    static ref LAST_SEEN_TIMESTAMP: IntGaugeVec = register_int_gauge_vec!(
        "purpleair_lastseen_timestamp",
        "UTC timestamp for sensor last seen time",
        &["id", "sensor_label"]
    )
    .unwrap();
    static ref UPTIME: IntGaugeVec = register_int_gauge_vec!(
        "purpleair_uptime_seconds",
        "Sensor uptime in seconds",
        &["id", "sensor_label"]
    )
    .unwrap();
    static ref INFO: IntGaugeVec = register_int_gauge_vec!(
        "purpleair_info",
        "Sensor info",
        &["id", "sensor_label", "lat", "lon"]
    )
    .unwrap();
    static ref PM2_5_VALUE: GaugeVec = register_gauge_vec!(
        "purpleair_pm2_5_value",
        "Sensor-reported PM2.5 value particulate mass in ug/m3",
        &["id", "sensor_label"]
    )
    .unwrap();
    static ref PARTICULATE_MASS: GaugeVec = register_gauge_vec!(
        "purpleair_",
        "Sensor reported raw value particulate mass in ug/m3",
        &["id", "sensor_label", "microns", "cf"]
    )
    .unwrap();
    static ref TEMP: GaugeVec = register_gauge_vec!(
        "purpleair_temperature_fahrenheit",
        "Sensor reported temperature in Fahrenheit",
        &["id", "sensor_label"]
    )
    .unwrap();
    static ref HUMIDITY: GaugeVec = register_gauge_vec!(
        "purpleair_humidity",
        "Sensor reported humidity (in percent)",
        &["id", "sensor_label"]
    )
    .unwrap();
    static ref PRESSURE: GaugeVec = register_gauge_vec!(
        "purpleair_pressure",
        "Sensor reported pressure",
        &["id", "sensor_label"]
    )
    .unwrap();
}

#[derive(Deserialize, Debug)]
struct SensorInfo {
    #[serde(rename = "ID")]
    id: i64,

    #[serde(rename = "Label")]
    label: String,

    #[serde(rename = "Lat")]
    lat: f64,

    #[serde(rename = "Lon")]
    lon: f64,

    #[serde(rename = "PM2_5Value")]
    pm_2_5_value: String,

    #[serde(rename = "Uptime")]
    uptime: Option<String>,

    #[serde(rename = "LastSeen")]
    last_seen: i64,

    // particles/deciliter?
    p_0_3_um: String,
    p_0_5_um: String,
    p_1_0_um: String,
    p_2_5_um: String,
    p_5_0_um: String,
    p_10_0_um: String,

    pm1_0_cf_1: String,
    pm2_5_cf_1: String,
    pm10_0_cf_1: String,

    pm1_0_atm: String,
    pm2_5_atm: String,
    pm10_0_atm: String,

    temp_f: Option<String>,
    humidity: Option<String>,
    pressure: Option<String>,
}

impl SensorInfo {
    fn id_string(&self) -> String {
        format!("{}", self.id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let app = route("/metrics", get(metrics));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    hyper::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn scrape_purple_air(sensor_ids: &str) -> Result<(), Box<dyn std::error::Error>> {
    let purple_air_resp: serde_json::Value = reqwest::get(format!(
        "https://www.purpleair.com/json?show={}",
        sensor_ids.replace(',', "|")
    ))
    .await?
    .json()
    .await?;
    debug!("resp = {:?}", purple_air_resp);

    match purple_air_resp.get("results") {
        Some(results) => {
            if let Some(results_arr) = results.as_array() {
                for result in results_arr {
                    let sensor_info: SensorInfo = serde_json::from_value(result.clone())?;
                    let common_labels: &[&str] = &[&sensor_info.id_string(), &sensor_info.label];
                    if let Some(ref uptime) = sensor_info.uptime {
                        UPTIME
                            .with_label_values(&common_labels)
                            .set(uptime.parse::<i64>()?);
                    }
                    LAST_SEEN_TIMESTAMP
                        .with_label_values(common_labels)
                        .set(sensor_info.last_seen);
                    INFO.with_label_values(&[
                        &sensor_info.id_string(),
                        &sensor_info.label,
                        format!("{}", sensor_info.lat).as_str(),
                        format!("{}", sensor_info.lon).as_str(),
                    ])
                    .set(1);

                    PM2_5_VALUE
                        .with_label_values(common_labels)
                        .set(sensor_info.pm_2_5_value.parse::<f64>()?);

                    if let Some(temp_f) = sensor_info.temp_f {
                        TEMP.with_label_values(common_labels)
                            .set(temp_f.parse::<f64>()?);
                    }
                    if let Some(humidity) = sensor_info.humidity {
                        HUMIDITY
                            .with_label_values(common_labels)
                            .set(humidity.parse::<f64>()?);
                    }
                    if let Some(pressure) = sensor_info.pressure {
                        PRESSURE
                            .with_label_values(common_labels)
                            .set(pressure.parse::<f64>()?);
                    }
                }
            }
        }
        None => warn!("results array not found!"),
    };
    Ok(())
}

async fn metrics() -> Result<String, StatusCode> {
    info!("Handling metrics call");
    let sensor_ids = env::var("PURPLEAIR_SENSOR_IDS").map_err(log_error)?;
    scrape_purple_air(&sensor_ids)
        .await
        .map_err(log_box_error)?;
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .map_err(log_error)?;
    let prom_metrics = String::from_utf8(buffer).map_err(log_error)?;
    Ok(prom_metrics)
}

fn log_box_error(err: Box<dyn std::error::Error>) -> StatusCode {
    error!("{:?}", err);
    StatusCode::INTERNAL_SERVER_ERROR
}

fn log_error<E>(err: E) -> StatusCode
where
    E: std::error::Error,
{
    error!("{:?}", err);
    StatusCode::INTERNAL_SERVER_ERROR
}

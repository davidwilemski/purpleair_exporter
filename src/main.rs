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
    static ref PM2_5_AQI: IntGaugeVec = register_int_gauge_vec!(
        "purpleair_pm2_5_aqi_estimate",
        "Estimated instantaneous PM2.5 AQI value based on interpretation of AQI ranges given by the EPA",
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
                    PM2_5_AQI
                        .with_label_values(common_labels)
                        .set(pm2_5_aqi_estimate(sensor_info.pm_2_5_value.parse::<f64>()?) as i64);

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

fn pm2_5_aqi_estimate(pm_2_5_value: f64) -> i32 {
    // From: https://aqs.epa.gov/aqsweb/documents/codetables/aqi_breakpoints.html as of August 09,
    // 2021
    let breakpoints = [12f64, 35.4, 55.5, 150.5, 250.5, 350.5, 500.5, 99999.9];
    // Simple linear interpolation within AQI ranges
    // ((high aqi - low aqi) / (high concentration - low concentration)) * concentration value
    match pm_2_5_value {
        v if v <= breakpoints[0] => (((50f64 - 0f64) / (breakpoints[0] - 0f64)) * v).round() as i32,
        v if v <= breakpoints[1] => ((((100f64 - 51f64) / (breakpoints[1] - breakpoints[0]))
            * (v - breakpoints[0]))
            + 51f64)
            .round() as i32,
        v if v < breakpoints[2] => {
            ((((150f64 - 101f64) / (breakpoints[2] - breakpoints[1])) * (v - breakpoints[1])) + 101f64)
                .round() as i32
        }
        v if v < breakpoints[3] => {
            ((((200f64 - 151f64) / (breakpoints[3] - breakpoints[2])) * (v - breakpoints[2])) + 151f64)
                .round() as i32
        }
        v if v < breakpoints[4] => {
            ((((300f64 - 201f64) / (breakpoints[4] - breakpoints[3])) * (v - breakpoints[3])) + 201f64)
                .round() as i32
        }
        v if v < breakpoints[5] => {
            ((((400f64 - 301f64) / (breakpoints[5] - breakpoints[4])) * (v - breakpoints[4])) + 301f64)
                .round() as i32
        }
        v if v < breakpoints[6] => {
            ((((500f64 - 401f64) / (breakpoints[6] - breakpoints[5])) * (v - breakpoints[5])) + 401f64)
                .round() as i32
        }
        v if v < breakpoints[7] => {
            warn!("value {} exceeds concentration limit", v);
            ((((999f64 - 501f64) / (breakpoints[7] - breakpoints[6])) * (v - breakpoints[6])) + 501f64)
                .round() as i32
        }
        v => {
            warn!("value {} exceeds concentration limit", v);
            501
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pm_2_5_aqi_estimate() {
        assert_eq!(pm2_5_aqi_estimate(9.5f64), 40);
        assert_eq!(pm2_5_aqi_estimate(11f64), 46);
        assert_eq!(pm2_5_aqi_estimate(12f64), 50);
        assert_eq!(pm2_5_aqi_estimate(12.1f64), 51);
        assert_eq!(pm2_5_aqi_estimate(15f64), 57);
        assert_eq!(pm2_5_aqi_estimate(30f64), 89);
        assert_eq!(pm2_5_aqi_estimate(35.4f64), 100);
        assert_eq!(pm2_5_aqi_estimate(35.5f64), 101);
        assert_eq!(pm2_5_aqi_estimate(50f64), 137);
        assert_eq!(pm2_5_aqi_estimate(55.4f64), 150);
        assert_eq!(pm2_5_aqi_estimate(55.5f64), 151);
        assert_eq!(pm2_5_aqi_estimate(100f64), 174);
        assert_eq!(pm2_5_aqi_estimate(150.4f64), 200);
        assert_eq!(pm2_5_aqi_estimate(150.5f64), 201);
        assert_eq!(pm2_5_aqi_estimate(175f64), 225);
        assert_eq!(pm2_5_aqi_estimate(200f64), 250);
        assert_eq!(pm2_5_aqi_estimate(201f64), 251);
        assert_eq!(pm2_5_aqi_estimate(250f64), 300);
        assert_eq!(pm2_5_aqi_estimate(251f64), 301);
        assert_eq!(pm2_5_aqi_estimate(300f64), 350);
        assert_eq!(pm2_5_aqi_estimate(350f64), 400);
        assert_eq!(pm2_5_aqi_estimate(351f64), 401);
        assert_eq!(pm2_5_aqi_estimate(400f64), 434);
        assert_eq!(pm2_5_aqi_estimate(500f64), 500);
        // These are above the scale
        assert_eq!(pm2_5_aqi_estimate(501f64), 501);
        assert_eq!(pm2_5_aqi_estimate(550f64), 501);
        assert_eq!(pm2_5_aqi_estimate(900f64), 503);
    }
}

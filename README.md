# purpleair-exporter

Exports data from https://purpleair.com into a Prometheus-compatible format

## Configuring

As of now, there is a single configuration option and it is required to be set:

The `PURPLEAIR_SENSOR_IDS` environment variable controls what sensor to request data from. This is intended to support a comma-separated or pipe-separate list of sensor IDs. For example, `PURPLEAIR_SENSOR_IDS="123|456"`, `PURPLEAIR_SENSOR_IDS="123,456"`, and `PURPLEAIR_SENSOR_IDS=123` all work.

## Running

Assuming the appropriate environment variables have been configured, executing the `purpleair_exporter` binary is sufficient to start the exporter. It listens on port 3000.

There is a Docker container published to `dtw0/purpleair-exporter` built using the Dockerfile in this repository if you want to run this server in a container (or feel free to re-use the Dockerfile to build your own image). The same configuration options described above apply to the container image.

## TODO

- Make ip/port binding configurable (hardcoded to 0.0.0.0:3000 for now)
- Export further data from sensors: we have other data available in the `SensorInfo` struct but it is not currently exported to metrics
- Export a computed AQI score? Requires converting the raw particle values to the AQI value.


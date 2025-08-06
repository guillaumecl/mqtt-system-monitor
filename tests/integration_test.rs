use mqtt_system_monitor::configuration;
use mqtt_system_monitor::daemon::Daemon;
use mqtt_system_monitor::home_assistant::Sensor;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use sysinfo::{Components, Networks};

#[test]
fn test_empty_values() -> Result<(), Box<dyn Error>> {
    let conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    let interface = conf.sensors.network.clone();
    let temp_sensor = conf.sensors.temperature.clone();

    let mut daemon = Daemon::new(conf);

    let status = daemon.update_data();

    println!("First read:");
    println!(
        "We have read net_tx={:?}, net_rx={:?} on interface {:?}",
        status.net_tx, status.net_rx, interface
    );
    println!(
        "We have read temp={:?} on component {:?}",
        status.cpu_temp, temp_sensor
    );

    assert_eq!(status.net_rx, None);
    assert_eq!(status.net_tx, None);
    assert_eq!(status.cpu_temp, None);

    let status = daemon.update_data();

    println!("Second read:");
    println!(
        "We have read net_tx={:?}, net_rx={:?} on interface {:?}",
        status.net_tx, status.net_rx, interface
    );
    println!(
        "We have read temp={:?} on component {:?}",
        status.cpu_temp, temp_sensor
    );

    assert_eq!(status.net_rx, None);
    assert_eq!(status.net_tx, None);
    assert_eq!(status.cpu_temp, None);

    daemon.register_sensors();

    let descriptor = daemon.registration_descriptor();

    assert!(descriptor.has_sensor(Sensor::CpuUsage));
    assert_eq!(descriptor.has_sensor(Sensor::CpuTemperature), false);
    assert_eq!(descriptor.has_sensor(Sensor::NetTx), false);
    assert_eq!(descriptor.has_sensor(Sensor::NetRx), false);

    Ok(())
}

#[test]
fn test_selection() -> Result<(), Box<dyn Error>> {
    let network = Networks::new_with_refreshed_list();
    let components = Components::new_with_refreshed_list();
    let mut conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    conf.sensors.network = Some(
        network
            .iter()
            .next()
            .expect("Should have at least a network to test")
            .0
            .clone(),
    );
    conf.sensors.temperature = components
        .iter()
        .next()
        .map(|c| c.label())
        .map(|l| l.to_string());

    let interface = conf.sensors.network.clone();
    let temp_sensor = conf.sensors.temperature.clone();

    let mut daemon = Daemon::new(conf);

    daemon.register_sensors();

    let status = daemon.update_data();

    println!("First read:");
    println!(
        "We have read net_tx={:?}, net_rx={:?} on interface {:?}",
        status.net_tx, status.net_rx, interface
    );
    println!(
        "We have read temp={:?} on component {:?}",
        status.cpu_temp, temp_sensor
    );

    // The first call, the transfer rate is always at 0. It can be non-zero after some time
    assert_eq!(status.net_rx, Some(0.0));
    assert_eq!(status.net_tx, Some(0.0));
    if temp_sensor.is_some() {
        assert_ne!(status.cpu_temp, None);
    }

    let status = daemon.update_data();

    println!("Second read:");
    println!(
        "We have read net_tx={:?}, net_rx={:?} on interface {:?}",
        status.net_tx, status.net_rx, interface
    );
    println!(
        "We have read temp={:?} on component {:?}",
        status.cpu_temp, temp_sensor
    );

    let descriptor = &mut daemon.registration_descriptor();

    assert!(descriptor.has_sensor(Sensor::CpuUsage));
    assert!(descriptor.has_sensor(Sensor::NetTx));
    assert!(descriptor.has_sensor(Sensor::NetRx));

    // After the first call we always have a value, it can be zero if the network interface didn't get used
    assert_ne!(status.net_rx, None);
    assert_ne!(status.net_tx, None);
    if temp_sensor.is_some() {
        assert!(descriptor.has_sensor(Sensor::CpuTemperature));
        assert_ne!(status.cpu_temp, None);
    }

    Ok(())
}

#[test]
fn test_registration() -> Result<(), Box<dyn Error>> {
    let network = Networks::new_with_refreshed_list();
    let components = Components::new_with_refreshed_list();
    let mut conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    conf.sensors.network = Some(
        network
            .iter()
            .next()
            .expect("Should have at least a network to test")
            .0
            .clone(),
    );
    conf.sensors.temperature = components
        .iter()
        .next()
        .map(|c| c.label())
        .map(|l| l.to_string());

    let temp_sensor = conf.sensors.temperature.clone();

    let prefix = "test_prefix";
    conf.mqtt.entity = "test_entity".to_string();

    let mut daemon = Daemon::new(conf);
    daemon.register_sensors();
    let descriptor = daemon.registration_descriptor();
    assert_eq!(
        descriptor.discovery_topic(prefix),
        "test_prefix/device/test_entity/config"
    );

    let json: HashMap<String, Value> = serde_json::from_str(descriptor.to_string().as_str())?;
    assert_eq!(json["device"]["name"].as_str().unwrap(), "test_entity");
    assert_eq!(
        json["state_topic"].as_str().unwrap(),
        "mqtt-system-monitor/test_entity/state"
    );
    if temp_sensor.is_some() {
        assert_eq!(
            json["components"]["cpu_temp"]["platform"].as_str().unwrap(),
            "sensor"
        );
        assert_eq!(
            json["components"]["cpu_temp"]["unique_id"]
                .as_str()
                .unwrap(),
            "test_entity_cpu_temp"
        );
    }
    assert_eq!(
        json["components"]["cpu_usage"]["platform"]
            .as_str()
            .unwrap(),
        "sensor"
    );
    assert_eq!(
        json["components"]["cpu_usage"]["unique_id"]
            .as_str()
            .unwrap(),
        "test_entity_cpu_usage"
    );
    assert_eq!(
        json["components"]["net_rx"]["platform"].as_str().unwrap(),
        "sensor"
    );
    assert_eq!(
        json["components"]["net_rx"]["unique_id"].as_str().unwrap(),
        "test_entity_net_rx"
    );
    assert_eq!(
        json["components"]["net_tx"]["platform"].as_str().unwrap(),
        "sensor"
    );
    assert_eq!(
        json["components"]["net_tx"]["unique_id"].as_str().unwrap(),
        "test_entity_net_tx"
    );

    Ok(())
}

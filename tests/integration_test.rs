use minijinja::{Environment, context};
use mqtt_system_monitor::configuration;
use mqtt_system_monitor::daemon::Daemon;
use mqtt_system_monitor::home_assistant::Sensor;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
use sysinfo::{Components, Networks};

#[test]
fn test_empty_values() -> Result<(), Box<dyn Error>> {
    let conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    let mut daemon = Daemon::new(conf);

    let status = daemon.update_data();
    assert!(status.network.is_empty());
    assert!(status.temperature.is_empty());

    let status = daemon.update_data();

    assert!(status.network.is_empty());
    assert!(status.temperature.is_empty());

    daemon.register_sensors();

    let descriptor = daemon.registration_descriptor();

    assert!(descriptor.has_sensor(Sensor::CpuUsage));

    Ok(())
}

#[test]
fn test_selection() -> Result<(), Box<dyn Error>> {
    let network = Networks::new_with_refreshed_list();
    let components = Components::new_with_refreshed_list();
    let mut conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    conf.sensors.network = network.iter().map(|n| n.0.clone()).collect();
    conf.sensors.temperature = components
        .iter()
        .map(|c| c.id().unwrap().to_string())
        .map(|id| id.to_string().clone())
        .collect();

    let interface = conf.sensors.network.clone();
    let temp_sensor = conf.sensors.temperature.clone();

    let mut daemon = Daemon::new(conf);

    daemon.register_sensors();

    let status = daemon.update_data();
    let network_status = &status.network[interface.first().unwrap()];

    println!("First read:");
    println!(
        "We have read net_tx={:?}, net_rx={:?} on interface {:?}",
        network_status.tx, network_status.rx, interface
    );
    println!("We have read temp={:?}", status.temperature);

    // The first call, the transfer rate is always at 0. It can be non-zero after some time
    assert_eq!(network_status.rx, 0.0);
    assert_eq!(network_status.tx, 0.0);
    if !temp_sensor.is_empty() {
        assert!(!status.temperature.is_empty());
    }

    let status = daemon.update_data();
    let network_status = &status.network.get(interface.first().unwrap());

    println!("Second read:");
    println!("We have read {network_status:?} on interface {interface:?}");
    println!("We have read temp={:?}", status.temperature);

    let descriptor = &mut daemon.registration_descriptor();

    assert!(descriptor.has_sensor(Sensor::CpuUsage));

    // After the first call we always have a value, it can be zero if the network interface didn't get used
    assert!(network_status.is_some());
    assert_eq!(temp_sensor.is_empty(), status.temperature.is_empty());

    Ok(())
}

#[test]
fn test_registration() -> Result<(), Box<dyn Error>> {
    let network = Networks::new_with_refreshed_list();
    let components = Components::new_with_refreshed_list();
    let mut conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    conf.sensors.network = network.iter().map(|n| n.0.clone()).collect();
    conf.sensors.temperature = components
        .iter()
        .map(|c| c.id().unwrap().to_string())
        .map(|id| id.to_string().clone())
        .collect();

    let first_interface = conf.sensors.network.first().unwrap().clone();
    let first_temp = conf.sensors.temperature.first().map(|s| s.clone());

    let prefix = "test_prefix";
    conf.mqtt.entity = "Test Entity".to_string();

    let mut daemon = Daemon::new(conf);
    daemon.register_sensors();
    let descriptor = daemon.registration_descriptor();
    assert_eq!(
        descriptor.discovery_topic(prefix),
        "test_prefix/device/test_entity/config"
    );

    let json: HashMap<String, Value> = serde_json::from_str(descriptor.to_string().as_str())?;
    assert_eq!(json["device"]["name"].as_str().unwrap(), "Test Entity");
    assert_eq!(
        json["device"]["identifiers"].as_str().unwrap(),
        "test_entity"
    );
    assert_eq!(
        json["state_topic"].as_str().unwrap(),
        "mqtt-system-monitor/test_entity/state"
    );
    if let Some(temp) = first_temp {
        assert_eq!(
            json["components"][format!("{temp}_temp")]["platform"]
                .as_str()
                .unwrap(),
            "sensor"
        );
        assert_eq!(
            json["components"][format!("{temp}_temp")]["unique_id"]
                .as_str()
                .unwrap(),
            format!("test_entity_{temp}_temp")
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
        json["components"]["memory_usage"]["unique_id"]
            .as_str()
            .unwrap(),
        "test_entity_memory_usage"
    );
    assert_eq!(
        json["components"][format!("{first_interface}_net_rx")]["platform"]
            .as_str()
            .unwrap(),
        "sensor"
    );
    assert_eq!(
        json["components"][format!("{first_interface}_net_rx")]["unique_id"]
            .as_str()
            .unwrap(),
        format!("test_entity_{first_interface}_net_rx")
    );
    assert_eq!(
        json["components"][format!("{first_interface}_net_tx")]["platform"]
            .as_str()
            .unwrap(),
        "sensor"
    );
    assert_eq!(
        json["components"][format!("{first_interface}_net_tx")]["unique_id"]
            .as_str()
            .unwrap(),
        format!("test_entity_{first_interface}_net_tx")
    );

    Ok(())
}

fn get_value<T>(
    env: &Environment,
    context: &minijinja::Value,
    name: &str,
) -> Result<T, <T as FromStr>::Err>
where
    T: FromStr,
{
    T::from_str(
        &env.get_template(name)
            .expect(&format!("Cannot find value {name}"))
            .render(&context)
            .expect("Failed to render value"),
    )
}

#[test]
fn test_templates() -> Result<(), Box<dyn Error>> {
    let network = Networks::new_with_refreshed_list();
    let components = Components::new_with_refreshed_list();
    let mut conf = configuration::Configuration::load("conf/mqtt-system-monitor.conf")?;

    conf.sensors.network = network.iter().map(|n| n.0.clone()).collect();
    conf.sensors
        .network
        .push("disconnected_interface".to_string());
    conf.sensors.temperature = components
        .iter()
        .map(|c| c.id().unwrap().to_string())
        .map(|id| id.to_string().clone())
        .collect();

    let first_interface = conf.sensors.network.first().unwrap().clone();
    let first_temperature = conf.sensors.temperature.first().map(|s| s.clone());

    let mut daemon = Daemon::new(conf);

    daemon.register_sensors();

    let status = daemon.update_data();

    let registration = daemon.registration_descriptor();

    println!("Input data: {status:?}");
    let mut env = Environment::new();
    let context = context!(value_json => status);
    for (name, device) in registration.components() {
        env.add_template(name, device.value_template())
            .expect("Invalid expression");
        let template = env.get_template(name).expect("Invalid template");
        let value = template.render(&context)?;

        println!("For component {name} the value is {value}");
        assert!(!value.is_empty());
    }

    assert_eq!(
        get_value::<f32>(&env, &context, "cpu_usage")?,
        status.cpu_usage.unwrap()
    );
    assert_eq!(
        get_value::<f32>(&env, &context, "memory_usage")?,
        status.memory_usage.unwrap()
    );
    assert_eq!(
        get_value::<String>(&env, &context, "disconnected_interface_net_rx")?,
        "none"
    );
    assert_eq!(
        get_value::<String>(&env, &context, "disconnected_interface_net_tx")?,
        "none"
    );
    assert_eq!(
        get_value::<f64>(&env, &context, &format!("{first_interface}_net_rx"))?,
        status.network[&first_interface].rx
    );
    assert_eq!(
        get_value::<f64>(&env, &context, &format!("{first_interface}_net_tx"))?,
        status.network[&first_interface].tx
    );

    if let Some(temp) = first_temperature {
        let name = format!("{temp}_temp");
        println!("Searching for {name}");
        assert_eq!(
            get_value::<f32>(&env, &context, &name)?,
            status.temperature[&temp]
        );
    }

    Ok(())
}

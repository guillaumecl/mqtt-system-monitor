use mqtt_system_monitor::daemon::Daemon;

use mqtt_system_monitor::configuration;

const DEFAULT_CONFIG_PATH: &str = "/etc/mqtt-system-monitor.conf";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let config_path = match args.get(1) {
        Some(path) => path.as_str(),
        None => DEFAULT_CONFIG_PATH,
    };

    let config =
        configuration::Configuration::load(&config_path).expect("Failed to load configuration");

    stderrlog::new()
        .module(module_path!())
        .verbosity(config.log_verbosity)
        .init()
        .expect("Failed to initialize logging");

    Daemon::new(config).run().await;
}

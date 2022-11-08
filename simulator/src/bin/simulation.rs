use env_logger::Env;
use log::info;

fn main() {
    let log_level = "debug";
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", log_level)
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);
    info!("Initialising simulation.");
}

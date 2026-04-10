use crate::service::config::ConfigService;

mod entity;
mod service;

fn main() {
    let config = ConfigService::read_config(Option::<&str>::None);
    match config {
        Ok(config) => {
            println!("{:?}", config);
        }
        Err(e) => {
            eprintln!("错误: {}", e);
        }
    }
}

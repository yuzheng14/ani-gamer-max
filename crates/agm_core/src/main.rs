use crate::service::config::ConfigService;

mod entity;
mod service;

#[tokio::main]
async fn main() {
    let config = ConfigService::read_config(Option::<&str>::None).await;
    match config {
        Ok(config) => {
            println!("{:?}", config);
        }
        Err(e) => {
            eprintln!("错误: {}", e);
        }
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgmCoreError {
    #[error("std io error: {0}")]
    StdIoError(#[from] std::io::Error),
    #[error("配置文件解析错误: {0}")]
    TomlDeserializeError(#[from] toml::de::Error),
    #[error("配置文件写入错误: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, AgmCoreError>;

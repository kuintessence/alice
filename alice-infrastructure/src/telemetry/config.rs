use serde::Deserialize;
use tracing_appender::rolling::Rotation;

/// 遥测系统配置
#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    /// 是否开启
    #[serde(default = "CommonLogConfig::default_enable")]
    pub enable: bool,

    /// 控制台输出设置
    #[serde(default)]
    pub console: ConsoleConfig,

    /// 远程输出设置
    #[serde(default)]
    pub remote: RemoteConfig,

    /// 文件输出设置
    #[serde(default)]
    pub file: FileConfig,
}

/// 控制台输出配置
#[derive(Debug, Default, Clone, Deserialize)]
pub struct ConsoleConfig(pub CommonLogConfig);

#[derive(Debug, Clone, Deserialize)]
pub struct CommonLogConfig {
    /// 启用
    #[serde(default = "CommonLogConfig::default_enable")]
    pub enable: bool,

    /// 调试模式：日志内容更详细
    #[serde(default)]
    pub verbose: bool,

    /// 自定义过滤规则
    #[serde(default)]
    pub filter: String,

    /// 自定义过滤规则环境变量
    #[serde(default)]
    pub filter_env: String,
}

/// 文件输出配置
#[derive(Debug, Clone, Deserialize)]
pub struct FileConfig {
    #[serde(flatten)]
    pub common: CommonLogConfig,

    /// 自定义日志文件夹位置（默认 `./logs`）
    #[serde(default = "FileConfig::default_path")]
    pub path: String,

    /// 自定义日志文件名，或滚动写入前缀（默认 `prefix.log`）
    #[serde(default = "FileConfig::default_filename")]
    pub prefix: String,

    /// 滚动创建文件写入时长，默认为 `Never` 即禁止滚动创建文件写入
    #[serde(default)]
    pub rolling_time: LogRotation,
}

/// 调用追踪配置
#[derive(Debug, Default, Clone, Deserialize)]
pub struct RemoteConfig {
    /// 启用调用追踪
    #[serde(default)]
    pub enable: bool,

    /// 远程收集器地址
    #[serde(default)]
    pub collector_endpoint: String,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
#[serde(from = "String")]
pub enum LogRotation {
    Daily,
    Hourly,
    Minutely,
    #[default]
    Never,
}

impl From<String> for LogRotation {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "daily" => Self::Daily,
            "hourly" => Self::Hourly,
            "minutely" => Self::Minutely,
            _ => Self::Never,
        }
    }
}

impl From<LogRotation> for Rotation {
    fn from(val: LogRotation) -> Self {
        match val {
            LogRotation::Daily => Rotation::DAILY,
            LogRotation::Hourly => Rotation::HOURLY,
            LogRotation::Minutely => Rotation::MINUTELY,
            LogRotation::Never => Rotation::NEVER,
        }
    }
}

impl CommonLogConfig {
    fn default_enable() -> bool {
        true
    }
}

impl FileConfig {
    fn default_path() -> String {
        String::from("./logs")
    }

    fn default_filename() -> String {
        String::from("prefix.log")
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enable: CommonLogConfig::default_enable(),
            console: Default::default(),
            remote: Default::default(),
            file: Default::default(),
        }
    }
}

impl Default for CommonLogConfig {
    fn default() -> Self {
        Self {
            enable: Self::default_enable(),
            verbose: Default::default(),
            filter: Default::default(),
            filter_env: Default::default(),
        }
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            common: Default::default(),
            path: Self::default_path(),
            prefix: Self::default_filename(),
            rolling_time: Default::default(),
        }
    }
}

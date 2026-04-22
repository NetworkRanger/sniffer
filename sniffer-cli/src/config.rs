
#[derive(Debug, Clone)]
pub struct Config {
    pub freq: u64, // 频率：单位秒
    pub has_domain: bool, // 是否有域名
}

impl Config {
    pub fn new() -> Self {
        Self { 
            freq: 3000,
            has_domain: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new_defaults() {
        let config = Config::new();
        assert_eq!(config.freq, 3000);
        assert!(config.has_domain);
    }

    #[test]
    fn test_config_clone() {
        let config = Config::new();
        let cloned = config.clone();
        assert_eq!(cloned.freq, config.freq);
        assert_eq!(cloned.has_domain, config.has_domain);
    }
}

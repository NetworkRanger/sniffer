
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
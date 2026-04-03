use std::any::{Any};
use std::collections::HashMap;
use std::sync::{Arc,  RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// 全局表：String → Box<dyn Any + Send + Sync>
type Table = Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>;

fn table() -> &'static Table {
    static INSTANCE: std::sync::OnceLock<Table> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

#[derive(Debug, Clone)]
struct ExpiredKey {
    id: String,
    timestamp: u64,
}

impl ExpiredKey {
    pub fn new(id: String, timeout: u64) -> Self {
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Self {
            id: id,
            timestamp: secs + timeout,
        }
    }
}

type ExpiredValue = Arc<RwLock<Vec<ExpiredKey>>>;

fn expired_value() -> &'static ExpiredValue {
    static INSTANCE: std::sync::OnceLock<ExpiredValue> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(Vec::new())))
}

pub struct Registry;
impl Registry{


    /// 更新过期键的函数
    ///
    /// # 参数
    /// * `key` - 需要更新的键名
    /// * `timeout` - 新的超时时间（秒）
    pub fn update_expired(key: String, timeout: u64) {
        // 读取过期值集合的不可变引用
        let vec_read = expired_value()
            .read()
            .ok()
            .unwrap();
        // 获取当前时间戳（从Unix纪元开始的秒数）
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        // 检查键是否存在于过期集合中
        match vec_read.iter().position(|k| k.id == key) {
            Some(index) => {
                // 获取现有值
                let value = vec_read[index].clone();
                let mut new_value = value.clone();
                // 更新时间戳
                new_value.timestamp = now_timestamp + timeout;
                drop(vec_read);
                // 如果时间戳确实发生了变化，则更新集合
                if new_value.timestamp != value.timestamp {
                    // 获取过期值集合的可变引用
                    let mut vec_write = expired_value().write().unwrap();
                    vec_write.push(new_value);
                    vec_write.swap_remove(index);
                }
            },
            None => {
                drop(vec_read);
                // 获取过期值集合的可变引用
                let mut vec_write = expired_value().write().unwrap();
                // 如果键不存在，则创建新的过期键
                vec_write.push(ExpiredKey::new(key, timeout));
            }
        };
        // 读取过期值集合的不可变引用
        let vec_read = expired_value()
            .read()
            .ok()
            .unwrap();
        // 过滤掉已过期的键
        vec_read.iter().filter_map(|k| {
            if k.timestamp >= now_timestamp {
                None
            } else {
                Some(k.id.clone())
            }
        }).for_each(|id| {
            // 移除已过期的键
            Self::remove(id);
        });
        drop(vec_read);
        // 获取过期值集合的可变引用
        let mut vec_write = expired_value().write().unwrap();
        // 保留未过期的键
        vec_write.retain(|k| k.timestamp >= now_timestamp);
    }
    
    pub fn update_expired_without_timeout(key: String) {
        // 读取过期值集合的不可变引用
        let vec_read = expired_value()
            .read()
            .ok()
            .unwrap();
    
        // 获取当前时间戳（从Unix纪元开始的秒数）
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if let Some(index) = vec_read.iter().position(|k| k.id == key) {
            // 获取现有值
            let value = vec_read[index].clone();
            let mut new_value = value.clone();
            // 更新时间戳
            new_value.timestamp = now_timestamp + 60;
            drop(vec_read);
            // 如果时间戳确实发生了变化，则更新集合
            if new_value.timestamp != value.timestamp {
                // 获取过期值集合的可变引用
                let mut vec_write = expired_value().write().unwrap();
                vec_write.push(new_value);
                vec_write.swap_remove(index);
            }
        }
    }

    /// 设置值（覆盖）
    pub fn set<T: Send + Sync + 'static>(key: impl Into<String>, value: T, timeout: Option<u64>) {
        let timeout = timeout.unwrap_or(60);
        let key: String = key.into();
        if timeout > 0 {
            Self::update_expired(key.clone(), timeout);
        }
        table()
            .write()
            .unwrap()
            .insert(key, Box::new(value));
    }

    /// 获取值，按指定类型向下转型
    pub fn get<T: Send + Sync + 'static + Clone>(key: impl Into<String>) -> Option<T> {
        let key = key.into();
        Self::update_expired_without_timeout(key.clone());
        table()
            .read()
            .ok()?
            .get(&key)
            .and_then(|any| any.downcast_ref::<T>())  // 使用and_then进行链式调用
            .cloned()
    }

    /// 删除键
    pub fn remove(key: impl Into<String>) -> bool {
        table().write().unwrap().remove(&key.into()).is_some()
    }
}


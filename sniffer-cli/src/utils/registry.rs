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


#[cfg(test)]
mod tests {
    use super::*;

    // Use unique keys per test to avoid interference from shared global state.

    #[test]
    fn test_set_and_get_string() {
        let key = "test_reg_str_1";
        Registry::set(key, "hello".to_string(), Some(60));
        let val: Option<String> = Registry::get(key);
        assert_eq!(val.unwrap(), "hello");
        Registry::remove(key);
    }

    #[test]
    fn test_set_and_get_i32() {
        let key = "test_reg_i32_1";
        Registry::set(key, 42i32, Some(60));
        let val: Option<i32> = Registry::get(key);
        assert_eq!(val.unwrap(), 42);
        Registry::remove(key);
    }

    #[test]
    fn test_get_wrong_type_returns_none() {
        let key = "test_reg_wrongtype_1";
        Registry::set(key, 42i32, Some(60));
        let val: Option<String> = Registry::get(key);
        assert!(val.is_none());
        Registry::remove(key);
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let val: Option<String> = Registry::get("test_reg_nonexistent_key_xyz");
        assert!(val.is_none());
    }

    #[test]
    fn test_remove_existing_key() {
        let key = "test_reg_remove_1";
        Registry::set(key, "value".to_string(), Some(60));
        assert!(Registry::remove(key));
        let val: Option<String> = Registry::get(key);
        assert!(val.is_none());
    }

    #[test]
    fn test_remove_nonexistent_key() {
        assert!(!Registry::remove("test_reg_remove_nonexistent_xyz"));
    }

    #[test]
    fn test_set_overwrite() {
        let key = "test_reg_overwrite_1";
        Registry::set(key, "first".to_string(), Some(60));
        Registry::set(key, "second".to_string(), Some(60));
        let val: Option<String> = Registry::get(key);
        assert_eq!(val.unwrap(), "second");
        Registry::remove(key);
    }

    #[test]
    fn test_set_with_zero_timeout_no_expiry_tracking() {
        // timeout=0 means no expiry tracking (the if timeout > 0 branch)
        let key = "test_reg_zero_timeout_1";
        Registry::set(key, "persistent".to_string(), Some(0));
        let val: Option<String> = Registry::get(key);
        assert_eq!(val.unwrap(), "persistent");
        Registry::remove(key);
    }

    #[test]
    fn test_set_default_timeout() {
        // None defaults to 60 seconds
        let key = "test_reg_default_timeout_1";
        Registry::set(key, 99i32, None);
        let val: Option<i32> = Registry::get(key);
        assert_eq!(val.unwrap(), 99);
        Registry::remove(key);
    }

    #[test]
    fn test_set_vec_type() {
        let key = "test_reg_vec_1";
        Registry::set(key, vec![1u8, 2, 3], Some(60));
        let val: Option<Vec<u8>> = Registry::get(key);
        assert_eq!(val.unwrap(), vec![1, 2, 3]);
        Registry::remove(key);
    }

    #[test]
    fn test_update_expired_creates_entry() {
        let key = "test_reg_expired_create_1".to_string();
        Registry::set(key.clone(), "data".to_string(), Some(60));
        // Key should still be accessible (not expired)
        let val: Option<String> = Registry::get(key.clone());
        assert_eq!(val.unwrap(), "data");
        Registry::remove(key);
    }

    #[test]
    fn test_update_expired_refreshes_timestamp() {
        let key = "test_reg_expired_refresh_1".to_string();
        Registry::set(key.clone(), "data".to_string(), Some(60));
        // Call update_expired again with a longer timeout
        Registry::update_expired(key.clone(), 120);
        let val: Option<String> = Registry::get(key.clone());
        assert_eq!(val.unwrap(), "data");
        Registry::remove(key);
    }
}

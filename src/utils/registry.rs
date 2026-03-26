use std::any::{Any};
use std::collections::HashMap;
use std::sync::{Arc,  RwLock};


// 全局表：String → Box<dyn Any + Send + Sync>
type Table = Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>;

fn table() -> &'static Table {
    static INSTANCE: std::sync::OnceLock<Table> = std::sync::OnceLock::new();
    INSTANCE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub struct Registry;
impl Registry{

    /// 设置值（覆盖）
    pub fn set<T: Send + Sync + 'static>(key: impl Into<String>, value: T) {
        table()
            .write()
            .unwrap()
            .insert(key.into(), Box::new(value));
    }

    /// 获取值，按指定类型向下转型
    pub fn get<T: Send + Sync + 'static + Clone>(key: impl Into<String>) -> Option<T> {
        table()
            .read()
            .ok()?
            .get(&key.into())
            .and_then(|any| any.downcast_ref::<T>())  // 使用and_then进行链式调用
            .cloned()
    }

    /// 删除键
    pub fn remove(key: impl Into<String>) -> bool {
        table().write().unwrap().remove(&key.into()).is_some()
    }
}


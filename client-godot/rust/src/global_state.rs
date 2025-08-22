/// 全局状态管理模块
/// 
/// 此模块提供线程安全的全局状态管理，替代不安全的静态变量
/// 由于 Godot 对象不是线程安全的，我们使用 thread_local 存储和 OnceCell 进行单线程使用

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::cell::RefCell;
use crate::{DbConnection, EntityController, PlayerController, PrefabManager};
use spacetimedb_sdk::Identity;
use godot::prelude::*;

/// 连接管理
/// 使用 Arc 包装以支持跨函数共享
static CONNECTION: OnceLock<Arc<DbConnection>> = OnceLock::new();

/// 本地身份标识
static LOCAL_IDENTITY: OnceLock<Identity> = OnceLock::new();

// 游戏实体存储 - 使用 thread_local 因为 Godot 对象不是线程安全的
thread_local! {
    static ENTITIES: RefCell<HashMap<u32, EntityController>> = RefCell::new(HashMap::new());
    static PLAYERS: RefCell<HashMap<u32, Gd<PlayerController>>> = RefCell::new(HashMap::new());
    static LOCAL_PLAYER: RefCell<Option<Arc<Gd<PlayerController>>>> = RefCell::new(None);
    static PREFAB_MANAGER: RefCell<Option<Gd<PrefabManager>>> = RefCell::new(None);
}

/// 连接管理函数
pub mod connection {
    use super::*;

    /// 设置数据库连接
    pub fn set_connection(conn: DbConnection) {
        let _ = CONNECTION.set(Arc::new(conn));
    }

    /// 获取数据库连接
    pub fn get_connection() -> Option<Arc<DbConnection>> {
        CONNECTION.get().cloned()
    }

    /// 检查是否已连接
    pub fn is_connected() -> bool {
        CONNECTION.get().is_some()
    }

    /// 清除连接
    pub fn clear_connection() {
        // OnceCell 不支持清除，但我们可以通过重新创建来实现类似效果
        // 在实际应用中，连接断开后通常会重新创建连接
    }
}

/// 身份管理函数
pub mod identity {
    use super::*;

    /// 设置本地身份
    pub fn set_local_identity(identity: Identity) {
        let _ = LOCAL_IDENTITY.set(identity);
    }

    /// 获取本地身份
    pub fn get_local_identity() -> Option<Identity> {
        LOCAL_IDENTITY.get().copied()
    }
}

/// 实体管理函数
pub mod entities {
    use super::*;

    /// 插入实体
    pub fn insert_entity(entity_id: u32, controller: EntityController) {
        ENTITIES.with_borrow_mut(|entities| {
            entities.insert(entity_id, controller);
        });
    }

    /// 获取实体（注意：EntityController 没有实现 Clone，所以不能直接返回拷贝）
    pub fn with_entity<F, R>(entity_id: u32, f: F) -> Option<R>
    where
        F: FnOnce(&EntityController) -> R,
    {
        ENTITIES.with_borrow(|entities| {
            entities.get(&entity_id).map(f)
        })
    }

    /// 移除实体
    pub fn remove_entity(entity_id: u32) -> Option<EntityController> {
        ENTITIES.with_borrow_mut(|entities| {
            entities.remove(&entity_id)
        })
    }

    /// 更新实体
    pub fn update_entity<F>(entity_id: u32, updater: F) -> bool 
    where
        F: FnOnce(&mut EntityController),
    {
        ENTITIES.with_borrow_mut(|entities| {
            if let Some(entity) = entities.get_mut(&entity_id) {
                updater(entity);
                true
            } else {
                false
            }
        })
    }
}

/// 玩家管理函数
pub mod players {
    use super::*;

    /// 插入玩家
    pub fn insert_player(player_id: u32, controller: Gd<PlayerController>) {
        PLAYERS.with_borrow_mut(|players| {
            players.insert(player_id, controller);
        });
    }

    /// 获取玩家
    pub fn get_player(player_id: u32) -> Option<Gd<PlayerController>> {
        PLAYERS.with_borrow(|players| {
            players.get(&player_id).cloned()
        })
    }

    /// 移除玩家
    pub fn remove_player(player_id: u32) -> Option<Gd<PlayerController>> {
        PLAYERS.with_borrow_mut(|players| {
            players.remove(&player_id)
        })
    }

    /// 检查玩家是否存在
    pub fn contains_player(player_id: u32) -> bool {
        PLAYERS.with_borrow(|players| {
            players.contains_key(&player_id)
        })
    }

    /// 设置本地玩家
    pub fn set_local_player(player: Gd<PlayerController>) {
        LOCAL_PLAYER.with_borrow_mut(|local_player| {
            *local_player = Some(Arc::new(player));
        });
    }

    /// 获取本地玩家
    pub fn get_local_player() -> Option<Arc<Gd<PlayerController>>> {
        LOCAL_PLAYER.with_borrow(|local_player| {
            local_player.clone()
        })
    }

    /// 清除本地玩家
    pub fn clear_local_player() {
        LOCAL_PLAYER.with_borrow_mut(|local_player| {
            *local_player = None;
        });
    }
}

/// 预制体管理器状态管理函数
pub mod prefab_state {
    use super::*;

    /// 设置预制体管理器实例
    pub fn set_instance(instance: Gd<PrefabManager>) {
        PREFAB_MANAGER.with_borrow_mut(|manager| {
            *manager = Some(instance);
        });
    }

    /// 获取预制体管理器实例
    pub fn get_instance() -> Option<Gd<PrefabManager>> {
        PREFAB_MANAGER.with_borrow(|manager| {
            manager.clone()
        })
    }
}
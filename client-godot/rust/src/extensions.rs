use crate::module_bindings::DbVector2;
use godot::prelude::Vector2;

impl From<DbVector2> for Vector2 {
    fn from(db_vector: DbVector2) -> Self {
        Vector2::new(db_vector.x, db_vector.y)
    }
}

impl From<Vector2> for DbVector2 {
    fn from(db_vector: Vector2) -> Self {
        DbVector2 {
            x: db_vector.x,
            y: db_vector.y,
        }
    }
}

impl From<&DbVector2> for Vector2 {
    fn from(db_vector: &DbVector2) -> Self {
        Vector2::new(db_vector.x, db_vector.y)
    }
}

impl From<&Vector2> for DbVector2 {
    fn from(db_vector: &Vector2) -> Self {
        DbVector2 {
            x: db_vector.x,
            y: db_vector.y,
        }
    }
}

pub trait DbVector2Ext {
    fn to_godot_pos(&self) -> Vector2;
}

impl DbVector2Ext for DbVector2 {
    fn to_godot_pos(&self) -> Vector2 {
        Vector2::new(self.x, -self.y)
    }
}

pub trait Vector2Ext {
    fn to_db_pos(&self) -> DbVector2;
    fn to_db_vec(&self) -> DbVector2;
}

impl Vector2Ext for Vector2 {
    fn to_db_pos(&self) -> DbVector2 {
        DbVector2 {
            x: self.x,
            y: -self.y,
        }
    }

    fn to_db_vec(&self) -> DbVector2 {
        DbVector2 {
            x: self.x,
            y: -self.y,
        }
    }
}
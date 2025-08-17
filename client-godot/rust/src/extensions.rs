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
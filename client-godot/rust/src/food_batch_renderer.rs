use super::*;
use crate::global_state::*;
use crate::module_bindings::Food;
use crate::entity_controller::mass_to_scale;
use godot::classes::{Control, IControl, Texture2D};
use godot::prelude::*;
use std::collections::HashMap;

/// 食物批量渲染器
/// 
/// 替代单独的FoodController节点，使用图形API批量绘制所有食物
#[derive(GodotClass)]
#[class(init, base=Control)]
pub struct FoodBatchRenderer {
    base: Base<Control>,
    
    /// 食物渲染数据映射表
    food_instances: HashMap<u32, FoodRenderData>,
    
    /// 是否需要重绘
    needs_redraw: bool,
    
    /// 食物纹理
    texture: Option<Gd<Texture2D>>,
}

/// 食物渲染数据
#[derive(Clone)]
pub struct FoodRenderData {
    /// 实体ID
    pub entity_id: u32,
    /// 当前渲染位置
    pub position: Vector2,
    /// 当前缩放
    pub scale: Vector2,
    /// 渲染颜色
    pub color: Color,
    /// 插值动画数据
    pub lerp_data: LerpData,
}

/// 插值动画数据
#[derive(Clone)]
pub struct LerpData {
    /// 插值时间
    pub lerp_time: f32,
    /// 起始位置
    pub start_position: Vector2,
    /// 目标位置
    pub target_position: Vector2,
    /// 目标缩放
    pub target_scale: Vector2,
}

impl Default for LerpData {
    fn default() -> Self {
        Self {
            lerp_time: 0.0,
            start_position: Vector2::ZERO,
            target_position: Vector2::ZERO,
            target_scale: Vector2::ONE,
        }
    }
}

/// 食物颜色调色板（与原FoodController保持一致）
const COLOR_PALETTE: &[Color] = &[
    Color::from_rgba8(119, 252, 173, 255),
    Color::from_rgba8(76, 250, 146, 255),
    Color::from_rgba8(35, 246, 120, 255),
    Color::from_rgba8(119, 251, 201, 255),
    Color::from_rgba8(76, 249, 184, 255),
    Color::from_rgba8(35, 245, 165, 255),
];

const FOOD_SIZE: Vector2 = Vector2::new(100.0, 100.0); // 基础食物大小
const LERP_DURATION_SEC: f32 = 0.1; // 插值动画持续时间

// 性能优化相关常量
const CULLING_MARGIN: f32 = 100.0; // 视锥剔除边距
const LOD_DISTANCE_HIGH: f32 = 200.0; // 高质量LOD距离
const LOD_DISTANCE_MEDIUM: f32 = 500.0; // 中等质量LOD距离

/// 食物LOD级别
#[derive(Clone, Copy, PartialEq)]
pub enum FoodLOD {
    High,    // 完整纹理
    Medium,  // 简化纹理
    Low,     // 单色圆点
}

impl FoodBatchRenderer {
    /// 添加食物到批量渲染器
    pub fn add_food(&mut self, food: &Food) {
        // 从Entity表获取位置和质量信息
        if let Some(conn) = connection::get_connection() {
            if let Some(entity) = conn.db.entity().entity_id().find(&food.entity_id) {
                let position: Vector2 = entity.position.into();
                let scale = mass_to_scale(entity.mass);
                
                // 选择颜色（根据entity_id）
                let color_index = (food.entity_id as usize) % COLOR_PALETTE.len();
                let color = COLOR_PALETTE[color_index];
                
                let food_data = FoodRenderData {
                    entity_id: food.entity_id,
                    position,
                    scale,
                    color,
                    lerp_data: LerpData {
                        lerp_time: 0.0,
                        start_position: position,
                        target_position: position,
                        target_scale: scale,
                    },
                };
                
                self.food_instances.insert(food.entity_id, food_data);
                self.needs_redraw = true;
                
                godot_print!("Food {} added to batch renderer at position {:?}", food.entity_id, position);
            }
        }
    }
    
    /// 从批量渲染器移除食物
    pub fn remove_food(&mut self, entity_id: u32) {
        if self.food_instances.remove(&entity_id).is_some() {
            self.needs_redraw = true;
            godot_print!("Food {} removed from batch renderer", entity_id);
        }
    }
    
    /// 更新食物实体数据（通常在entity_on_update时调用）
    pub fn update_food_entity(&mut self, entity: &crate::module_bindings::Entity) {
        if let Some(food_data) = self.food_instances.get_mut(&entity.entity_id) {
            // 重置插值动画
            food_data.lerp_data.lerp_time = 0.0;
            food_data.lerp_data.start_position = food_data.position;
            food_data.lerp_data.target_position = (&entity.position).into();
            food_data.lerp_data.target_scale = mass_to_scale(entity.mass);
            
            self.needs_redraw = true;
            
            godot_print!("Food entity {} updated in batch renderer", entity.entity_id);
        }
    }
    
    /// 检查是否包含指定食物
    pub fn contains_food(&self, entity_id: u32) -> bool {
        self.food_instances.contains_key(&entity_id)
    }
    
    /// 获取食物数量（用于调试）
    pub fn get_food_count(&self) -> usize {
        self.food_instances.len()
    }
    
    /// 性能优化：视锥剔除检查
    fn should_render_food(&self, food_data: &FoodRenderData, camera_bounds: Rect2) -> bool {
        let food_bounds = Rect2::new(
            food_data.position - (food_data.scale * FOOD_SIZE) * 0.5,
            food_data.scale * FOOD_SIZE
        );
        // 添加边距以确保边缘食物也能正常显示
        let extended_camera_bounds = Rect2::new(
            camera_bounds.position - Vector2::splat(CULLING_MARGIN),
            camera_bounds.size + Vector2::splat(CULLING_MARGIN * 2.0)
        );
        extended_camera_bounds.intersects(food_bounds)
    }
    
    /// 性能优化：获取食物LOD级别
    fn get_food_lod(&self, distance_to_camera: f32) -> FoodLOD {
        if distance_to_camera < LOD_DISTANCE_HIGH {
            FoodLOD::High
        } else if distance_to_camera < LOD_DISTANCE_MEDIUM {
            FoodLOD::Medium
        } else {
            FoodLOD::Low
        }
    }
    
    /// 计算食物到摄像机的距离
    fn calculate_distance_to_camera(&self, food_position: Vector2) -> f32 {
        // 简单的距离计算，实际项目中可以使用摄像机位置
        // 这里假设摄像机在原点附近
        food_position.length()
    }
    
    /// 更新食物插值动画
    fn update_food_lerp(&mut self, food_data: &mut FoodRenderData, delta: f32) -> bool {
        if food_data.lerp_data.lerp_time < LERP_DURATION_SEC {
            food_data.lerp_data.lerp_time = f32::min(
                food_data.lerp_data.lerp_time + delta,
                LERP_DURATION_SEC
            );
            
            let t = food_data.lerp_data.lerp_time / LERP_DURATION_SEC;
            
            // 位置插值
            food_data.position = Vector2::lerp(
                food_data.lerp_data.start_position,
                food_data.lerp_data.target_position,
                t
            );
            
            // 缩放插值
            food_data.scale = Vector2::lerp(
                food_data.scale,
                food_data.lerp_data.target_scale,
                delta * 8.0
            );
            
            true // 需要重绘
        } else {
            false // 动画完成，不需要重绘
        }
    }
}

#[godot_api]
impl IControl for FoodBatchRenderer {
    /// 初始化
    fn ready(&mut self) {        
        // 尝试加载食物纹理，如果失败则创建简单纹理
        match load::<Texture2D>("res://icon.svg") {
            texture => self.texture = Some(texture),
        }
        
        // 如果仍然没有纹理，则创建一个简单的白色纹理
        if self.texture.is_none() {
            godot_warn!("Could not load food texture, using fallback rendering");
        }
        
        godot_print!("FoodBatchRenderer ready and registered with {} foods", self.food_instances.len());
        godot_print!("FoodBatchRenderer instance: {:?}", self.base().instance_id());
        
        // 启用处理以便定期重绘
        self.base_mut().set_process(true);
    }
    
    /// 每帧处理
    fn process(&mut self, delta: f64) {
        let delta = delta as f32;
        let mut needs_redraw = false;
        
        // 更新所有食物的插值动画
        let mut entities_to_update = Vec::new();
        for (entity_id, food_data) in &self.food_instances {
            entities_to_update.push((*entity_id, food_data.clone()));
        }
        
        for (entity_id, mut food_data) in entities_to_update {
            if self.update_food_lerp(&mut food_data, delta) {
                needs_redraw = true;
                self.food_instances.insert(entity_id, food_data);
            }
        }
        
        // 如果有动画更新或标记需要重绘，则重绘
        if needs_redraw || self.needs_redraw {
            self.base_mut().queue_redraw();
            self.needs_redraw = false;
        }
    }
    
    /// 批量绘制所有食物（带性能优化）
    fn draw(&mut self) {
        // 先收集所有绘制数据避免借用冲突
        let foods_to_draw: Vec<FoodRenderData> = self.food_instances.values().cloned().collect();
        
        if foods_to_draw.is_empty() {
            return; // 没有食物需要绘制
        }
        
        godot_print!("Drawing {} foods", foods_to_draw.len());
        
        // 简单的摄像机边界计算（实际项目中应该从摄像机获取）
        let camera_bounds = Rect2::new(
            Vector2::new(-1000.0, -1000.0),
            Vector2::new(2000.0, 2000.0)
        );
        
        let mut rendered_count = 0;
        let mut culled_count = 0;
        
        // 提前克隆纹理以避免借用冲突
        let texture = self.texture.clone();
        
        for food_data in foods_to_draw {
            // 视锥剔除检查
            if !self.should_render_food(&food_data, camera_bounds) {
                culled_count += 1;
                continue;
            }
            
            // 计算到摄像机的距离
            let distance = self.calculate_distance_to_camera(food_data.position);
            let lod = self.get_food_lod(distance);
            
            // 计算绘制矩形
            let draw_rect = Rect2::new(
                food_data.position - (food_data.scale * FOOD_SIZE) * 0.5,
                food_data.scale * FOOD_SIZE
            );
            
            match lod {
                FoodLOD::High => {
                    if let Some(ref tex) = texture {
                        // 高质量：完整纹理绘制
                        self.base_mut().set_modulate(food_data.color);
                        self.base_mut().draw_texture_rect(
                            tex,
                            draw_rect,
                            false
                        );
                    } else {
                        // 无纹理时使用圆形
                        self.base_mut().draw_circle(
                            food_data.position,
                            (food_data.scale.x * FOOD_SIZE.x) * 0.5,
                            food_data.color
                        );
                    }
                }
                FoodLOD::Medium => {
                    if let Some(ref tex) = texture {
                        // 中等质量：稍小的纹理
                        let smaller_rect = Rect2::new(
                            draw_rect.position + draw_rect.size * 0.1,
                            draw_rect.size * 0.8
                        );
                        self.base_mut().set_modulate(food_data.color);
                        self.base_mut().draw_texture_rect(
                            tex,
                            smaller_rect,
                            false
                        );
                    } else {
                        // 无纹理时使用较小圆形
                        self.base_mut().draw_circle(
                            food_data.position,
                            (food_data.scale.x * FOOD_SIZE.x) * 0.4,
                            food_data.color
                        );
                    }
                }
                FoodLOD::Low => {
                    // 低质量：简单圆点
                    self.base_mut().draw_circle(
                        food_data.position,
                        (food_data.scale.x * FOOD_SIZE.x) * 0.3,
                        food_data.color
                    );
                }
            }
            
            rendered_count += 1;
        }
        
        // 重置颜色调制
        self.base_mut().set_modulate(Color::WHITE);
        
        // 调试信息
        if rendered_count > 0 {
            godot_print!("Food rendering: {} rendered, {} culled", rendered_count, culled_count);
        }
    }
}
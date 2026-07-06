use object_detector::DetectedObject;
use serde::{Deserialize, Serialize};

/// Corresponds to the 'object' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Object {
    pub position_x: f32,
    pub position_y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
    pub tag: String,
}

impl From<DetectedObject> for Object {
    fn from(object_box: DetectedObject) -> Self {
        Self {
            position_x: object_box.bbox.x1,
            position_y: object_box.bbox.y1,
            width: object_box.bbox.x2 - object_box.bbox.x1,
            height: object_box.bbox.y2 - object_box.bbox.y1,
            confidence: object_box.score,
            tag: object_box.tag,
        }
    }
}

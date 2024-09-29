use web_sys::HtmlImageElement;

#[derive(Clone)]
pub struct Enemy {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub speed: f64,
    pub image: HtmlImageElement, // 敵の画像
}

impl Enemy {
    // 必要に応じてメソッドを追加
}

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, HtmlAudioElement, HtmlImageElement,
};
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::Math::random;
use once_cell::sync::Lazy;

// ライフサイクルのための状態管理
#[derive(PartialEq)]
enum GameState {
    Playing,
    GameOver,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

struct Player {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
    image: HtmlImageElement, // プレイヤーの画像
}

struct Bullet {
    x: f64,
    y: f64,
    radius: f64,
    speed: f64,
    color: String,
}

#[derive(Clone)]
struct Enemy {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
    image: HtmlImageElement, // 敵の画像
}

struct Game {
    player: Player,
    bullets: Vec<Bullet>,
    enemies: Vec<Enemy>,
    last_enemy_spawn: f64,
    enemy_spawn_interval: f64,
    score: u32,
    lives: u32,
    state: GameState,
    keys_pressed: Vec<String>,
    context: CanvasRenderingContext2d,
    shoot_sound: HtmlAudioElement,
    explosion_sound: HtmlAudioElement,
    last_frame_time: f64,
    background_image: HtmlImageElement, // 背景画像
    enemy_image: HtmlImageElement,      // 敵の共通画像
}

impl Game {
    fn new(
        context: CanvasRenderingContext2d,
        shoot_sound: HtmlAudioElement,
        explosion_sound: HtmlAudioElement,
        player_image: HtmlImageElement,
        background_image: HtmlImageElement,
        enemy_image: HtmlImageElement,
    ) -> Rc<RefCell<Game>> {
        Rc::new(RefCell::new(Game {
            player: Player {
                x: 300.0,
                y: 550.0,
                width: 50.0,
                height: 50.0,
                speed: 5.0,
                image: player_image,
            },
            bullets: Vec::new(),
            enemies: Vec::new(),
            last_enemy_spawn: 0.0,
            enemy_spawn_interval: 2000.0, // 毎2秒に1体の敵を生成
            score: 0,
            lives: 3,
            state: GameState::Playing,
            keys_pressed: Vec::new(),
            context,
            shoot_sound,
            explosion_sound,
            last_frame_time: 0.0,
            background_image,
            enemy_image,
        }))
    }

    fn key_down(&mut self, key: String) {
        if !self.keys_pressed.contains(&key) {
            self.keys_pressed.push(key.clone());
        }

        if key == " " || key == "Space" {
            // スペースバーが押された場合、弾丸を発射
            self.fire_bullet();
        }
    }

    fn key_up(&mut self, key: String) {
        if let Some(pos) = self.keys_pressed.iter().position(|x| *x == key) {
            self.keys_pressed.remove(pos);
        }
    }

    fn fire_bullet(&mut self) {
        let bullet = Bullet {
            x: self.player.x + self.player.width / 2.0 - 5.0, // 弾丸の中央に合わせる
            y: self.player.y,
            radius: 5.0,
            speed: 7.0,
            color: "red".to_string(),
        };
        self.bullets.push(bullet);

        // 射撃音を再生
        let _ = self.shoot_sound.play();
    }

    fn spawn_enemy(&mut self) {
        let enemy_width = 50.0;
        let enemy_height = 50.0;
        let x = random() * (800.0 - enemy_width);
        let y = 0.0;
        let speed = 2.0 + random() * 3.0; // 2.0から5.0の速度

        let enemy = Enemy {
            x,
            y,
            width: enemy_width,
            height: enemy_height,
            speed,
            image: self.enemy_image.clone(),
        };
        self.enemies.push(enemy);
    }

    fn update_enemies(&mut self, _delta_time: f64) {
        for enemy in &mut self.enemies {
            enemy.y += enemy.speed;
        }

        // 敵が画面下に到達した場合、敵を削除
        self.enemies.retain(|enemy| enemy.y <= 600.0);
    }

    fn check_collisions(&mut self) {
        let mut bullets_to_remove = Vec::new();
        let mut enemies_to_remove = Vec::new();

        // 弾と敵の当たり判定
        for (b_idx, bullet) in self.bullets.iter().enumerate() {
            for (e_idx, enemy) in self.enemies.iter().enumerate() {
                // 弾を矩形として扱うために、幅と高さを設定
                let bullet_width = bullet.radius * 2.0;
                let bullet_height = bullet.radius * 2.0;

                if bullet.x < enemy.x + enemy.width
                    && bullet.x + bullet_width > enemy.x
                    && bullet.y < enemy.y + enemy.height
                    && bullet.y + bullet_height > enemy.y
                {
                    bullets_to_remove.push(b_idx);
                    enemies_to_remove.push(e_idx);
                    self.score += 1;

                    // 爆発音を再生
                    let _ = self.explosion_sound.play();
                }
            }
        }

        // プレイヤーと敵の衝突判定
        let mut enemies_to_remove_on_collision = Vec::new();
        for (e_idx, enemy) in self.enemies.iter().enumerate() {
            if self.player.x < enemy.x + enemy.width
                && self.player.x + self.player.width > enemy.x
                && self.player.y < enemy.y + enemy.height
                && self.player.y + self.player.height > enemy.y
            {
                enemies_to_remove_on_collision.push(e_idx);
                self.lives = self.lives.saturating_sub(1);

                // ダメージ音やエフェクトを追加する場合はここに記述
            }
        }

        // 重複削除
        bullets_to_remove.sort_unstable();
        bullets_to_remove.dedup();
        enemies_to_remove.sort_unstable();
        enemies_to_remove.dedup();
        enemies_to_remove_on_collision.sort_unstable();
        enemies_to_remove_on_collision.dedup();

        // 弾丸と敵を削除
        for &b_idx in bullets_to_remove.iter().rev() {
            self.bullets.remove(b_idx);
        }
        for &e_idx in enemies_to_remove.iter().rev() {
            self.enemies.remove(e_idx);
        }
        // プレイヤーと衝突した敵を削除
        for &e_idx in enemies_to_remove_on_collision.iter().rev() {
            self.enemies.remove(e_idx);
        }

        // ライフが0になったらゲームオーバー
        if self.lives == 0 {
            self.state = GameState::GameOver;
        }
    }

    fn start(game_rc: Rc<RefCell<Self>>) {
        let closure = Closure::wrap(Box::new(move |timestamp: f64| {
            {
                let mut game = game_rc.borrow_mut();
                if game.state == GameState::Playing {
                    game.render_frame(timestamp);
                }
            }
            // 再度アニメーションフレームを要求
            if game_rc.borrow().state == GameState::Playing {
                Game::start(game_rc.clone());
            }
        }) as Box<dyn FnMut(f64)>);

        web_sys::window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .expect("requestAnimationFrame failed");

        closure.forget(); // クロージャをメモリに保持させる
    }

    fn render_frame(&mut self, current_time: f64) {
        // 初回フレームでlast_enemy_spawnを設定
        if self.last_enemy_spawn == 0.0 {
            self.last_enemy_spawn = current_time;
        }

        // フレーム間の経過時間を計算
        let delta_time = current_time - self.last_frame_time;
        self.last_frame_time = current_time;

        // 敵の生成
        if current_time - self.last_enemy_spawn > self.enemy_spawn_interval {
            self.spawn_enemy();
            self.last_enemy_spawn = current_time;
        }

        // キー入力に基づいてプレイヤーの移動
        if self.keys_pressed.contains(&"ArrowLeft".to_string())
            || self.keys_pressed.contains(&"a".to_string())
        {
            self.player.x -= self.player.speed;
            if self.player.x < 0.0 {
                self.player.x = 0.0;
            }
        }

        if self.keys_pressed.contains(&"ArrowRight".to_string())
            || self.keys_pressed.contains(&"d".to_string())
        {
            self.player.x += self.player.speed;
            if self.player.x + self.player.width > 800.0 {
                self.player.x = 800.0 - self.player.width;
            }
        }

        if self.keys_pressed.contains(&"ArrowUp".to_string())
            || self.keys_pressed.contains(&"w".to_string())
        {
            self.player.y -= self.player.speed;
            if self.player.y < 0.0 {
                self.player.y = 0.0;
            }
        }

        if self.keys_pressed.contains(&"ArrowDown".to_string())
            || self.keys_pressed.contains(&"s".to_string())
        {
            self.player.y += self.player.speed;
            if self.player.y + self.player.height > 600.0 {
                self.player.y = 600.0 - self.player.height;
            }
        }

        // 弾丸の位置を更新
        self.bullets.iter_mut().for_each(|bullet| {
            bullet.y -= bullet.speed;
        });

        // 弾丸が画面外に出た場合、弾丸を削除
        self.bullets.retain(|bullet| bullet.y >= 0.0);

        // 敵の位置を更新
        self.update_enemies(delta_time);

        // 衝突判定
        self.check_collisions();

        // Canvasをクリア
        self.context.clear_rect(0.0, 0.0, 800.0, 600.0);

        // 背景画像を描画
        if let Err(e) = self.context.draw_image_with_html_image_element(
            &self.background_image,
            0.0,
            0.0,
        ) {
            console_log!("Error drawing background: {:?}", e);
        }

        // プレイヤーを描画
        if let Err(e) = self.context.draw_image_with_html_image_element(
            &self.player.image,
            self.player.x,
            self.player.y,
        ) {
            console_log!("Error drawing player: {:?}", e);
        }

        // 弾丸を描画
        for bullet in &self.bullets {
            self.context.begin_path();
            if let Err(e) = self.context.arc(
                bullet.x + bullet.radius,
                bullet.y + bullet.radius,
                bullet.radius,
                0.0,
                std::f64::consts::PI * 2.0,
            ) {
                console_log!("Error drawing arc: {:?}", e);
            }
            self.context.set_fill_style(&JsValue::from_str(&bullet.color));
            self.context.fill();
        }

        // 敵を描画
        for enemy in &self.enemies {
            if let Err(e) = self.context.draw_image_with_html_image_element(
                &enemy.image,
                enemy.x,
                enemy.y,
            ) {
                console_log!("Error drawing enemy: {:?}", e);
            }
        }

        // スコアを更新
        self.update_ui();
    }

    fn update_ui(&self) {
        // スコアをHTML要素に反映
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");

        let score_element = document
            .get_element_by_id("score")
            .expect("should have score element");
        score_element.set_inner_html(&self.score.to_string());

        // ライフをHTML要素に反映
        let lives_element = document
            .get_element_by_id("lives")
            .expect("should have lives element");
        lives_element.set_inner_html(&self.lives.to_string());

        // ゲームオーバー時の処理
        let game_over_element = document.get_element_by_id("gameOver");
        if self.state == GameState::GameOver {
            if let Some(element) = game_over_element {
                element
                    .dyn_ref::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "block")
                    .unwrap();
            }
        } else {
            // ゲームオーバーでない場合、非表示にする
            if let Some(element) = game_over_element {
                element
                    .dyn_ref::<HtmlElement>()
                    .unwrap()
                    .style()
                    .set_property("display", "none")
                    .unwrap();
            }
        }
    }

    fn reset(&mut self) {
        self.player.x = 300.0;
        self.player.y = 550.0;
        self.bullets.clear();
        self.enemies.clear();
        self.last_enemy_spawn = 0.0;
        self.score = 0;
        self.lives = 3; // ライフの初期化
        self.state = GameState::Playing;
        self.keys_pressed.clear();

        // ゲームオーバー表示を非表示にする
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let game_over_element = document.get_element_by_id("gameOver");
        if let Some(element) = game_over_element {
            element
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .style()
                .set_property("display", "none")
                .unwrap();
        }
    }
}

// グローバルなゲームインスタンスを保持
static mut GAME: Option<Rc<RefCell<Game>>> = None;

#[wasm_bindgen]
pub fn start_game() {
    // Webシステムの初期化
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let canvas = document
        .get_element_by_id("gameCanvas")
        .expect("should have gameCanvas element")
        .dyn_into::<HtmlCanvasElement>()
        .expect("gameCanvas should be a HtmlCanvasElement");
    let context = canvas
        .get_context("2d")
        .expect("should have 2d context")
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context should be CanvasRenderingContext2d");

    // オーディオ要素の取得
    let shoot_sound = document
        .get_element_by_id("shootSound")
        .expect("should have shootSound element")
        .dyn_into::<HtmlAudioElement>()
        .expect("shootSound should be HtmlAudioElement");
    let explosion_sound = document
        .get_element_by_id("explosionSound")
        .expect("should have explosionSound element")
        .dyn_into::<HtmlAudioElement>()
        .expect("explosionSound should be HtmlAudioElement");

    // 画像のロード
    let player_image = HtmlImageElement::new().unwrap();
    player_image.set_src("assets/player.png");

    let background_image = HtmlImageElement::new().unwrap();
    background_image.set_src("assets/background.png");

    let enemy_image = HtmlImageElement::new().unwrap();
    enemy_image.set_src("assets/enemy.png");

    // ゲームの初期化
    let game = Game::new(
        context,
        shoot_sound,
        explosion_sound,
        player_image,
        background_image,
        enemy_image,
    );

    // グローバルなゲームインスタンスを設定
    unsafe {
        GAME = Some(game.clone());
    }

    // キーボードイベントリスナーの設定
    {
        let game_rc = game.clone();
        let key_down_closure =
            Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                let key = event.key();
                game_rc.borrow_mut().key_down(key);
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        window
            .add_event_listener_with_callback(
                "keydown",
                key_down_closure.as_ref().unchecked_ref(),
            )
            .expect("failed to add keydown listener");
        key_down_closure.forget();
    }

    {
        let game_rc = game.clone();
        let key_up_closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let key = event.key();
            game_rc.borrow_mut().key_up(key);
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        window
            .add_event_listener_with_callback("keyup", key_up_closure.as_ref().unchecked_ref())
            .expect("failed to add keyup listener");
        key_up_closure.forget();
    }

    // ゲームの開始
    Game::start(game.clone());
}

#[wasm_bindgen]
pub fn reset_game() {
    // グローバルなゲームインスタンスを取得してリセット
    unsafe {
        if let Some(game_rc) = &mut GAME {
            game_rc.borrow_mut().reset();
            // ゲームループを再開
            Game::start(game_rc.clone());
        }
    }
}

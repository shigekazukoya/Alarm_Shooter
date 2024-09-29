use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, HtmlAudioElement,
};
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::Math::random;
use once_cell::sync::Lazy;
use std::sync::Mutex;

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
    color: String,
}

struct Bullet {
    x: f64,
    y: f64,
    radius: f64,
    speed: f64,
    color: String,
}

#[derive(Clone, PartialEq)]
struct Enemy {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    speed: f64,
    color: String,
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
    last_frame_time: f64, // 追加
}

impl Game {
    fn new(
        context: CanvasRenderingContext2d,
        shoot_sound: HtmlAudioElement,
        explosion_sound: HtmlAudioElement,
    ) -> Rc<RefCell<Game>> {
        Rc::new(RefCell::new(Game {
            player: Player {
                x: 400.0,
                y: 550.0,
                width: 50.0,
                height: 50.0,
                speed: 5.0,
                color: "blue".to_string(),
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
            last_frame_time: 0.0, // 初期化
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
        let color = "green".to_string();

        let enemy = Enemy {
            x,
            y,
            width: enemy_width,
            height: enemy_height,
            speed,
            color,
        };
        self.enemies.push(enemy);
    }

    fn update_enemies(&mut self, _delta_time: f64) { // プレフィックス _ を追加
        for enemy in &mut self.enemies {
            enemy.y += enemy.speed;
        }

        // 敵が画面下に到達した場合
        let mut enemies_to_remove = Vec::new();
        for enemy in &self.enemies {
            if enemy.y > 600.0 {
                enemies_to_remove.push(enemy.clone());
            }
        }
        self.enemies.retain(|e| !enemies_to_remove.contains(e));

        if self.lives == 0 {
            self.state = GameState::GameOver;
        }
    }

    fn check_collisions(&mut self) {
        let mut bullets_to_remove = Vec::new();
        let mut enemies_to_remove = Vec::new();

        for (b_idx, bullet) in self.bullets.iter().enumerate() {
            for (e_idx, enemy) in self.enemies.iter().enumerate() {
                if bullet.x > enemy.x
                    && bullet.x < enemy.x + enemy.width
                    && bullet.y > enemy.y
                    && bullet.y < enemy.y + enemy.height
                {
                    bullets_to_remove.push(b_idx);
                    enemies_to_remove.push(e_idx);
                    self.score += 1;

                    // 爆発音を再生
                    let _ = self.explosion_sound.play();
                }
            }
        }

        // 重複削除
        bullets_to_remove.sort_unstable();
        bullets_to_remove.dedup();
        enemies_to_remove.sort_unstable();
        enemies_to_remove.dedup();

        // 弾丸と敵を削除
        for &b_idx in bullets_to_remove.iter().rev() {
            self.bullets.remove(b_idx);
        }
        for &e_idx in enemies_to_remove.iter().rev() {
            self.enemies.remove(e_idx);
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

        closure.forget(); // クローズをメモリに保持させる
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
    if self.keys_pressed.contains(&"ArrowLeft".to_string()) || self.keys_pressed.contains(&"a".to_string()) {
        self.player.x -= self.player.speed;
        if self.player.x < 0.0 {
            self.player.x = 0.0;
        }
    }

        if self.keys_pressed.contains(&"ArrowRight".to_string()) || self.keys_pressed.contains(&"d".to_string()) {
            self.player.x += self.player.speed;
            if self.player.x + self.player.width > 800.0 {
                self.player.x = 800.0 - self.player.width;
            }
        }

         if self.keys_pressed.contains(&"ArrowUp".to_string()) || self.keys_pressed.contains(&"w".to_string()) {
        self.player.y -= self.player.speed;
        if self.player.y < 0.0 {
            self.player.y = 0.0;
        }
    }

    if self.keys_pressed.contains(&"ArrowDown".to_string()) || self.keys_pressed.contains(&"s".to_string()) {
        self.player.y += self.player.speed;
        if self.player.y + self.player.height > 600.0 {
            self.player.y = 600.0 - self.player.height;
        }
    }

        // 弾丸の位置を更新
        self.bullets.iter_mut().for_each(|bullet| {
            bullet.y -= bullet.speed;
        });

        // 敵の位置を更新
        self.update_enemies(delta_time);

        // 衝突判定
        self.check_collisions();

        // Canvasをクリア
        self.context.set_fill_style(&JsValue::from_str("black"));
        self.context.fill_rect(0.0, 0.0, 800.0, 600.0); // `.unwrap()` を削除

        // プレイヤーを描画
        self.context.set_fill_style(&JsValue::from_str(&self.player.color));
        self.context.fill_rect(
            self.player.x,
            self.player.y,
            self.player.width,
            self.player.height,
        );

        // 弾丸を描画
        for bullet in &self.bullets {
            self.context.begin_path();
            if let Err(e) = self.context.arc(bullet.x, bullet.y, bullet.radius, 0.0, std::f64::consts::PI * 2.0) {
                console_log!("Error drawing arc: {:?}", e);
            }
            self.context.set_fill_style(&JsValue::from_str(&bullet.color));
            self.context.fill();
        }

        // 敵を描画
        for enemy in &self.enemies {
            self.context.set_fill_style(&JsValue::from_str(&enemy.color));
            self.context.fill_rect(
                enemy.x,
                enemy.y,
                enemy.width,
                enemy.height,
            );
        }

        // スコアとライフを更新
        self.update_ui();
    }

    fn update_ui(&self) {
        // スコアとライフをHTML要素に反映
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");

        let score_element = document
            .get_element_by_id("score")
            .expect("should have score element");
        score_element.set_inner_html(&self.score.to_string());

        let lives_element = document
            .get_element_by_id("lives")
            .expect("should have lives element");
        lives_element.set_inner_html(&self.lives.to_string());

        // ゲームオーバー時の処理
        if self.state == GameState::GameOver {
            let game_over_element = document
                .get_element_by_id("gameOver")
                .expect("should have gameOver element");
            game_over_element
                .dyn_ref::<HtmlElement>()
                .unwrap()
                .style()
                .set_property("display", "block")
                .unwrap();
        }
    }

    fn reset(&mut self) {
        self.player.x = 400.0;
        self.bullets.clear();
        self.enemies.clear();
        self.last_enemy_spawn = 0.0;
        self.score = 0;
        self.lives = 3;
        self.state = GameState::Playing;
        self.keys_pressed.clear();

        // ゲームオーバー表示を非表示にする
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let game_over_element = document
            .get_element_by_id("gameOver")
            .expect("should have gameOver element");
        game_over_element
            .dyn_ref::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("display", "none")
            .unwrap();
    }
}

// グローバルなゲームインスタンスを unsafe static mut に変更
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

    let game = Game::new(context, shoot_sound, explosion_sound);

    // グローバルなゲームインスタンスを設定（unsafe ブロック内で）
    unsafe {
        GAME = Some(game.clone());
    }

    // キーボードイベントリスナーの設定
    {
        let game_rc = game.clone();
        let key_down_closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let key = event.key();
            game_rc.borrow_mut().key_down(key);
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        window
            .add_event_listener_with_callback("keydown", key_down_closure.as_ref().unchecked_ref())
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
    // グローバルなゲームインスタンスを取得してリセット（unsafe ブロック内で）
    unsafe {
        if let Some(game_rc) = &mut GAME {
            game_rc.borrow_mut().reset();
            // ゲームループを再開
            Game::start(game_rc.clone());
        }
    }
}
use wasm_bindgen::{prelude::*, JsCast, closure::Closure, JsValue};
use web_sys::{window, CanvasRenderingContext2d, HtmlAudioElement, HtmlImageElement, HtmlElement};
use std::cell::RefCell;
use std::rc::Rc;
use js_sys::Math::random;
use std::f64::consts::PI;

use crate::game::{Player, Bullet, Enemy, GameState};
use crate::console_log;

pub struct Game {
    pub player: Player,
    pub bullets: Vec<Bullet>,
    pub enemies: Vec<Enemy>,
    pub last_enemy_spawn: f64,
    pub enemy_spawn_interval: f64,
    pub score: u32,
    pub lives: u32,
    pub state: GameState,
    pub keys_pressed: Vec<String>,
    pub context: CanvasRenderingContext2d,
    pub shoot_sound: HtmlAudioElement,
    pub explosion_sound: HtmlAudioElement,
    pub last_frame_time: f64,
    pub background_image: HtmlImageElement, // 背景画像
    pub enemy_image: HtmlImageElement,      // 敵の共通画像
}

impl Game {
    pub fn new(
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

    pub fn key_down(&mut self, key: String) {
        if !self.keys_pressed.contains(&key) {
            self.keys_pressed.push(key.clone());
        }

        if key == " " || key == "Space" {
            // スペースバーが押された場合、弾丸を発射
            self.fire_bullet();
        }
    }

    pub fn key_up(&mut self, key: String) {
        if let Some(pos) = self.keys_pressed.iter().position(|x| *x == key) {
            self.keys_pressed.remove(pos);
        }
    }

    pub fn fire_bullet(&mut self) {
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

    pub fn spawn_enemy(&mut self) {
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

    pub fn update_enemies(&mut self, _delta_time: f64) {
        for enemy in &mut self.enemies {
            enemy.y += enemy.speed;
        }

        // 敵が画面下に到達した場合、敵を削除
        self.enemies.retain(|enemy| enemy.y <= 600.0);
    }

    pub fn check_collisions(&mut self) {
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

    pub fn start(game_rc: Rc<RefCell<Self>>) {
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

        window()
            .unwrap()
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .expect("requestAnimationFrame failed");

        closure.forget(); // クロージャをメモリに保持させる
    }

    pub fn render_frame(&mut self, current_time: f64) {
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
                PI * 2.0,
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

    pub fn update_ui(&self) {
        // スコアをHTML要素に反映
        let window = window().expect("no global `window` exists");
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

    pub fn reset(&mut self) {
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
        let window = window().expect("no global `window` exists");
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

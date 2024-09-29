use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, HtmlAudioElement, HtmlImageElement};

use std::cell::RefCell;
use std::rc::Rc;

use crate::game::{Game, GameState};

// グローバルなゲームインスタンス
static mut GAME: Option<Rc<RefCell<Game>>> = None;

pub fn start_game() {
    // ウィンドウとドキュメントの取得
    let window = window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    // Canvasの取得と2Dコンテキストの設定
    let canvas = document
        .get_element_by_id("gameCanvas")
        .expect("should have gameCanvas element")
        .dyn_into::<HtmlCanvasElement>()
        .expect("gameCanvas should be a HtmlCanvasElement");
    let context = canvas
        .get_context("2d")
        .expect("should have 2d context")
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
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

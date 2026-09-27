use std::cell::{Cell, RefCell};

use gloo_net::http::{Request, Response};
use serde::Serialize;
use serde_json::{json, Value};
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;

const TOKEN_KEY: &str = "blog_token";
const USER_ID_KEY: &str = "blog_user_id";

#[wasm_bindgen]
pub struct BlogApp {
    server_url: String,
    token: RefCell<Option<String>>,
    user_id: Cell<Option<i64>>,
}

impl Default for BlogApp {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl BlogApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BlogApp {
        Self::with_server("http://127.0.0.1:8080".to_string())
    }

    pub fn with_server(server_url: String) -> BlogApp {
        BlogApp {
            server_url: server_url.trim_end_matches('/').to_string(),
            token: RefCell::new(get_token_from_storage()),
            user_id: Cell::new(
                storage()
                    .and_then(|store| store.get_item(USER_ID_KEY).ok().flatten())
                    .and_then(|value| value.parse::<i64>().ok()),
            ),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.borrow().is_some()
    }

    pub fn current_user_id(&self) -> Option<i64> {
        self.user_id.get()
    }

    pub fn logout(&self) {
        *self.token.borrow_mut() = None;
        self.user_id.set(None);
        if let Some(store) = storage() {
            let _ = store.remove_item(TOKEN_KEY);
            let _ = store.remove_item(USER_ID_KEY);
        }
    }

    pub async fn register(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<JsValue, JsValue> {
        if username.trim().is_empty() || email.trim().is_empty() || password.trim().is_empty() {
            return Err(js_error("Заполните все поля"));
        }
        let request = Request::post(&format!("{}/api/auth/register", self.server_url))
            .json(&json!({ "username": username, "email": email, "password": password }))
            .map_err(js_error)?;
        let result = response_json(request.send().await.map_err(js_error)?).await?;
        self.store_auth(&result)?;
        to_js(&result)
    }

    pub async fn login(&self, username: String, password: String) -> Result<JsValue, JsValue> {
        if username.trim().is_empty() || password.trim().is_empty() {
            return Err(js_error("Заполните все поля"));
        }
        let request = Request::post(&format!("{}/api/auth/login", self.server_url))
            .json(&json!({ "username": username, "password": password }))
            .map_err(js_error)?;
        let result = response_json(request.send().await.map_err(js_error)?).await?;
        self.store_auth(&result)?;
        to_js(&result)
    }

    pub async fn load_posts(&self) -> Result<JsValue, JsValue> {
        let response = Request::get(&format!("{}/api/posts?limit=100&offset=0", self.server_url))
            .send()
            .await
            .map_err(js_error)?;
        let result = response_json(response).await?;
        to_js(&result)
    }

    pub async fn get_post(&self, id: i64) -> Result<JsValue, JsValue> {
        let response = Request::get(&format!("{}/api/posts/{id}", self.server_url))
            .send()
            .await
            .map_err(js_error)?;
        let result = response_json(response).await?;
        to_js(&result)
    }

    pub async fn create_post(&self, title: String, content: String) -> Result<JsValue, JsValue> {
        validate_post(&title, &content)?;
        let token = self.required_token()?;
        let request = Request::post(&format!("{}/api/posts", self.server_url))
            .header("Authorization", &format!("Bearer {token}"))
            .json(&json!({ "title": title, "content": content }))
            .map_err(js_error)?;
        let result = response_json(request.send().await.map_err(js_error)?).await?;
        to_js(&result)
    }

    pub async fn update_post(
        &self,
        id: i64,
        title: String,
        content: String,
    ) -> Result<JsValue, JsValue> {
        validate_post(&title, &content)?;
        let token = self.required_token()?;
        let request = Request::put(&format!("{}/api/posts/{id}", self.server_url))
            .header("Authorization", &format!("Bearer {token}"))
            .json(&json!({ "title": title, "content": content }))
            .map_err(js_error)?;
        let result = response_json(request.send().await.map_err(js_error)?).await?;
        to_js(&result)
    }

    pub async fn delete_post(&self, id: i64) -> Result<JsValue, JsValue> {
        let token = self.required_token()?;
        let response = Request::delete(&format!("{}/api/posts/{id}", self.server_url))
            .header("Authorization", &format!("Bearer {token}"))
            .send()
            .await
            .map_err(js_error)?;
        if !response.ok() {
            return Err(js_error(response.text().await.map_err(js_error)?));
        }
        Ok(JsValue::NULL)
    }
}

impl BlogApp {
    fn required_token(&self) -> Result<String, JsValue> {
        self.token
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Войдите, чтобы изменить пост"))
    }

    fn store_auth(&self, result: &Value) -> Result<(), JsValue> {
        let token = result
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| js_error("В ответе сервера нет токена"))?;
        let user_id = result
            .get("user")
            .and_then(|user| user.get("id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| js_error("В ответе сервера нет пользователя"))?;
        save_token_to_storage(token)?;
        if let Some(store) = storage() {
            store.set_item(USER_ID_KEY, &user_id.to_string())?;
        }
        *self.token.borrow_mut() = Some(token.to_string());
        self.user_id.set(Some(user_id));
        Ok(())
    }
}

fn validate_post(title: &str, content: &str) -> Result<(), JsValue> {
    if title.trim().is_empty() || content.trim().is_empty() {
        Err(js_error("Заполните заголовок и текст"))
    } else {
        Ok(())
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

fn save_token_to_storage(token: &str) -> Result<(), JsValue> {
    let store = storage().ok_or_else(|| js_error("localStorage недоступен"))?;
    store.set_item(TOKEN_KEY, token)
}

fn get_token_from_storage() -> Option<String> {
    storage().and_then(|store| store.get_item(TOKEN_KEY).ok().flatten())
}

async fn response_json(response: Response) -> Result<Value, JsValue> {
    if !response.ok() {
        let status = response.status();
        let body = response.text().await.map_err(js_error)?;
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or(body);
        return Err(js_error(format!("HTTP {status}: {message}")));
    }
    response.json().await.map_err(js_error)
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(js_error)
}

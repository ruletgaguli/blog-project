# blog-project

## Проектная работа модуля 3. Блог на Rust

Один Cargo workspace содержит четыре крейта:

- `blog-server` — HTTP API на Actix Web, gRPC API на tonic, PostgreSQL, SQLx, Argon2 и JWT;
- `blog-client` — библиотека с одинаковыми методами для HTTP и gRPC;
- `blog-cli` — команды регистрации, входа и управления постами;
- `blog-wasm` — HTTP-клиент для браузера на Rust/WebAssembly.

Сервер использует общие сервисы `AuthService` и `BlogService` для HTTP и gRPC. В папке `blog-server/proto` хранится единственный `blog.proto`; оба крейта генерируют код из него при сборке. Публично доступны список и отдельный пост. Создание, обновление и удаление требуют JWT, а изменять или удалять пост может только его автор.

## Подготовка

Нужны Rust/Cargo, Docker с Compose, `wasm-pack` и установленная цель `wasm32-unknown-unknown` для браузерной сборки. `protoc` установленным быть не обязан: он включён в сборочные зависимости. PostgreSQL можно запустить локально без Docker, если указать другой `DATABASE_URL`.

Из корня проекта (конфигурация PostgreSQL находится в `docker-compose.yaml`):

```bash
docker compose up -d postgres
cp blog-server/.env.example blog-server/.env
```

В `blog-server/.env` задайте собственный `JWT_SECRET` длиной не менее 32 символов. Например, получите значение командой `openssl rand -hex 32`. Не публикуйте этот файл: он исключён из Git. Значения `DATABASE_URL`, `HTTP_ADDR`, `GRPC_ADDR` и `CORS_ORIGIN` также можно переопределить переменными окружения. По умолчанию HTTP слушает `127.0.0.1:8080`, gRPC — `127.0.0.1:50051`, разрешённый браузерный origin — `http://127.0.0.1:8000`.

В примере Compose пользователь, пароль и база PostgreSQL равны `blog`. Эти значения предназначены только для локальной разработки.

## Запуск server

```bash
cargo run -p blog-server
```

При запуске сервер создаёт пул SQLx и автоматически применяет две миграции: `users` и `posts`. HTTP и gRPC работают одновременно.

## Запуск CLI

Клиентскую библиотеку можно собрать отдельно: `cargo build -p blog-client`. CLI собирается командой `cargo build -p blog-cli`. Следующие команды выполняйте из корня проекта в другом терминале:

```bash
cargo run -p blog-cli -- register --username ivan --email ivan@example.com --password secret123
cargo run -p blog-cli -- create --title "Первый пост" --content "Текст публикации"
cargo run -p blog-cli -- list --limit 20 --offset 0
cargo run -p blog-cli -- get --id 1
cargo run -p blog-cli -- update --id 1 --title "Новый заголовок"
cargo run -p blog-cli -- delete --id 1
```

`register` и `login` сохраняют JWT в `.blog_token` в текущем каталоге. Другие команды автоматически читают его. Файл исключён из Git; в Unix-подобных системах CLI устанавливает для него права `0600`. Для другого пользователя выполните `login`, чтобы заменить токен.

Для gRPC добавьте глобальный флаг `--grpc` до или после команды:

```bash
cargo run -p blog-cli -- list --grpc
cargo run -p blog-cli -- create --title "Через gRPC" --content "Второй транспорт" --grpc
```

Флаг `--server` меняет адрес, например `--server http://127.0.0.1:8080` для HTTP или `--server http://127.0.0.1:50051` для gRPC.

## HTTP API

Регистрация возвращает `201` и JSON с `token` и `user`; пароль и его хеш в ответ не попадают. Вход возвращает `200` или `401` при неверных данных. Повторная регистрация возвращает `409`.

```bash
curl -i -X POST http://127.0.0.1:8080/api/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"anna","email":"anna@example.com","password":"secret123"}'
curl -i http://127.0.0.1:8080/api/posts
curl -i http://127.0.0.1:8080/api/posts/1
```

Для защищённых запросов возьмите `token` из ответа регистрации или входа:

```bash
curl -i -X POST http://127.0.0.1:8080/api/posts \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer <token>' \
  -d '{"title":"Заметка","content":"Текст"}'
```

`PUT /api/posts/{id}` принимает JSON с `title` и `content`. `DELETE /api/posts/{id}` возвращает `204`. Список принимает `limit` (1–100, по умолчанию 10) и `offset` (неотрицательный, по умолчанию 0), а также отдаёт `total`.

## Запуск WASM

В корне workspace:

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build blog-wasm --target web --out-dir pkg
python3 -m http.server 8000 --bind 127.0.0.1
```

Откройте `http://127.0.0.1:8000`. Страница сразу загружает публичные посты. Через формы можно зарегистрироваться или войти; JWT хранится в `localStorage` браузера. После входа доступны публикация, редактирование и удаление собственных постов. Фронтенд обращается к HTTP API на `http://127.0.0.1:8080`; если меняете порт или адрес сервера, скорректируйте создание `BlogApp` в `app.js` и `CORS_ORIGIN` сервера.

## Проверки

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p blog-wasm --target wasm32-unknown-unknown
```

GitHub Actions запускает форматирование и Clippy при `push` и `pull_request`. Тесты запускаются только после успешного прохождения lint.

Для ручной проверки: зарегистрируйте двух пользователей, создайте пост первым и попробуйте обновить/удалить его токеном второго. Ожидается `403` в HTTP и `PERMISSION_DENIED` в gRPC. Запросы без токена возвращают `401` и `UNAUTHENTICATED`. При отключённой PostgreSQL сервер не запускается и выводит ошибку подключения.

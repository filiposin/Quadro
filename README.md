# Quadcopter Web Sim

3D веб‑симулятор квадрокоптера на **Rust + WebAssembly**.

В проекте уже есть:

- объёмная 3D‑сцена в браузере
- физика полёта на Rust/WASM
- серый квадрокоптер из простых 3D‑моделей
- зелёная земля, дороги и хрущёвки
- текстуры зданий с fallback на локально сгенерированные
- минимальный интерфейс без HUD
- готовый GitHub Actions workflow для публикации на **GitHub Pages**

## Управление

- `W / S` — тангаж (вперёд / назад)
- `A / D` — крен (влево / вправо)
- `Q / E` — рысканье
- `R / F` — увеличить / уменьшить тягу
- `P` — пауза
- `Space` — сброс симуляции

## Структура проекта

```text
quadcopter_web_sim/
├─ .github/workflows/deploy-pages.yml   # автодеплой на GitHub Pages
├─ docs/
│  ├─ index.html                        # входная HTML-страница
│  ├─ styles.css                        # стили
│  ├─ app.js                            # Three.js сцена и рендер
│  └─ .nojekyll                         # отключение Jekyll
├─ src/
│  └─ lib.rs                            # Rust/WASM логика симулятора
├─ Cargo.toml
├─ README.md
└─ GITHUB_PAGES_RU.md                   # пошаговая инструкция по заливке
```

## Локальный запуск

### 1) Установи Rust

Установи Rust через `rustup`.

### 2) Добавь wasm target

```bash
rustup target add wasm32-unknown-unknown
```

### 3) Установи wasm-pack

```bash
cargo install wasm-pack
```

### 4) Собери проект

```bash
wasm-pack build --release --target web --out-dir docs/pkg
```

### 5) Открой через локальный сервер

Важно: `index.html` лучше открывать **не напрямую двойным кликом**, а через локальный HTTP‑сервер.

Пример на Python:

```bash
python3 -m http.server 8080 --directory docs
```

После этого открой в браузере:

```text
http://localhost:8080
```

## Публикация на GitHub Pages

Подробная русская инструкция лежит в файле:

**`GITHUB_PAGES_RU.md`**

Коротко:

1. Создай репозиторий на GitHub.
2. Залей в него все файлы проекта.
3. В репозитории открой **Settings → Pages**.
4. В поле **Source** выбери **GitHub Actions**.
5. Запушь в ветку `main`.
6. Дождись завершения workflow `Deploy GitHub Pages`.
7. Открой ссылку вида:
   - `https://USERNAME.github.io/REPO-NAME/`

## Что можно улучшить дальше

- добавить режим FPV
- сделать карту препятствий
- добавить PID‑настройки в UI
- сделать переключение камер
- добавить посадку на площадку и очки
- вынести параметры дрона в меню

## Лицензия

MIT

# Как залить проект на GitHub и открыть сайт через GitHub Pages

Ниже два способа:

- **Способ A** — через сайт GitHub в браузере
- **Способ B** — через `git` в терминале

---

## Способ A — загрузка через браузер GitHub

### Шаг 1. Создай репозиторий

1. Зайди на GitHub.
2. Нажми **New repository**.
3. Назови репозиторий, например:

```text
quadcopter-web-sim
```

4. Сделай репозиторий **Public**.
5. Нажми **Create repository**.

---

### Шаг 2. Загрузи файлы проекта

Внутри нового репозитория:

1. Нажми **Add file** → **Upload files**.
2. Перетащи в окно **всю папку проекта целиком**:
   - `.github/`
   - `docs/`
   - `src/`
   - `Cargo.toml`
   - `README.md`
   - `GITHUB_PAGES_RU.md`
3. Нажми **Commit changes**.

> Если GitHub не даёт удобно загрузить структуру папок через браузер, проще использовать **Способ B** ниже.

---

### Шаг 3. Включи GitHub Pages

1. Открой вкладку **Settings**.
2. Слева открой **Pages**.
3. В блоке **Build and deployment** в поле **Source** выбери:

```text
GitHub Actions
```

Ничего больше вручную настраивать не нужно: workflow уже лежит в `.github/workflows/deploy-pages.yml`.

---

### Шаг 4. Дождись сборки

1. Открой вкладку **Actions**.
2. Найди workflow **Deploy GitHub Pages**.
3. Дождись статуса **Success**.

---

### Шаг 5. Открой сайт

После успешной сборки сайт будет доступен по адресу вида:

```text
https://YOUR-USERNAME.github.io/YOUR-REPOSITORY/
```

Пример:

```text
https://ivanpetrov.github.io/quadcopter-web-sim/
```

Также ссылку можно открыть через:

- **Settings → Pages → Visit site**

---

## Способ B — загрузка через git в терминале

### Шаг 1. Создай пустой репозиторий на GitHub

1. Нажми **New repository**.
2. Назови его, например:

```text
quadcopter-web-sim
```

3. Сделай **Public**.
4. Нажми **Create repository**.

---

### Шаг 2. Открой терминал в папке проекта

Перейди в папку проекта:

```bash
cd quadcopter_web_sim
```

---

### Шаг 3. Инициализируй git и сделай первый коммит

```bash
git init
git add .
git commit -m "Initial commit: Rust WebAssembly quadcopter sim"
```

---

### Шаг 4. Подключи удалённый репозиторий

Подставь свой URL вместо примера:

```bash
git branch -M main
git remote add origin https://github.com/YOUR-USERNAME/quadcopter-web-sim.git
git push -u origin main
```

Если GitHub попросит авторизацию — войди через браузер или используй Personal Access Token.

---

### Шаг 5. Включи GitHub Pages

1. Открой репозиторий на GitHub.
2. Перейди в **Settings → Pages**.
3. В **Build and deployment** выбери:

```text
Source = GitHub Actions
```

---

### Шаг 6. Дождись deploy

После пуша GitHub сам запустит workflow:

```text
Deploy GitHub Pages
```

Открой **Actions** и дождись статуса **Success**.

---

### Шаг 7. Открой сайт

Адрес будет такой:

```text
https://YOUR-USERNAME.github.io/quadcopter-web-sim/
```

---

## Если сайт не открылся

### 1) Проверь, что включён именно GitHub Actions

Открой:

```text
Settings → Pages
```

И убедись, что там стоит:

```text
Source = GitHub Actions
```

### 2) Проверь, что workflow завершился успешно

Открой:

```text
Actions → Deploy GitHub Pages
```

Если там ошибка, раскрой шаги и посмотри, на каком этапе упало.

### 3) Проверь ветку

Workflow настроен на запуск из:

```text
main
```

Если ты пушишь не в `main`, публикация не стартует.

### 4) Подожди пару минут

Иногда GitHub Pages публикует сайт не мгновенно.

---

## Как обновлять сайт дальше

После первого деплоя всё просто:

```bash
git add .
git commit -m "Update simulator"
git push
```

GitHub автоматически пересоберёт и обновит сайт.

---

## Важно

Файл `docs/index.html` уже настроен на загрузку WASM‑пакета из:

```text
docs/pkg/
```

Этот пакет собирается workflow автоматически командой:

```bash
wasm-pack build --release --target web --out-dir docs/pkg
```

То есть вручную загружать `pkg` на GitHub **не обязательно**, если ты используешь встроенный workflow из проекта.

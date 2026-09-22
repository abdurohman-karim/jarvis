# Анализ проекта и план улучшений

> ✅ — сделано (блоки «стабилизация» и «отзывчивость», 2026-09-22).
>
> **Замеры wake-word (2026-09-22, живой голос + release-сборка):** Vosk 10/10 recall при 7.2% CPU, Rustpotter 14/19 при 10.1%. Понижать порог Rustpotter нельзя — фоновый шум даёт score 0.59–0.60, ровно в диапазоне пропущенных фраз. Дефолт остаётся Vosk.
>
> Открытия по ходу работ: **Rustpotter никогда не работал** — `process_samples` требует кадры ровно по 480 семплов, а recorder отдаёт 512 (тихий `None`). Исправлено (`ce81c45`), но дефолт остаётся Vosk до проверки качества `.rpw` на реальном голосе (пункт 2.1).

Дата: 2026-09-22. Прогон по `crates/*`, `frontend/src`, конфигам сборки.
Приоритет: **P0** — баг/блокер, **P1** — заметно влияет на UX или ресурсы, **P2** — качество/долг.
Оценка: S (< 1 ч), M (пол-дня), L (день+).

---

## 1. Баги, найденные при анализе

| # | Пр. | Оц. | Проблема | Где |
|---|-----|-----|----------|-----|
| ~~1.1~~ ✅ | P0 | S | **Настройки «Определение намерения», «Извлечение параметров» и «VAD» никогда не сохраняются.** GUI пишет ключи `selected_intent_recognition_engine`, `selected_slot_extraction_engine`, `vad`, а БД принимает только `intent_backend`, `slots_backend`, `vad_backend` → `db_write` возвращает `unknown setting`. Читается `selected_stt_engine` вместо `speech_to_text_engine`. | `frontend/src/routes/settings/index.svelte`, `crates/jarvis-core/src/db/structs.rs:47-125` |
| ~~1.2~~ ✅ | P0 | S | **Значения из GUI не совпадают с ожидаемыми бэкендом (регистр).** GUI шлёт `IntentClassifier`/`EmbeddingClassifier`, `Energy`/`Nnnoiseless`/`None`, `GLiNER`; бэкенд матчит `"intent-classifier"`, `"energy"`, `"none"` и т.д. строго по регистру → тихий fallback. | `intent.rs:22`, `audio_processing/vad.rs:24-46`, `slots.rs:22` |
| ~~1.3~~ ✅ | P0 | S | **Rustpotter грузит `.rpw` по пути относительно cwd**, а не `APP_DIR` → в `.app`-бандле (и при запуске не из корня репо) wake-word не инициализируется. | `crates/jarvis-core/src/listener/rustpotter.rs:20` |
| ~~1.4~~ ✅ | P0 | S | **Шумоподавление и нормализация громкости не влияют на распознавание.** `audio_processing::process()` возвращает обработанные семплы, но в Vosk/Rustpotter уходит сырой `frame_buffer`. Обработка применяется только к VAD — двойная работа без пользы. | `crates/jarvis-app/src/app.rs:63-77, 85-88, 172-190` |
| ~~1.5~~ ✅ | P1 | S | Пакет `resources/commands/weather` не парсится (`phrases` как список вместо таблицы по языкам) — команда молча не загружается. | `resources/commands/weather/command.toml:27` |
| ~~1.6~~ ✅ | P1 | S | Изменённые в GUI настройки не применяются к работающему `jarvis-app` (читает БД только на старте), UI об этом не сообщает. Нужен либо IPC-`reload`, либо баннер «перезапустите ассистента». | `crates/jarvis-app/src/main.rs:131` (`ReloadCommands` — TODO) |
| ~~1.7~~ ✅ | P1 | S | `SetMuted` и `ReloadCommands` в IPC приняты, но не реализованы (`TODO`). | `main.rs:131-138` |
| ~~1.8~~ ✅ | P2 | S | Таймаут Lua-скрипта работает только на инструкциях Lua: блокирующие `jarvis.system.exec` / `jarvis.http.*` внутри Rust-функций таймаут не прерывает. | `crates/jarvis-core/src/lua/engine.rs:120-130` |
| ~~1.9~~ ✅ | P2 | S | `find_jarvis_app_pid` ищет процесс по `name.contains("jarvis-app")` — совпадёт с любым похожим процессом; GUI сам запускает дочерний процесс и мог бы хранить PID. | `crates/jarvis-gui/src/tauri_commands/sys.rs:26-34` |

## 2. Производительность: горячий аудио-цикл

Цикл `main_loop` крутится ~31 раз/с (512 семплов при 16 кГц). Всё, что делается в нём, умножается на 31.

| # | Пр. | Оц. | Проблема | Предложение |
|---|-----|-----|----------|-------------|
| ~~2.1~~ ❌ | P1 | M | **Проверено замерами 2026-09-22 — гипотеза не подтвердилась.** Vosk как wake-word использует грамматику из 9 состояний, это дёшево. В release-сборке во время речи: **Vosk 7.2% CPU против Rustpotter 10.1%**; в простое оба ~2.7% (движок вообще не получает аудио, пока VAD молчит). Recall на живом голосе: **Vosk 10/10 (100%), Rustpotter 14/19 (74%)**. Дефолт остаётся Vosk; Rustpotter починен (`ce81c45`) и доступен как альтернатива. |
| ~~2.2~~ ✅ | P1 | L | **Выполнение команды блокирует аудио-поток.** `execute_command` → Lua/CLI/HTTP синхронно внутри цикла чтения микрофона; pvrecorder за это время переполняет буфер, кадры теряются (`Failed to read audio frame`). Точно так же `voices::play_goodbye` и `intent::classify` (`rt.block_on`) в цикле. | Разнести на потоки: capture-поток → канал кадров → поток обработки (VAD/wake/STT) → отдельный executor команд (tokio task или thread pool). Состояние «выполняется команда» отдавать в GUI. |
| 2.3 | P1 | S | Аллокации в каждом кадре: `frame.to_vec()` в `AudioRingBuffer::push`, `samples: after_ns.to_vec()` в `ProcessedAudio`, `drain_all()` собирает `Vec<Vec<i16>>`. При включённом gain/NS — ещё 2 копии. | Ring buffer на `Vec<i16>` фиксированного размера с индексом записи; `ProcessedAudio` — `Cow`/ссылка либо запись in-place в переданный буфер. |
| ~~2.4~~ ✅ | P1 | S | **Energy-VAD с фиксированным порогом** (`VAD_ENERGY_THRESHOLD`): в тихой комнате/на тихом микрофоне речь не детектится, в шумной — постоянные false-positive и лишние прогоны Vosk. | Адаптивный порог: скользящий noise floor (EMA по RMS тишины) × коэффициент; гистерезис на включение/выключение. |
| 2.5 | P2 | S | `config::get_wake_phrases` / `get_phrases_to_remove` / `i18n::get_language()` (RwLock + clone `String`) вызываются в цикле при каждом результате распознавания. | Кэшировать на время сессии; `get_language` → `Arc<str>` или `Copy`-enum. |
| 2.6 | P2 | S | `listener/vosk.rs`: на каждый кандидат `chars().collect()` для всех wake-фраз заново. | Предвычислить `Vec<char>` wake-фраз при init. |
| 2.7 | P2 | S | `fetch_command` (levenshtein-fallback) делает `to_lowercase()` + `chars().collect()` для каждой фразы каждой команды при каждом вызове. | Нормализовать фразы при загрузке команд, хранить готовые `Vec<char>`/слова. |
| ~~2.8~~ ✅ | P2 | S | `kira::play_sound` читает и декодирует mp3/wav с диска при каждом воспроизведении (реплики играются постоянно). | Кэш `StaticSoundData` по пути (LRU или просто HashMap — файлов мало). |

## 3. Производительность: старт и GUI

| # | Пр. | Оц. | Проблема | Предложение |
|---|-----|-----|----------|-------------|
| ~~3.1~~ ✅ | P1 | S | **Старт `jarvis-app` ~5.5 с, из них ~5 с — pvrecorder.** Перечисление устройств через CoreAudio занимает ~2.4 с и вызывается минимум 3 раза за старт (`recorder::init` для лога, `get_selected_microphone_index` → `get_audio_devices`, снова в `start_recording`, затем `get_audio_device_name`). | Кэшировать список устройств (OnceCell/TTL), не перечислять для лога, если уровень не debug. Ожидаемо: старт < 2 с. |
| ~~3.2~~ ✅ | P1 | S | GUI дёргает `pv_get_audio_devices` (то же перечисление, 1–2 с) на странице настроек и в статистике, блокируя команду Tauri. | Тот же кэш + `#[tauri::command(async)]`. |
| ~~3.3~~ ✅ | P1 | S | `#[global_allocator] PeakAlloc` в GUI — атомарные счётчики на **каждую** аллокацию всего процесса ради `get_peak_ram_usage`, который нигде не показывается. | Удалить `peak_alloc`, `systemstat`, `lazy_static` (не используются), `get_cpu_temp`/`get_cpu_usage` (`sleep(200ms)` внутри команды). |
| ~~3.4~~ ✅ | P2 | S | `get_jarvis_app_stats` каждые 5 с делает `refresh_processes(All)` — полный скан таблицы процессов. | Хранить PID дочернего процесса (GUI его сам спавнит) и обновлять один процесс; проверять `running` через IPC-соединение. |
| ~~3.5~~ ✅ | P2 | S | Сохранение настроек — 12 отдельных `db_write`, каждый пишет JSON на диск (12 записей файла + 12 логов). | Один `db_write_many(map)` или batching в `SettingsManager` (dirty-flag + один flush). |
| ~~3.6~~ ✅ | P2 | S | Vosk-модель `en-us-0.22-lgraph` — 204 МБ и грузится ~0.3 с; для команд достаточно `small`-модели. | Использовать small-модели по умолчанию, большую — опционально. |
| 3.7 | P2 | S | `resources/vosk` копируется в `target/debug` целиком (`post_build.py`, 419 МБ) при каждой сборке в новой папке. | Symlink вместо копирования в dev-режиме. |

## 4. Архитектура и код

| # | Пр. | Оц. | Проблема | Предложение |
|---|-----|-----|----------|-------------|
| 4.1 | P1 | L | **Глобальное состояние через `OnceCell` + `Mutex` в каждом модуле** (`recorder`, `stt`, `listener`, `audio_processing`, `intent`, `slots`, `voices`, `ipc`…). Невозможно переинициализировать (сменить микрофон/модель без рестарта), сложно тестировать, порядок init зашит в `main.rs`. | Собрать в `struct Assistant { recorder, stt, wake, vad, intent, … }` с явным `Config` → `build()`. Модули — обычные структуры с методами; глобальные `static` только для логгера. |
| ~~4.2~~ ✅ | P1 | M | Дублирование логики между `jarvis-app`, `jarvis-cli` и мёртвыми файлами: `_app.rs`, `_main.rs`, `_listener.rs` (415 строк Porcupine), `recorder/cpal.rs`, `recorder/portaudio.rs` (не подключены, `todo!()`/`panic!()` ветки в `recorder.rs`). | Удалить мёртвый код; `RecorderType` свести к реальному (pvrecorder) или вынести за trait `Recorder` без `todo!()`. |
| 4.3 | P1 | M | Тестов нет (кроме `lua/tests.rs`). Нет теста на матчинг команд, VAD, парсинг `command.toml`, IPC-протокол, маппинг ключей настроек (баг 1.1 поймался бы таблицей ключей). | Минимум: unit-тесты `commands::fetch_command`, `Settings::get/set` round-trip по `Settings::keys()`, парсинг всех пакетов из `resources/commands` (ловит 1.5), `AudioRingBuffer`. |
| 4.4 | P2 | S | Настройки-строки вместо enum (`intent_backend: String`, `vad_backend: String`, `slots_backend: String`) — источник багов 1.1/1.2. | `#[derive(Serialize, Deserialize)] enum` с `#[serde(rename_all = "kebab-case")]`, единый источник правды для GUI (отдавать список вариантов из бэкенда через tauri-команду). |
| ~~4.5~~ ✅ | P2 | S | `unwrap()`/`expect()` на путях, зависящих от окружения (`DB.get().unwrap()`, `RUSTPOTTER.get().unwrap()`, `exe_path.to_str().unwrap()`), `panic!` в `read_microphone`. Паника в аудио-потоке = молчаливая смерть ассистента при живом трее. | `Result` + логирование; в `main` — `catch_unwind` вокруг цикла с уведомлением в GUI. |
| 4.6 | P2 | M | IPC-протокол: `IpcEvent`/`IpcAction` не версионированы, фронтенд дублирует типы вручную (`ipc.ts`). | Генерировать TS-типы из Rust (`ts-rs`/`specta`) или хотя бы один общий JSON-schema-тест. |
| 4.7 | P2 | S | `jarvis-cli` тянет `intent-classifier` и `tokio`, но не Vosk — фактически отдельный урезанный ассистент с собственным циклом (203 строки). | Либо сделать CLI тонким клиентом IPC (`jarvis-cli say "открой браузер"`, `status`, `stop`), либо убрать. |
| ~~4.8~~ ✅ | P2 | S | Команды `ahk` (Windows `.exe`) в дефолтных пакетах `browser`, `steam`, `volume` — на macOS/Linux «команда найдена», но spawn падает. | Добавить `platforms = ["windows"]` в `command.toml` и фильтровать при загрузке; для macOS — Lua-эквиваленты (`open -a`). |
| ~~4.9~~ ✅ | P2 | S | Фронтенд: `stopStatsPolling`/`enableIpc` вызываются из `subscribe` внутри страницы — при переходе между роутами IPC отключается (`onDestroy → disableIpc`) и переподключается через 5 с. | Держать IPC-соединение на уровне `App.svelte`, страницы только читают сторы. |

## 5. Безопасность

| # | Пр. | Оц. | Проблема | Предложение |
|---|-----|-----|----------|-------------|
| ~~5.1~~ ✅ | P1 | M | **IPC `ws://127.0.0.1:9712` без аутентификации.** Любой локальный процесс (или страница в браузере через WebSocket!) может отправить `text_command` и выполнить команду типа `cli` (`sh -c …`) или Lua с `sandbox = "full"`. | Токен: `jarvis-app` при старте пишет случайный токен в `APP_CONFIG_DIR/ipc.token` (0600), клиент передаёт его в первом сообщении/`Sec-WebSocket-Protocol`; проверять `Origin` (у браузера он есть, у Tauri — `tauri://localhost`). |
| ~~5.2~~ ✅ | P1 | S | Фиксированный порт — две копии `jarvis-app` конфликтуют (видел `Address already in use`), второй экземпляр живёт без IPC. | Single-instance lock (файл-лок в config dir) + при занятом порте — exit с понятной ошибкой. |
| 5.3 | P2 | S | Тип команды `cli` выполняет произвольную строку через `sh -c` из `command.toml`, `Lua full` даёт `exec` и `io`. Пакеты команд планируются как пользовательские/скачиваемые. | Подтверждение в GUI при первом запуске пакета с `cli`/`full`; whitelist бинарников; убрать `sh -c` в пользу `Command::new(cmd).args(args)`. |
| 5.4 | P2 | S | API-ключи (`api_key__openai`, `picovoice`) хранятся в `app.db` открытым JSON. | Keychain через `keyring` crate. |

## 6. Сборка и дистрибуция

| # | Пр. | Оц. | Проблема | Предложение |
|---|-----|-----|----------|-------------|
| ~~6.1~~ ✅ | P1 | S | Нет `[profile.release]`: без LTO/`codegen-units=1`/`strip` бинарники крупнее и медленнее (candle, ort, tokenizers — тяжёлые крейты). | `lto = "thin"`, `codegen-units = 1`, `strip = true`, `panic = "abort"` (после 4.5), `opt-level = 3`. |
| ~~6.2~~ ✅ | P1 | M | DMG 278 МБ, из них ~270 — три модели Vosk. Первый запуск на другом языке всё равно требует «своей» модели. | Загружать модели по требованию из GUI (уже есть `models/catalog.rs` и registry) с прогрессом; в бандл класть только `small`-модель текущего языка или ничего. |
| 6.3 | P1 | S | Сборка только `aarch64`; `libpv_recorder.dylib` в `tauri.macos.conf.json` захардкожен на arm64. | Universal-сборка: `tauri build --target universal-apple-darwin`, `lipo` для pvrecorder (x86_64 dylib есть в `vendor/`). |
| ~~6.4~~ ✅ | P2 | M | Нет CI: сборка/тесты/clippy не проверяются, Windows-путь (`post_build.py`, `lib/windows`) наверняка уже расходится с macOS-изменениями. | GitHub Actions: `cargo clippy -D warnings`, `cargo test`, `svelte-check`, `tauri build` на macOS + Windows matrix. |
| ~~6.5~~ ✅ | P2 | S | 40+ warnings в `cargo build` (unused imports, unreachable patterns) маскируют новые. | `cargo fix`, затем `#![deny(warnings)]` в CI. |
| 6.6 | P2 | S | Нотаризация: ad-hoc подпись, Gatekeeper ругается. | Когда появится Developer ID — `signingIdentity` + `notarize` в конфиге Tauri, всё остальное уже готово. |

## 7. Рекомендуемый порядок

1. **Неделя 1 — стабилизация (все S):** 1.1, 1.2, 1.3, 1.4, 1.5, 3.1, 3.3, 5.2, 6.1, 4.3 (тесты на ключи настроек и парсинг команд, чтобы это не вернулось).
2. **Неделя 2 — отзывчивость:** 2.2 (разделение потоков), 2.1 (Rustpotter по умолчанию), 2.4 (адаптивный VAD), 1.6/1.7 (reload/mute по IPC), 5.1 (токен IPC).
3. **Далее — рефакторинг:** 4.1 (убрать глобальное состояние), 4.2 (мёртвый код), 4.4 (enum-настройки), 6.2 (модели по требованию), 6.4 (CI).

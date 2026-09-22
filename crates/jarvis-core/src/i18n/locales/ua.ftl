# ### APP INFO
app-name = JARVIS
app-description = Голосовий асистент

# ### TRAY MENU
tray-restart = Перезапустити
tray-settings = Налаштування
tray-exit = Вихід
tray-tooltip = JARVIS - Голосовий асистент
tray-language = Мова
tray-voice = Голос
tray-wake-word = Рушій детекції
tray-noise-suppression = Шумозаглушення
tray-vad = Детекцiя голосу (VAD)
tray-gain-normalizer = Нормалізація гучності

# ### HEADER

# ### SEARCH
search-placeholder = Введіть команду вручну або скажіть «Джарвіс» ...

# ### MAIN PAGE
assistant-not-running = АСИСТЕНТ НЕ ЗАПУЩЕНО
assistant-offline-hint = Налаштувати його можна не запускаючи.
btn-start = ЗАПУСТИТИ
btn-starting = ЗАПУСК...

# ### STATUS
status-disconnected = Відключено
status-standby = Очікування
status-listening = Слухаю...
status-processing = Обробка...

# ### STATS
stats-microphone = МІКРОФОН
stats-neural-networks = НЕЙРОМЕРЕЖІ
stats-resources = РЕСУРСИ
stats-system-default = Системний
stats-not-selected = Не вибрано
stats-loading = Завантаження...


# ### SETTINGS
settings-title = Налаштування
settings-general = Основні
settings-devices = Пристрої
settings-neural-networks = Нейромережі
settings-audio = Аудіо
settings-recognition = Розпізнавання
settings-about = Про програму
settings-language = Мова
settings-microphone = Мікрофон
settings-microphone-desc = Його буде слухати асистент.
settings-mic-default = За замовчуванням (Система)
settings-voice = Голос асистента
settings-voice-desc =
    Не всі команди працюють з усіма звуковими пакетами.
    Натисніть, щоб прослухати як звучить голос.
settings-wake-word-engine = Рушій активації
settings-wake-word-desc = Виберіть нейромережу для розпізнавання активаційної фрази.
settings-stt-engine = Розпізнавання мовлення
settings-intent-engine = Визначення наміру
settings-intent-engine-desc = Виберіть нейромережу для розпізнавання команд.
settings-noise-suppression = Шумозаглушення
settings-noise-suppression-desc = Зменшує фоновий шум. Може негативно впливати на розпізнавання.
settings-vad = Визначення голосу (VAD)
settings-vad-desc = Пропускає тишу, економить ресурси CPU.
settings-gain-normalizer = Нормалізація гучності
settings-gain-normalizer-desc = Автоматично регулює рівень гучності.
settings-api-keys = API Ключі
settings-save = Зберегти
settings-cancel = Скасувати
settings-back = Назад
settings-enabled = Увімкнено
settings-disabled = Вимкнено


# settings - picovoice
settings-picovoice-warning = Ця нейромережа працює не у всіх!
settings-picovoice-key-desc = Введіть сюди свій ключ Picovoice. Він видається безкоштовно при реєстрації в
settings-picovoice-key = Ключ Picovoice

# settings - vosk
settings-auto-detect = Авто-визначення
settings-vosk-model = Модель розпізнавання мовлення (Vosk)
settings-vosk-model-desc =
    Виберіть модель Vosk для розпізнавання мовлення.
    Ви можете завантажити моделі тут: https://alphacephei.com/vosk/models
settings-models-not-found = Моделі не знайдено

# settings - openai
settings-openai-key = Ключ OpenAI
settings-openai-not-supported = Наразі ChatGPT не підтримується. Він буде доданий у наступних оновленнях.

# ### COMMANDS PAGE
commands-title = Команди
commands-search = Пошук команд...
commands-count = { $count } команд

# ### ERRORS
error-generic = Сталася помилка
error-connection = Помилка підключення
error-not-found = Не знайдено

# ### NOTIFICATIONS
notification-saved = Налаштування збережено!
notification-error = Помилка
notification-assistant-started = Асистент запущено
notification-assistant-stopped = Асистент зупинено

# SLOTS EXTRACTION
settings-slot-engine = Витяг параметрів
settings-slot-engine-desc = Витягує параметри з голосових команд (напр. назва міста, число).
settings-gliner-model = Модель GLiNER ONNX
settings-gliner-model-desc = 
    Оберіть варіант моделі.
    Квантизовані моделі (int8, uint8) швидші, але менш точні.
settings-gliner-models-hint = Моделі GLiNER не знайдено.

# ETC
search-error-not-running = Асистент не запущено
search-error-failed = Не вдалося виконати команду
settings-no-voices = Голоси не знайдено

# ### UI (redesign)
nav-assistant = Асистент
nav-commands = Команди
nav-settings = Налаштування
assistant-subtitle = Голосове керування комп'ютером
assistant-ready = Готовий до роботи
assistant-ready-hint = Скажіть «Джарвіс» або введіть команду нижче.
assistant-last-heard = Остання розпізнана фраза
status-offline = Не запущено
status-connecting = Підключення…
btn-stop = Зупинити
btn-stopping = Зупинка…
settings-subtitle = Голос, пристрої та нейромережі
settings-diagnostics = Діагностика
settings-logs = Логи
settings-open-logs = Відкрити теку з логами
commands-empty-title = Команди не знайдено
commands-empty-desc = Помістіть пакети команд у теку resources/commands.
commands-no-results = Нічого не знайдено
status-muted = Мікрофон вимкнено
btn-mute = Вимкнути мікрофон
btn-unmute = Увімкнути мікрофон
btn-restart = Перезапустити
btn-restarting = Перезапуск…
commands-reload = Оновити
settings-vosk-catalog = Доступні моделі
settings-vosk-catalog-desc = Моделі завантажуються з alphacephei.com у теку даних застосунку.
settings-models-hint = Завантажте модель зі списку нижче — без неї асистент не запуститься.
models-download = Завантажити
models-downloading = Завантаження…
models-extracting = Розпакування…
models-delete = Видалити
models-installed = Встановлена
models-bundled = Вбудована
models-error = Помилка
assistant-no-model = Немає моделі розпізнавання
assistant-no-model-hint = Завантажте модель Vosk для вашої мови в налаштуваннях.
btn-get-model = Завантажити модель
settings-applying = Застосовую налаштування…
settings-applied = Налаштування застосовано
settings-restart-hint = Асистент не відповів — зміни застосуються після перезапуску
settings-ai = Відповіді AI
settings-ai-desc = Коли жодна команда не підійшла, питання йде до Google Gemini.
settings-ai-key = Ключ Gemini
settings-ai-key-desc = Отримати можна на aistudio.google.com. Зберігається локально.
settings-ai-model = Модель
settings-ai-model-desc = Типово gemini-3.5-flash.
settings-ai-fallback = Відповідати на питання
settings-ai-fallback-desc = Питати модель, якщо команду не знайдено.
settings-ai-speak = Озвучувати відповіді
settings-ai-speak-desc = Читати відповідь системним голосом.
models-recommended = радимо
settings-vosk-accuracy-title = У шумі модель помиляється частіше
settings-tts-engine = Чим озвучувати
settings-tts-engine-desc = Заздалегідь записані фрази завжди звучать голосом асистента; для решти обирається рушій.
settings-tts-system = Системний голос (миттєво)
settings-tts-clone = Голос Джарвіса (кілька секунд)
settings-tts-clone-title = Голос Джарвіса можна увімкнути
settings-tts-clone-hint = Компонент клонування голосу не встановлено. Запустіть scripts/voice-clone/install.sh — це близько 4 ГБ і не входить у застосунок.
settings-vosk-accuracy-desc = Встановлено лише компактну модель. У замірах із фоновим шумом велика модель давала у 2.5 раза менше помилок. Слово активації й надалі розпізнаватиме мала — не видаляйте її.

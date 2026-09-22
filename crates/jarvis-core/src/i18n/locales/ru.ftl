# APP INFO
app-name = JARVIS
app-description = Голосовой ассистент

# TRAY MENU
tray-restart = Перезапустить
tray-settings = Настройки
tray-exit = Выход
tray-tooltip = JARVIS - Голосовой ассистент
tray-language = Язык
tray-voice = Голос
tray-wake-word = Движок wake-word
tray-noise-suppression = Шумоподавление
tray-vad = Детекция голоса (VAD)
tray-gain-normalizer = Нормализация громкости

# HEADER

# SEARCH
search-placeholder = Введите команду вручную или произнесите «Джарвис» ...

# MAIN PAGE
assistant-not-running = АССИСТЕНТ НЕ ЗАПУЩЕН
assistant-offline-hint = Настроить его можно не запуская.
btn-start = ЗАПУСТИТЬ
btn-starting = ЗАПУСК...

# STATUS
status-disconnected = Отключен
status-standby = Ожидание
status-listening = Слушаю...
status-processing = Обработка...

# STATS
stats-microphone = МИКРОФОН
stats-neural-networks = НЕЙРОСЕТИ
stats-resources = РЕСУРСЫ
stats-system-default = Системный
stats-not-selected = Не выбран
stats-loading = Загрузка...


# SETTINGS
settings-title = Настройки
settings-general = Основные
settings-devices = Устройства
settings-neural-networks = Нейросети
settings-audio = Аудио
settings-recognition = Распознавание
settings-about = О программе
settings-language = Язык
settings-microphone = Микрофон
settings-microphone-desc = Его будет слушать ассистент.
settings-mic-default = По умолчанию (Система)
settings-voice = Голос ассистента
settings-voice-desc =
    Не все команды работают со всеми звуковыми пакетами.
    Кликните, чтобы прослушать как звучит голос.
settings-wake-word-engine = Движок активации
settings-wake-word-desc = Выберите нейросеть для распознавания активационной фразы.
settings-stt-engine = Распознавание речи
settings-intent-engine = Определение намерения
settings-intent-engine-desc = Выберите нейросеть для распознавания команд.
settings-noise-suppression = Шумоподавление
settings-noise-suppression-desc = Уменьшает фоновый шум. Может негативно влиять на распознавание.
settings-vad = Определение голоса (VAD)
settings-vad-desc = Пропускает тишину, экономит ресурсы CPU.
settings-gain-normalizer = Нормализация громкости
settings-gain-normalizer-desc = Автоматически регулирует уровень громкости.
settings-api-keys = API Ключи
settings-save = Сохранить
settings-cancel = Отмена
settings-back = Назад
settings-enabled = Включено
settings-disabled = Отключено


# settings - picovoice
settings-picovoice-warning = Эта нейросеть работает не у всех!
settings-picovoice-key-desc = Введите сюда свой ключ Picovoice. Он выдается бесплатно при регистрации в
settings-picovoice-key = Ключ Picovoice

# settings - vosk
settings-auto-detect = Авто-определение
settings-vosk-model = Модель распознавания речи (Vosk)
settings-vosk-model-desc =
    Выберите модель Vosk для распознавания речи.
    Вы можете скачать модели здесь: https://alphacephei.com/vosk/models
settings-models-not-found = Модели не найдены

# settings - openai
settings-openai-key = Ключ OpenAI
settings-openai-not-supported = В данный момент ChatGPT не поддерживается. Он будет добавлен в ближайших обновлениях.

# COMMANDS PAGE
commands-title = Команды
commands-search = Поиск команд...
commands-count = { $count } команд

# ERRORS
error-generic = Произошла ошибка
error-connection = Ошибка подключения
error-not-found = Не найдено

# NOTIFICATIONS
notification-saved = Настройки сохранены!
notification-error = Ошибка
notification-assistant-started = Ассистент запущен
notification-assistant-stopped = Ассистент остановлен

# SLOTS EXTRACTION
settings-slot-engine = Извлечение параметров
settings-slot-engine-desc = Извлекает параметры из голосовых команд (напр. название города, число).
settings-gliner-model = Модель GLiNER ONNX
settings-gliner-model-desc =
    Выберите вариант модели.
    Квантизированные модели (int8, uint8) быстрее, но менее точны.
settings-gliner-models-hint = Модели GLiNER не найдены.

# ETC
search-error-not-running = Ассистент не запущен
search-error-failed = Не удалось выполнить команду
settings-no-voices = Голоса не найдены

# ### UI (redesign)
nav-assistant = Ассистент
nav-commands = Команды
nav-settings = Настройки
assistant-subtitle = Голосовое управление компьютером
assistant-ready = Готов к работе
assistant-ready-hint = Скажите «Джарвис» или введите команду ниже.
assistant-last-heard = Последняя распознанная фраза
status-offline = Не запущен
status-connecting = Подключение…
btn-stop = Остановить
btn-stopping = Остановка…
settings-subtitle = Голос, устройства и нейросети
settings-diagnostics = Диагностика
settings-logs = Логи
settings-open-logs = Открыть папку с логами
commands-empty-title = Команды не найдены
commands-empty-desc = Поместите пакеты команд в папку resources/commands.
commands-no-results = Ничего не найдено
status-muted = Микрофон выключен
btn-mute = Выключить микрофон
btn-unmute = Включить микрофон
btn-restart = Перезапустить
btn-restarting = Перезапуск…
commands-reload = Обновить
settings-vosk-catalog = Доступные модели
settings-vosk-catalog-desc = Модели скачиваются с alphacephei.com и хранятся в папке данных приложения.
settings-models-hint = Скачайте модель из списка ниже — без неё ассистент не запустится.
models-download = Скачать
models-downloading = Загрузка…
models-extracting = Распаковка…
models-delete = Удалить
models-installed = Установлена
models-bundled = Встроенная
models-error = Ошибка
assistant-no-model = Нет модели распознавания
assistant-no-model-hint = Скачайте модель Vosk для вашего языка в настройках.
btn-get-model = Скачать модель
settings-applying = Применяю настройки…
settings-applied = Настройки применены
settings-restart-hint = Ассистент не ответил — изменения применятся после перезапуска
settings-ai = Ответы AI
settings-ai-desc = Когда ни одна команда не подошла, вопрос уходит в языковую модель Google Gemini.
settings-ai-key = Ключ Gemini
settings-ai-key-desc = Получить можно на aistudio.google.com. Хранится локально в файле настроек.
settings-ai-model = Модель
settings-ai-model-desc = По умолчанию gemini-3.5-flash.
settings-ai-fallback = Отвечать на вопросы
settings-ai-fallback-desc = Спрашивать модель, если команда не найдена.
settings-ai-speak = Озвучивать ответы
settings-ai-speak-desc = Читать ответ вслух.
models-recommended = советуем
settings-vosk-accuracy-title = В шуме модель ошибается чаще
settings-tts-engine = Чем озвучивать
settings-tts-engine-desc = Заранее сгенерированные фразы — а это почти все ответы команд, включая время, дату и заряд, — всегда звучат голосом ассистента. Движок озвучивает остальное.
settings-tts-system = Системный голос (мгновенно)
settings-tts-clone = Голос Джарвиса (несколько секунд)
settings-tts-clone-title = Голос Джарвиса можно включить
settings-tts-clone-hint = Компонент клонирования голоса не установлен. Запустите scripts/voice-clone/install.sh — он поставит около 4 ГБ и не входит в приложение. После этого ассистент сможет произносить любой текст своим голосом.
settings-vosk-accuracy-desc = Сейчас установлена только компактная модель. На замерах с фоновым шумом большая русская модель давала в 2.5 раза меньше ошибок. Слово активации при этом продолжит распознавать маленькая — большие модели для этого не подходят, поэтому не удаляйте её.

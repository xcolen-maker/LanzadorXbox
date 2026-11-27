//! Idiomas de la interfaz. Los textos del código están en español y son la clave:
//! `tr("Instalar")` devuelve la traducción al idioma elegido, o el mismo texto si no hay.
//! `trf!("Instalando {}", nombre)` hace lo mismo con textos que llevan datos (cada `{}`
//! se reemplaza en orden). Las traducciones están en `ENTRIES`, al final del archivo.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Es,
    En,
    Pt,
    Fr,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 5] = [Lang::Es, Lang::En, Lang::Pt, Lang::Fr, Lang::Ru];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
            Lang::Pt => "pt",
            Lang::Fr => "fr",
            Lang::Ru => "ru",
        }
    }

    /// El nombre del idioma en ese mismo idioma, para el selector.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
            Lang::Pt => "Português",
            Lang::Fr => "Français",
            Lang::Ru => "Русский",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize % Lang::ALL.len()]
}

/// El idioma de Windows, si es uno de los que hay; si no, inglés.
pub fn system() -> Lang {
    #[cfg(windows)]
    {
        // SAFETY: no recibe nada y solo devuelve un número.
        let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
        match id & 0x3ff {
            0x0a => Lang::Es,
            0x16 => Lang::Pt,
            0x0c => Lang::Fr,
            0x19 => Lang::Ru,
            _ => Lang::En,
        }
    }
    #[cfg(not(windows))]
    {
        Lang::En
    }
}

/// El idioma de los ajustes: un código ("en", "fr"…) o vacío para usar el de Windows.
pub fn resolve(code: &str) -> Lang {
    Lang::from_code(code).unwrap_or_else(system)
}

static TABLE: LazyLock<HashMap<&'static str, [&'static str; 4]>> = LazyLock::new(|| ENTRIES.iter().copied().collect());

/// La traducción de un texto en español al idioma elegido (o el mismo texto si no hay).
pub fn tr(es: &str) -> &str {
    match current() {
        Lang::Es => es,
        l => TABLE.get(es).map_or(es, |t| t[l as usize - 1]),
    }
}

/// Reemplaza cada `{}` del texto por el siguiente dato.
pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut args = args.iter();
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// `trf!("Instalando {}", nombre)`: traduce y completa con los datos.
macro_rules! trf {
    ($s:expr $(, $a:expr)* $(,)?) => {
        crate::i18n::fill(crate::i18n::tr($s), &[$(&$a as &dyn ::std::fmt::Display),*])
    };
}

/// Español → [inglés, portugués, francés, ruso].
const ENTRIES: &[(&str, [&str; 4])] = &[
    ("Todavía no está instalado", ["Not installed yet", "Ainda não instalado", "Pas encore installé", "Ещё не установлен"]),
    ("Descargado, falta compilarlo", ["Downloaded, needs building", "Baixado, falta compilar", "Téléchargé, à compiler", "Загружен, нужно собрать"]),
    ("Hay {} · tenés la versión de {}", ["{} available · you have the version from {}", "{} disponíveis · você tem a versão de {}", "{} disponibles · vous avez la version du {}", "{} · у вас версия от {}"]),
    ("Hay cambios descargados sin compilar", ["There are downloaded changes not built yet", "Há mudanças baixadas sem compilar", "Des changements téléchargés ne sont pas compilés", "Есть загруженные, но не собранные изменения"]),
    ("Versión de {}", ["Version from {}", "Versão de {}", "Version du {}", "Версия от {}"]),
    ("1 cambio nuevo", ["1 new change", "1 mudança nova", "1 nouveau changement", "1 новое изменение"]),
    ("{} cambios nuevos", ["{} new changes", "{} mudanças novas", "{} nouveaux changements", "новые изменения ({})"]),
    ("fecha desconocida", ["unknown date", "data desconhecida", "date inconnue", "дата неизвестна"]),
    ("hoy, {}", ["today, {}", "hoje, {}", "aujourd'hui, {}", "сегодня, {}"]),
    ("ayer, {}", ["yesterday, {}", "ontem, {}", "hier, {}", "вчера, {}"]),
    ("original (Marvisak)", ["original (Marvisak)", "original (Marvisak)", "original (Marvisak)", "оригинал (Marvisak)"]),
    ("versión mejorada (fxceless)", ["improved version (fxceless)", "versão melhorada (fxceless)", "version améliorée (fxceless)", "улучшенная версия (fxceless)"]),
    ("para NVIDIA (trustno1-coder)", ["for NVIDIA (trustno1-coder)", "para NVIDIA (trustno1-coder)", "pour NVIDIA (trustno1-coder)", "для NVIDIA (trustno1-coder)"]),
    ("{} · agregado por vos", ["{} · added by you", "{} · adicionado por você", "{} · ajouté par vous", "{} · добавлен вами"]),
    ("Disponibles", ["Available", "Disponíveis", "Disponibles", "Доступные"]),
    ("Agregar emulador", ["Add emulator", "Adicionar emulador", "Ajouter un émulateur", "Добавить эмулятор"]),
    ("Mi biblioteca", ["My library", "Minha biblioteca", "Ma bibliothèque", "Моя библиотека"]),
    ("Nada instalado todavía", ["Nothing installed yet", "Nada instalado ainda", "Rien d'installé pour l'instant", "Пока ничего не установлено"]),
    ("Revisando requisitos…", ["Checking requirements…", "Verificando requisitos…", "Vérification des prérequis…", "Проверка требований…"]),
    ("Todo listo para jugar", ["Ready to play", "Tudo pronto para jogar", "Prêt à jouer", "Всё готово к игре"]),
    ("Faltan requisitos", ["Missing requirements", "Faltam requisitos", "Prérequis manquants", "Не хватает компонентов"]),
    ("Abrir la pestaña Instalar", ["Open the Install tab", "Abrir a aba Instalar", "Ouvrir l'onglet Installer", "Открыть вкладку «Установка»"]),
    ("Carpeta de datos", ["Data folder", "Pasta de dados", "Dossier de données", "Папка данных"]),
    ("JUEGO", ["GAME", "JOGO", "JEU", "ИГРА"]),
    ("AJUSTES", ["SETTINGS", "AJUSTES", "PARAMÈTRES", "НАСТРОЙКИ"]),
    ("Ajustes", ["Settings", "Ajustes", "Paramètres", "Настройки"]),
    ("Instalar", ["Install", "Instalar", "Installer", "Установить"]),
    ("No hay emuladores configurados", ["No emulators configured", "Nenhum emulador configurado", "Aucun émulateur configuré", "Эмуляторы не настроены"]),
    ("con {}", ["with {}", "com {}", "avec {}", "с {}"]),
    ("COMPILAR E INSTALAR", ["BUILD AND INSTALL", "COMPILAR E INSTALAR", "COMPILER ET INSTALLER", "СОБРАТЬ"]),
    ("INSTALAR", ["INSTALL", "INSTALAR", "INSTALLER", "УСТАНОВИТЬ"]),
    ("DETENER", ["STOP", "PARAR", "ARRÊTER", "ОСТАНОВИТЬ"]),
    ("JUGAR", ["PLAY", "JOGAR", "JOUER", "ИГРАТЬ"]),
    ("EN EJECUCIÓN", ["RUNNING", "EM EXECUÇÃO", "EN COURS", "ЗАПУЩЕНО"]),
    ("ACTUALIZAR", ["UPDATE", "ATUALIZAR", "METTRE À JOUR", "ОБНОВИТЬ"]),
    ("BUSCANDO…", ["CHECKING…", "VERIFICANDO…", "RECHERCHE…", "ПРОВЕРКА…"]),
    ("BUSCAR ACTUALIZACIÓN", ["CHECK FOR UPDATES", "VERIFICAR ATUALIZAÇÃO", "VÉRIFIER LES MISES À JOUR", "ПРОВЕРИТЬ ОБНОВЛЕНИЯ"]),
    ("¿Problemas al instalar?  ›", ["Trouble installing?  ›", "Problemas ao instalar?  ›", "Problèmes d'installation ?  ›", "Проблемы с установкой?  ›"]),
    ("Ya tenés una copia en {}", ["You already have a copy in {}", "Você já tem uma cópia em {}", "Vous avez déjà une copie dans {}", "У вас уже есть копия в {}"]),
    ("USAR ESA COPIA", ["USE THAT COPY", "USAR ESSA CÓPIA", "UTILISER CETTE COPIE", "ИСПОЛЬЗОВАТЬ ЕЁ"]),
    ("… y {} más", ["… and {} more", "… e mais {}", "… et {} de plus", "… и ещё {}"]),
    ("Ajustes propios: {} a media resolución, para NVIDIA", ["Own settings: {} at half resolution, for NVIDIA", "Ajustes próprios: {} em meia resolução, para NVIDIA", "Réglages propres : {} en demi-résolution, pour NVIDIA", "Свои настройки: {} в половинном разрешении, для NVIDIA"]),
    ("Modo seguro activado (se cambia en Ajustes)", ["Safe mode on (change it in Settings)", "Modo seguro ativado (mude em Ajustes)", "Mode sûr activé (modifiable dans Paramètres)", "Безопасный режим включён (меняется в настройках)"]),
    ("Consola: {} (se cambia en Ajustes)", ["Console: {} (change it in Settings)", "Console: {} (mude em Ajustes)", "Console : {} (modifiable dans Paramètres)", "Консоль: {} (меняется в настройках)"]),
    ("NO INSTALADO", ["NOT INSTALLED", "NÃO INSTALADO", "NON INSTALLÉ", "НЕ УСТАНОВЛЕН"]),
    ("NUEVA VERSIÓN", ["NEW VERSION", "NOVA VERSÃO", "NOUVELLE VERSION", "НОВАЯ ВЕРСИЯ"]),
    ("INSTALADO", ["INSTALLED", "INSTALADO", "INSTALLÉ", "УСТАНОВЛЕН"]),
    ("Idioma", ["Language", "Idioma", "Langue", "Язык"]),
    ("CARPETA DEL JUEGO", ["GAME FOLDER", "PASTA DO JOGO", "DOSSIER DU JEU", "ПАПКА ИГРЫ"]),
    ("Juego encontrado", ["Game found", "Jogo encontrado", "Jeu trouvé", "Игра найдена"]),
    ("No se encontró el juego", ["Game not found", "Jogo não encontrado", "Jeu introuvable", "Игра не найдена"]),
    ("Cambiar carpeta…", ["Change folder…", "Mudar pasta…", "Changer de dossier…", "Сменить папку…"]),
    ("Ocultar", ["Hide", "Ocultar", "Masquer", "Скрыть"]),
    ("Mostrar", ["Show", "Mostrar", "Afficher", "Показать"]),
    ("(sin elegir)", ["(not chosen)", "(não escolhida)", "(non choisi)", "(не выбрана)"]),
    ("Opciones de juego", ["Game options", "Opções de jogo", "Options de jeu", "Параметры игры"]),
    ("Consola que ve el juego:", ["Console the game sees:", "Console que o jogo vê:", "Console vue par le jeu :", "Консоль для игры:"]),
    ("Series S (menos memoria)", ["Series S (less memory)", "Series S (menos memória)", "Series S (moins de mémoire)", "Series S (меньше памяти)"]),
    ("Pulsar Enter solo para pasar las pantallas iniciales", ["Press Enter automatically to skip the intro screens", "Apertar Enter sozinho para passar as telas iniciais", "Appuyer sur Entrée automatiquement pour passer les écrans d'intro", "Нажимать Enter автоматически, чтобы пропустить заставки"]),
    ("Modo seguro · más lento, gasta menos memoria", ["Safe mode · slower, uses less memory", "Modo seguro · mais lento, usa menos memória", "Mode sûr · plus lent, utilise moins de mémoire", "Безопасный режим · медленнее, меньше памяти"]),
    ("La consola y el modo seguro valen para VIctory y su fork. VIctory NVIDIA usa siempre sus ajustes propios; xcl no los usa.", ["The console and safe mode apply to VIctory and its fork. VIctory NVIDIA always uses its own settings; xcl doesn't use them.", "O console e o modo seguro valem para o VIctory e seu fork. O VIctory NVIDIA sempre usa ajustes próprios; o xcl não os usa.", "La console et le mode sûr s'appliquent à VIctory et à son fork. VIctory NVIDIA utilise toujours ses propres réglages ; xcl ne les utilise pas.", "Консоль и безопасный режим действуют для VIctory и его форка. VIctory NVIDIA всегда использует свои настройки; xcl их не использует."]),
    ("Dónde está cada emulador", ["Where each emulator is", "Onde está cada emulador", "Emplacement de chaque émulateur", "Где находится каждый эмулятор"]),
    ("Por defecto el lanzador descarga su propia copia. Si ya tenés uno, elegí su carpeta: al actualizar se unen los cambios nuevos con los tuyos.", ["By default the launcher downloads its own copy. If you already have one, pick its folder: updates merge the new changes with yours.", "Por padrão o launcher baixa sua própria cópia. Se você já tem um, escolha a pasta: ao atualizar, as mudanças novas são unidas às suas.", "Par défaut, le lanceur télécharge sa propre copie. Si vous en avez déjà une, choisissez son dossier : les mises à jour fusionnent les nouveautés avec vos changements.", "По умолчанию лаунчер скачивает свою копию. Если у вас уже есть эмулятор, выберите его папку: при обновлении новые изменения объединяются с вашими."]),
    ("Quitar", ["Remove", "Remover", "Retirer", "Удалить"]),
    ("Usar copia del lanzador", ["Use the launcher's copy", "Usar a cópia do launcher", "Utiliser la copie du lanceur", "Использовать копию лаунчера"]),
    ("Elegir carpeta…", ["Choose folder…", "Escolher pasta…", "Choisir un dossier…", "Выбрать папку…"]),
    ("Elegí la carpeta del juego", ["Choose the game folder", "Escolha a pasta do jogo", "Choisissez le dossier du jeu", "Выберите папку игры"]),
    ("Elegí la carpeta del emulador", ["Choose the emulator folder", "Escolha a pasta do emulador", "Choisissez le dossier de l'émulateur", "Выберите папку эмулятора"]),
    ("Registros y archivos", ["Logs and files", "Registros e arquivos", "Journaux et fichiers", "Журналы и файлы"]),
    ("Registro {}", ["{} log", "Registro {}", "Journal {}", "Журнал {}"]),
    ("{} todavía no se ejecutó: no hay registro.", ["{} hasn't run yet: there is no log.", "{} ainda não foi executado: não há registro.", "{} n'a pas encore été lancé : pas de journal.", "{} ещё не запускался: журнала нет."]),
    ("Errores VIctory", ["VIctory errors", "Erros do VIctory", "Erreurs VIctory", "Ошибки VIctory"]),
    ("VIctory todavía no registró errores.", ["VIctory hasn't logged any errors yet.", "O VIctory ainda não registrou erros.", "VIctory n'a encore enregistré aucune erreur.", "VIctory пока не записал ошибок."]),
    ("Carpeta", ["Folder", "Pasta", "Dossier", "Папка"]),
    ("Configuración", ["Configuration", "Configuração", "Configuration", "Конфигурация"]),
    ("Configuración inicial", ["Initial setup", "Configuração inicial", "Configuration initiale", "Начальная настройка"]),
    ("Imágenes", ["Images", "Imagens", "Images", "Изображения"]),
    ("Un fork de VIctory o de xcl", ["A fork of VIctory or xcl", "Um fork do VIctory ou do xcl", "Un fork de VIctory ou de xcl", "Форк VIctory или xcl"]),
    ("El lanzador lo descarga, lo compila y lo ejecuta igual que el original en el que se basa, así que solo sirve para forks de esos dos proyectos. Antes de agregarlo se comprueba que la dirección exista.", ["The launcher downloads, builds and runs it just like the original it is based on, so it only works with forks of those two projects. The address is checked before it is added.", "O launcher baixa, compila e executa igual ao original em que se baseia, então só serve para forks desses dois projetos. Antes de adicionar, verifica-se que o endereço existe.", "Le lanceur le télécharge, le compile et le lance comme l'original dont il est issu : cela ne marche qu'avec des forks de ces deux projets. L'adresse est vérifiée avant l'ajout.", "Лаунчер скачивает, собирает и запускает его так же, как оригинал, на котором он основан, поэтому подходят только форки этих двух проектов. Перед добавлением адрес проверяется."]),
    ("Nombre", ["Name", "Nome", "Nom", "Название"]),
    ("Por ejemplo: VIctory AMD", ["For example: VIctory AMD", "Por exemplo: VIctory AMD", "Par exemple : VIctory AMD", "Например: VIctory AMD"]),
    ("Dirección de GitHub", ["GitHub address", "Endereço do GitHub", "Adresse GitHub", "Адрес на GitHub"]),
    ("Basado en", ["Based on", "Baseado em", "Basé sur", "Основан на"]),
    ("Rama (opcional)", ["Branch (optional)", "Branch (opcional)", "Branche (facultatif)", "Ветка (необязательно)"]),
    ("Vacía: la principal del repositorio", ["Empty: the repository's main branch", "Vazio: a branch principal do repositório", "Vide : la branche principale du dépôt", "Пусто: основная ветка репозитория"]),
    ("Comprobando…", ["Checking…", "Verificando…", "Vérification…", "Проверка…"]),
    ("Agregar", ["Add", "Adicionar", "Ajouter", "Добавить"]),
    ("Agregados por vos", ["Added by you", "Adicionados por você", "Ajoutés par vous", "Добавленные вами"]),
    ("{} · rama {}", ["{} · branch {}", "{} · branch {}", "{} · branche {}", "{} · ветка {}"]),
    ("Quitar no borra sus archivos: siguen en la carpeta del lanzador (repos).", ["Removing doesn't delete its files: they stay in the launcher's folder (repos).", "Remover não apaga os arquivos: eles continuam na pasta do launcher (repos).", "Retirer ne supprime pas ses fichiers : ils restent dans le dossier du lanceur (repos).", "Удаление не стирает файлы: они остаются в папке лаунчера (repos)."]),
    ("Listo", ["Done", "Pronto", "Prêt", "Готово"]),
    ("Arreglar", ["Fix", "Corrigir", "Corriger", "Исправить"]),
    ("Actualizar", ["Update", "Atualizar", "Mettre à jour", "Обновить"]),
    ("Hay cambios nuevos · versión de {}", ["New changes available · version from {}", "Há mudanças novas · versão de {}", "Nouveaux changements · version du {}", "Есть новые изменения · версия от {}"]),
    ("Encontré otra copia en {}", ["Found another copy in {}", "Encontrei outra cópia em {}", "Autre copie trouvée dans {}", "Найдена другая копия в {}"]),
    ("Usar esta", ["Use this one", "Usar esta", "Utiliser celle-ci", "Использовать эту"]),
    ("Sin elegir", ["Not chosen", "Não escolhida", "Non choisi", "Не выбрана"]),
    ("Carpeta del juego", ["Game folder", "Pasta do jogo", "Dossier du jeu", "Папка игры"]),
    ("Cambiar", ["Change", "Mudar", "Changer", "Изменить"]),
    ("Todo instalado", ["Everything installed", "Tudo instalado", "Tout est installé", "Всё установлено"]),
    ("Faltan cosas por instalar o actualizar", ["Some things still need installing or updating", "Faltam coisas para instalar ou atualizar", "Il reste des éléments à installer ou mettre à jour", "Нужно что-то установить или обновить"]),
    ("SIN TAREAS", ["NO TASKS", "SEM TAREFAS", "AUCUNE TÂCHE", "НЕТ ЗАДАЧ"]),
    ("Elegí un emulador a la izquierda y pulsá Jugar", ["Pick an emulator on the left and press Play", "Escolha um emulador à esquerda e clique em Jogar", "Choisissez un émulateur à gauche et cliquez sur Jouer", "Выберите эмулятор слева и нажмите «Играть»"]),
    ("AVISO", ["NOTICE", "AVISO", "AVIS", "ВНИМАНИЕ"]),
    ("LISTO", ["DONE", "PRONTO", "TERMINÉ", "ГОТОВО"]),
    ("OCULTAR", ["HIDE", "OCULTAR", "MASQUER", "СКРЫТЬ"]),
    ("REGISTRO", ["LOG", "REGISTRO", "JOURNAL", "ЖУРНАЛ"]),
    ("CANCELAR", ["CANCEL", "CANCELAR", "ANNULER", "ОТМЕНА"]),
    ("REVISAR", ["REFRESH", "VERIFICAR", "ACTUALISER", "ПРОВЕРИТЬ"]),
    ("Volver a revisar requisitos y actualizaciones", ["Check requirements and updates again", "Verificar requisitos e atualizações de novo", "Revérifier les prérequis et les mises à jour", "Проверить требования и обновления снова"]),
    ("Volver a la copia del lanzador", ["Go back to the launcher's copy", "Voltar à cópia do launcher", "Revenir à la copie du lanceur", "Вернуться к копии лаунчера"]),
    ("Primera vez", ["First time", "Primeira vez", "Première fois", "Первый запуск"]),
    ("¿Ya lo tenés todo?", ["Already set up?", "Já tem tudo?", "Déjà tout prêt ?", "Уже всё готово?"]),
    ("Saltear este paso", ["Skip this step", "Pular esta etapa", "Passer cette étape", "Пропустить шаг"]),
    ("Juego encontrado en esta carpeta", ["Game found in this folder", "Jogo encontrado nesta pasta", "Jeu trouvé dans ce dossier", "Игра найдена в этой папке"]),
    ("Elegí la carpeta donde está game_scarlett_master.exe", ["Choose the folder that contains game_scarlett_master.exe", "Escolha a pasta onde está game_scarlett_master.exe", "Choisissez le dossier qui contient game_scarlett_master.exe", "Выберите папку, где лежит game_scarlett_master.exe"]),
    ("En esta carpeta no está el juego (falta el .exe que termina en _master.exe)", ["The game is not in this folder (the .exe ending in _master.exe is missing)", "O jogo não está nesta pasta (falta o .exe que termina em _master.exe)", "Le jeu n'est pas dans ce dossier (il manque le .exe finissant par _master.exe)", "В этой папке нет игры (нет .exe, оканчивающегося на _master.exe)"]),
    ("Emuladores", ["Emulators", "Emuladores", "Émulateurs", "Эмуляторы"]),
    ("Si ya tenés alguno descargado, elegí su carpeta. Si no, dejalo así: el lanzador lo descarga y lo compila solo.", ["If you already downloaded one, pick its folder. If not, leave it as is: the launcher downloads and builds it for you.", "Se você já baixou algum, escolha a pasta. Se não, deixe assim: o launcher baixa e compila sozinho.", "Si vous en avez déjà téléchargé un, choisissez son dossier. Sinon, laissez tel quel : le lanceur le télécharge et le compile pour vous.", "Если вы уже скачали эмулятор, выберите его папку. Если нет, оставьте как есть: лаунчер скачает и соберёт его сам."]),
    ("Lo descarga el lanzador", ["The launcher downloads it", "O launcher baixa", "Le lanceur le télécharge", "Скачает лаунчер"]),
    ("Encontré una copia en {}", ["Found a copy in {}", "Encontrei uma cópia em {}", "Copie trouvée dans {}", "Найдена копия в {}"]),
    ("Usar esa", ["Use it", "Usar essa", "L'utiliser", "Использовать"]),
    ("Modo seguro", ["Safe mode", "Modo seguro", "Mode sûr", "Безопасный режим"]),
    ("Pasar solo las pantallas iniciales", ["Skip intro screens automatically", "Passar as telas iniciais sozinho", "Passer les écrans d'intro automatiquement", "Пропускать заставки"]),
    ("¿Dónde está el juego?", ["Where is the game?", "Onde está o jogo?", "Où est le jeu ?", "Где игра?"]),
    ("Es la carpeta con game_scarlett_master.exe. Si lo instalaste con la app de Xbox suele ser C:\\XboxGames\\<juego>\\Content. El lanzador busca en Escritorio, Documentos, Descargas y las carpetas Games, Juegos y XboxGames de cada disco.", ["It is the folder with game_scarlett_master.exe. If you installed it with the Xbox app it is usually C:\\XboxGames\\<game>\\Content. The launcher looks in Desktop, Documents, Downloads and the Games, Juegos and XboxGames folders on every drive.", "É a pasta com game_scarlett_master.exe. Se você instalou com o app do Xbox, costuma ser C:\\XboxGames\\<jogo>\\Content. O launcher procura em Área de Trabalho, Documentos, Downloads e nas pastas Games, Juegos e XboxGames de cada disco.", "C'est le dossier contenant game_scarlett_master.exe. Si vous l'avez installé avec l'application Xbox, c'est souvent C:\\XboxGames\\<jeu>\\Content. Le lanceur cherche dans Bureau, Documents, Téléchargements et les dossiers Games, Juegos et XboxGames de chaque disque.", "Это папка с game_scarlett_master.exe. Если игра установлена через приложение Xbox, обычно это C:\\XboxGames\\<игра>\\Content. Лаунчер ищет на рабочем столе, в документах, загрузках и в папках Games, Juegos и XboxGames на каждом диске."]),
    ("BUSCAR", ["SEARCH", "BUSCAR", "RECHERCHER", "ПОИСК"]),
    ("Volver a buscar el juego y los emuladores", ["Search for the game and emulators again", "Procurar o jogo e os emuladores de novo", "Rechercher à nouveau le jeu et les émulateurs", "Снова найти игру и эмуляторы"]),
    ("CONTINUAR", ["CONTINUE", "CONTINUAR", "CONTINUER", "ДАЛЕЕ"]),
    ("Primero elegí la carpeta del juego", ["First choose the game folder", "Primeiro escolha a pasta do jogo", "Choisissez d'abord le dossier du jeu", "Сначала выберите папку игры"]),
    ("CARPETA DE DATOS", ["DATA FOLDER", "PASTA DE DADOS", "DOSSIER DE DONNÉES", "ПАПКА ДАННЫХ"]),
    ("IMÁGENES", ["IMAGES", "IMAGENS", "IMAGES", "ИЗОБРАЖЕНИЯ"]),
    ("REGISTROS", ["LOGS", "REGISTROS", "JOURNAUX", "ЖУРНАЛЫ"]),
    ("Encontré el juego en {}", ["Found the game in {}", "Encontrei o jogo em {}", "Jeu trouvé dans {}", "Игра найдена в {}"]),
    ("Cerrá el juego antes de cambiar la carpeta del emulador.", ["Close the game before changing the emulator folder.", "Feche o jogo antes de mudar a pasta do emulador.", "Fermez le jeu avant de changer le dossier de l'émulateur.", "Закройте игру, прежде чем менять папку эмулятора."]),
    ("{} ahora usa {}", ["{} now uses {}", "{} agora usa {}", "{} utilise maintenant {}", "{} теперь использует {}"]),
    ("Agregando {}", ["Adding {}", "Adicionando {}", "Ajout de {}", "Добавление {}"]),
    ("Poné un nombre para el emulador.", ["Enter a name for the emulator.", "Digite um nome para o emulador.", "Saisissez un nom pour l'émulateur.", "Введите название эмулятора."]),
    ("La dirección tiene que ser como https://github.com/dueño/repositorio", ["The address must look like https://github.com/owner/repository", "O endereço deve ser como https://github.com/dono/repositorio", "L'adresse doit ressembler à https://github.com/propriétaire/dépôt", "Адрес должен быть вида https://github.com/владелец/репозиторий"]),
    ("Ese repositorio ya está en el lanzador, como {}.", ["That repository is already in the launcher, as {}.", "Esse repositório já está no launcher, como {}.", "Ce dépôt est déjà dans le lanceur, sous le nom {}.", "Этот репозиторий уже есть в лаунчере: {}."]),
    ("Comprobando {} …", ["Checking {} …", "Verificando {} …", "Vérification de {} …", "Проверка {} …"]),
    ("{} se agregó (rama {}). Ahora podés instalarlo.", ["{} was added (branch {}). You can install it now.", "{} foi adicionado (branch {}). Agora você pode instalá-lo.", "{} a été ajouté (branche {}). Vous pouvez maintenant l'installer.", "{} добавлен (ветка {}). Теперь его можно установить."]),
    ("Cerrá el juego antes de quitar el emulador.", ["Close the game before removing the emulator.", "Feche o jogo antes de remover o emulador.", "Fermez le jeu avant de retirer l'émulateur.", "Закройте игру, прежде чем удалять эмулятор."]),
    ("Se quitó {} del lanzador. Sus archivos siguen en {}", ["{} was removed from the launcher. Its files are still in {}", "{} foi removido do launcher. Os arquivos continuam em {}", "{} a été retiré du lanceur. Ses fichiers restent dans {}", "{} удалён из лаунчера. Его файлы остались в {}"]),
    ("Ya hay una tarea en curso.", ["A task is already running.", "Já há uma tarefa em andamento.", "Une tâche est déjà en cours.", "Уже выполняется задача."]),
    ("Cerrá el juego antes de actualizar {}: no se puede reemplazar mientras está abierto.", ["Close the game before updating {}: it can't be replaced while it's open.", "Feche o jogo antes de atualizar {}: não dá para substituir com ele aberto.", "Fermez le jeu avant de mettre à jour {} : impossible de le remplacer pendant qu'il est ouvert.", "Закройте игру перед обновлением {}: его нельзя заменить, пока он открыт."]),
    ("Instalando {}", ["Installing {}", "Instalando {}", "Installation de {}", "Установка {}"]),
    ("repositorio desconocido", ["unknown repository", "repositório desconhecido", "dépôt inconnu", "неизвестный репозиторий"]),
    ("{} quedó instalado y compilado.", ["{} is installed and built.", "{} foi instalado e compilado.", "{} est installé et compilé.", "{} установлен и собран."]),
    ("{}: hay {}.", ["{}: {} available.", "{}: {} disponíveis.", "{} : {} disponibles.", "{}: {}."]),
    ("{}: ya tenés lo último.", ["{}: you have the latest version.", "{}: você já tem a última versão.", "{} : vous avez la dernière version.", "{}: у вас последняя версия."]),
    ("{}: no se pudo buscar ({}). ¿Hay internet?", ["{}: couldn't check ({}). Are you online?", "{}: não foi possível verificar ({}). Tem internet?", "{} : vérification impossible ({}). Êtes-vous connecté ?", "{}: не удалось проверить ({}). Есть интернет?"]),
    ("Ya hay un juego en ejecución.", ["A game is already running.", "Já há um jogo em execução.", "Un jeu est déjà en cours.", "Игра уже запущена."]),
    ("{} iniciado. Su registro: {}", ["{} started. Its log: {}", "{} iniciado. Registro: {}", "{} lancé. Son journal : {}", "{} запущен. Журнал: {}"]),
    ("Terminó bien ({} s).", ["Finished fine ({} s).", "Terminou bem ({} s).", "Terminé sans erreur ({} s).", "Завершено успешно ({} с)."]),
    ("Terminó con código {} después de {} s. Mirá el registro.", ["Ended with code {} after {} s. Check the log.", "Terminou com código {} depois de {} s. Veja o registro.", "Terminé avec le code {} après {} s. Consultez le journal.", "Завершено с кодом {} через {} с. Посмотрите журнал."]),
    ("No se pudo ejecutar: {}", ["Couldn't run it: {}", "Não foi possível executar: {}", "Exécution impossible : {}", "Не удалось запустить: {}"]),
    ("terminó con error (código {})", ["ended with an error (code {})", "terminou com erro (código {})", "s'est terminé avec une erreur (code {})", "завершилась с ошибкой (код {})"]),
    ("No se pudo ejecutar git: {}", ["Couldn't run git: {}", "Não foi possível executar o git: {}", "Impossible d'exécuter git : {}", "Не удалось запустить git: {}"]),
    ("No encontré ese repositorio. Revisá la dirección (tiene que ser público).", ["Couldn't find that repository. Check the address (it must be public).", "Não encontrei esse repositório. Confira o endereço (precisa ser público).", "Dépôt introuvable. Vérifiez l'adresse (il doit être public).", "Репозиторий не найден. Проверьте адрес (он должен быть публичным)."]),
    ("El repositorio está vacío o no tiene rama principal.", ["The repository is empty or has no main branch.", "O repositório está vazio ou não tem branch principal.", "Le dépôt est vide ou n'a pas de branche principale.", "Репозиторий пуст или у него нет основной ветки."]),
    ("El repositorio no tiene la rama {}.", ["The repository has no branch {}.", "O repositório não tem a branch {}.", "Le dépôt n'a pas de branche {}.", "В репозитории нет ветки {}."]),
    ("Compilando… la primera vez tarda varios minutos.", ["Building… the first time takes several minutes.", "Compilando… a primeira vez leva vários minutos.", "Compilation… la première fois prend plusieurs minutes.", "Сборка… в первый раз это занимает несколько минут."]),
    ("La compilación {}", ["The build {}", "A compilação {}", "La compilation {}", "Сборка {}"]),
    ("La compilación terminó pero no dejó {}. ¿Es un fork de {}? Si el ejecutable quedó en otro lado, cambiá \"exe\" en la configuración.", ["The build finished but didn't produce {}. Is it a fork of {}? If the executable ended up elsewhere, change \"exe\" in the configuration.", "A compilação terminou mas não gerou {}. É um fork do {}? Se o executável ficou em outro lugar, mude \"exe\" na configuração.", "La compilation s'est terminée sans produire {}. Est-ce un fork de {} ? Si l'exécutable est ailleurs, modifiez \"exe\" dans la configuration.", "Сборка завершилась, но не создала {}. Это форк {}? Если исполняемый файл оказался в другом месте, измените \"exe\" в конфигурации."]),
    ("La compilación terminó pero no dejó el ejecutable.", ["The build finished but didn't produce the executable.", "A compilação terminou mas não gerou o executável.", "La compilation s'est terminée sans produire l'exécutable.", "Сборка завершилась, но не создала исполняемый файл."]),
    ("Descargando {} desde {} …", ["Downloading {} from {} …", "Baixando {} de {} …", "Téléchargement de {} depuis {} …", "Загрузка {} из {} …"]),
    ("carpeta inválida", ["invalid folder", "pasta inválida", "dossier invalide", "неверная папка"]),
    ("No se pudo crear la carpeta: {}", ["Couldn't create the folder: {}", "Não foi possível criar a pasta: {}", "Impossible de créer le dossier : {}", "Не удалось создать папку: {}"]),
    ("La descarga {}", ["The download {}", "O download {}", "Le téléchargement {}", "Загрузка {}"]),
    ("Buscando cambios nuevos…", ["Looking for new changes…", "Procurando mudanças novas…", "Recherche de nouveaux changements…", "Поиск новых изменений…"]),
    ("Pasando a la última versión de {} …", ["Switching to the latest version of {} …", "Passando para a última versão de {} …", "Passage à la dernière version de {} …", "Переход на последнюю версию {} …"]),
    ("No se pudo pasar a la versión nueva: {}", ["Couldn't switch to the new version: {}", "Não foi possível passar para a versão nova: {}", "Impossible de passer à la nouvelle version : {}", "Не удалось перейти на новую версию: {}"]),
    ("Uniendo los cambios de {} …", ["Merging the changes from {} …", "Unindo as mudanças de {} …", "Fusion des changements de {} …", "Объединение изменений из {} …"]),
    ("Los cambios nuevos chocan con los tuyos en {} archivo(s): {}. No se tocó nada. Esta carpeta ({}) tiene cambios propios; para que el lanzador la actualice solo, usá una copia en su carpeta (repos).", ["The new changes conflict with yours in {} file(s): {}. Nothing was touched. This folder ({}) has its own changes; to let the launcher update it by itself, use a copy in its folder (repos).", "As mudanças novas conflitam com as suas em {} arquivo(s): {}. Nada foi alterado. Esta pasta ({}) tem mudanças próprias; para o launcher atualizar sozinho, use uma cópia na pasta dele (repos).", "Les nouveaux changements entrent en conflit avec les vôtres dans {} fichier(s) : {}. Rien n'a été modifié. Ce dossier ({}) a ses propres changements ; pour que le lanceur le mette à jour seul, utilisez une copie dans son dossier (repos).", "Новые изменения конфликтуют с вашими в файлах ({}): {}. Ничего не изменено. В этой папке ({}) есть свои изменения; чтобы лаунчер обновлял её сам, используйте копию в его папке (repos)."]),
    ("{} todavía no está compilado: usá Instalar.", ["{} isn't built yet: use Install.", "{} ainda não foi compilado: use Instalar.", "{} n'est pas encore compilé : utilisez Installer.", "{} ещё не собран: используйте «Установить»."]),
    ("No encuentro el juego en la carpeta elegida (falta game_scarlett_master.exe).", ["Can't find the game in the chosen folder (game_scarlett_master.exe is missing).", "Não encontro o jogo na pasta escolhida (falta game_scarlett_master.exe).", "Jeu introuvable dans le dossier choisi (game_scarlett_master.exe manquant).", "Игра не найдена в выбранной папке (нет game_scarlett_master.exe)."]),
    ("No encuentro el juego en la carpeta elegida.", ["Can't find the game in the chosen folder.", "Não encontro o jogo na pasta escolhida.", "Jeu introuvable dans le dossier choisi.", "Игра не найдена в выбранной папке."]),
    ("No se pudo crear el registro: {}", ["Couldn't create the log: {}", "Não foi possível criar o registro: {}", "Impossible de créer le journal : {}", "Не удалось создать журнал: {}"]),
    ("No se pudo iniciar {}: {}", ["Couldn't start {}: {}", "Não foi possível iniciar {}: {}", "Impossible de lancer {} : {}", "Не удалось запустить {}: {}"]),
    ("Para bajar el código de los proyectos", ["To download the projects' code", "Para baixar o código dos projetos", "Pour télécharger le code des projets", "Для загрузки кода проектов"]),
    ("Para bajar el código de los proyectos · {}", ["To download the projects' code · {}", "Para baixar o código dos projetos · {}", "Pour télécharger le code des projets · {}", "Для загрузки кода проектов · {}"]),
    ("El lenguaje con el que están hechos", ["The language they are written in", "A linguagem em que são feitos", "Le langage dans lequel ils sont écrits", "Язык, на котором они написаны"]),
    ("El lenguaje con el que están hechos · {}", ["The language they are written in · {}", "A linguagem em que são feitos · {}", "Le langage dans lequel ils sont écrits · {}", "Язык, на котором они написаны · {}"]),
    (" · falta el Windows SDK", [" · the Windows SDK is missing", " · falta o Windows SDK", " · le Windows SDK manque", " · нет Windows SDK"]),
    (" · falta el compilador de C++", [" · the C++ compiler is missing", " · falta o compilador C++", " · le compilateur C++ manque", " · нет компилятора C++"]),
    ("Compilador de C++ y Windows SDK", ["C++ compiler and Windows SDK", "Compilador C++ e Windows SDK", "Compilateur C++ et Windows SDK", "Компилятор C++ и Windows SDK"]),
    ("Visual Studio Build Tools · tarda unos minutos{}", ["Visual Studio Build Tools · takes a few minutes{}", "Visual Studio Build Tools · leva alguns minutos{}", "Visual Studio Build Tools · prend quelques minutes{}", "Visual Studio Build Tools · займёт несколько минут{}"]),
    ("Lo piden xcl y los juegos de Xbox · viene de la Microsoft Store", ["Required by xcl and Xbox games · comes from the Microsoft Store", "Exigido pelo xcl e pelos jogos de Xbox · vem da Microsoft Store", "Requis par xcl et les jeux Xbox · vient du Microsoft Store", "Нужен xcl и играм Xbox · из Microsoft Store"]),
    ("Memoria virtual de Windows", ["Windows virtual memory", "Memória virtual do Windows", "Mémoire virtuelle de Windows", "Виртуальная память Windows"]),
    ("Tenés {} GB de RAM y un límite de {} GB · se recomienda 32 GB o más", ["You have {} GB of RAM and a limit of {} GB · 32 GB or more is recommended", "Você tem {} GB de RAM e um limite de {} GB · recomenda-se 32 GB ou mais", "Vous avez {} Go de RAM et une limite de {} Go · 32 Go ou plus recommandés", "У вас {} ГБ ОЗУ и лимит {} ГБ · рекомендуется 32 ГБ или больше"]),
    ("Se va a abrir una ventana de Windows pidiendo permiso para instalar.", ["A Windows window will open asking for permission to install.", "Vai abrir uma janela do Windows pedindo permissão para instalar.", "Une fenêtre Windows va s'ouvrir pour demander l'autorisation d'installer.", "Откроется окно Windows с запросом разрешения на установку."]),
    ("No se pudo ejecutar winget: {}", ["Couldn't run winget: {}", "Não foi possível executar o winget: {}", "Impossible d'exécuter winget : {}", "Не удалось запустить winget: {}"]),
    ("Instalado. Cerrá y volvé a abrir el lanzador para que lo detecte.", ["Installed. Close and reopen the launcher so it detects it.", "Instalado. Feche e abra o launcher de novo para ele detectar.", "Installé. Fermez et rouvrez le lanceur pour qu'il le détecte.", "Установлено. Закройте и снова откройте лаунчер, чтобы он это увидел."]),
    ("winget terminó con error (código {}). Si Windows Update o la tienda están bloqueados, instalalo a mano.", ["winget ended with an error (code {}). If Windows Update or the Store are blocked, install it manually.", "O winget terminou com erro (código {}). Se o Windows Update ou a loja estiverem bloqueados, instale manualmente.", "winget s'est terminé avec une erreur (code {}). Si Windows Update ou le Store sont bloqués, installez-le à la main.", "winget завершился с ошибкой (код {}). Если Windows Update или магазин заблокированы, установите вручную."]),
    ("Se abrió la Microsoft Store en Gaming Services: instalalo desde ahí.", ["The Microsoft Store opened on Gaming Services: install it from there.", "A Microsoft Store abriu em Gaming Services: instale por lá.", "Le Microsoft Store s'est ouvert sur Gaming Services : installez-le depuis là.", "Microsoft Store открыт на странице Gaming Services: установите оттуда."]),
    ("Rendimiento → Configuración → Opciones avanzadas → Memoria virtual → Cambiar. Poné un tamaño fijo de 32768 MB o más y reiniciá.", ["Performance → Settings → Advanced → Virtual memory → Change. Set a fixed size of 32768 MB or more and restart.", "Desempenho → Configurações → Avançado → Memória virtual → Alterar. Defina um tamanho fixo de 32768 MB ou mais e reinicie.", "Performances → Paramètres → Avancé → Mémoire virtuelle → Modifier. Définissez une taille fixe de 32768 Mo ou plus et redémarrez.", "Быстродействие → Параметры → Дополнительно → Виртуальная память → Изменить. Задайте фиксированный размер 32768 МБ или больше и перезагрузитесь."]),
    ("Requisito desconocido.", ["Unknown requirement.", "Requisito desconhecido.", "Prérequis inconnu.", "Неизвестный компонент."]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completa_los_datos() {
        assert_eq!(fill("{} de {}", &[&1, &"dos"]), "1 de dos");
        assert_eq!(fill("sin datos", &[&1]), "sin datos");
    }

    /// Cada traducción tiene que tener tantos `{}` como el original, o se pierden datos.
    #[test]
    fn mismos_huecos() {
        for (es, tr) in ENTRIES {
            for t in tr {
                assert_eq!(es.matches("{}").count(), t.matches("{}").count(), "«{es}» → «{t}»");
                assert!(!t.is_empty(), "falta la traducción de «{es}»");
            }
        }
    }

    #[test]
    fn sin_repetidos() {
        assert_eq!(TABLE.len(), ENTRIES.len());
    }
}
